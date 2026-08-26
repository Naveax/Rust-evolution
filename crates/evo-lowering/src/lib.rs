use evo_lexer::Span;
pub use evo_parser::BinaryOp;
use evo_parser::{
    Expr as SyntaxExpr, ExprKind as SyntaxExprKind, Program as SyntaxProgram, Stmt as SyntaxStmt,
    StmtKind as SyntaxStmtKind,
};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StmtKind {
    Let {
        name: String,
        mutable: bool,
        expr: Expr,
    },
    Assign {
        name: String,
        expr: Expr,
    },
    Print(Expr),
    Repeat {
        count: Expr,
        body: Vec<Stmt>,
    },
    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprKind {
    Integer(i64),
    String(String),
    Bool(bool),
    Local(String),
    InputInt,
    UnaryMinus(Box<Expr>),
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowerError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for LowerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}:{}",
            self.message, self.span.line, self.span.column
        )
    }
}

impl Error for LowerError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueType {
    Integer,
    String,
    Bool,
}

#[derive(Debug, Clone, Copy)]
struct BindingState {
    value_type: ValueType,
}

pub fn lower(program: &SyntaxProgram) -> Result<Program, LowerError> {
    Analyzer::new().lower_program(program)
}

struct Analyzer {
    bindings: HashMap<String, BindingState>,
    mutable_bindings: HashSet<String>,
    control_depth: usize,
}

impl Analyzer {
    fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            mutable_bindings: HashSet::new(),
            control_depth: 0,
        }
    }

    fn lower_program(mut self, program: &SyntaxProgram) -> Result<Program, LowerError> {
        let mut statements = self.lower_statements(&program.statements)?;
        self.apply_mutability(&mut statements);
        Ok(Program { statements })
    }

    fn lower_statements(&mut self, statements: &[SyntaxStmt]) -> Result<Vec<Stmt>, LowerError> {
        statements
            .iter()
            .map(|statement| self.lower_statement(statement))
            .collect()
    }

    fn lower_statement(&mut self, statement: &SyntaxStmt) -> Result<Stmt, LowerError> {
        let kind = match &statement.kind {
            SyntaxStmtKind::Bind { name, expr } => {
                let (expr, value_type) = self.lower_expr(expr)?;
                if let Some(binding) = self.bindings.get(name).copied() {
                    if binding.value_type != value_type {
                        return Err(LowerError {
                            message: format!(
                                "cannot assign a different value type to existing local {name:?}"
                            ),
                            span: statement.span,
                        });
                    }
                    self.mutable_bindings.insert(name.clone());
                    StmtKind::Assign {
                        name: name.clone(),
                        expr,
                    }
                } else {
                    if self.control_depth > 0 {
                        return Err(LowerError {
                            message: format!(
                                "local {name:?} must be defined before entering a control-flow block"
                            ),
                            span: statement.span,
                        });
                    }
                    self.bindings
                        .insert(name.clone(), BindingState { value_type });
                    StmtKind::Let {
                        name: name.clone(),
                        mutable: false,
                        expr,
                    }
                }
            }
            SyntaxStmtKind::Print(expr) => {
                let (expr, _) = self.lower_expr(expr)?;
                StmtKind::Print(expr)
            }
            SyntaxStmtKind::Repeat { count, body } => {
                let (count, count_type) = self.lower_expr(count)?;
                if count_type != ValueType::Integer {
                    return Err(LowerError {
                        message: "repeat count must be an integer".to_owned(),
                        span: count.span,
                    });
                }

                self.control_depth += 1;
                let lowered_body = self.lower_statements(body);
                self.control_depth -= 1;
                let body = lowered_body?;
                StmtKind::Repeat { count, body }
            }
            SyntaxStmtKind::If {
                condition,
                then_body,
                else_body,
            } => {
                let (condition, condition_type) = self.lower_expr(condition)?;
                if condition_type != ValueType::Bool {
                    return Err(LowerError {
                        message: "if condition must be a boolean".to_owned(),
                        span: condition.span,
                    });
                }

                self.control_depth += 1;
                let then_result = self.lower_statements(then_body);
                let else_result = if then_result.is_ok() {
                    self.lower_statements(else_body)
                } else {
                    Ok(Vec::new())
                };
                self.control_depth -= 1;
                let then_body = then_result?;
                let else_body = else_result?;
                StmtKind::If {
                    condition,
                    then_body,
                    else_body,
                }
            }
        };

        Ok(Stmt {
            kind,
            span: statement.span,
        })
    }

    fn lower_expr(&self, expr: &SyntaxExpr) -> Result<(Expr, ValueType), LowerError> {
        let (kind, value_type) = match &expr.kind {
            SyntaxExprKind::Integer(value) => (ExprKind::Integer(*value), ValueType::Integer),
            SyntaxExprKind::String(value) => (ExprKind::String(value.clone()), ValueType::String),
            SyntaxExprKind::Bool(value) => (ExprKind::Bool(*value), ValueType::Bool),
            SyntaxExprKind::Identifier(name) => {
                let binding = self.bindings.get(name).ok_or_else(|| LowerError {
                    message: format!("use of local {name:?} before definition"),
                    span: expr.span,
                })?;
                (ExprKind::Local(name.clone()), binding.value_type)
            }
            SyntaxExprKind::InputInt => (ExprKind::InputInt, ValueType::Integer),
            SyntaxExprKind::UnaryMinus(inner) => {
                let (inner, inner_type) = self.lower_expr(inner)?;
                if inner_type != ValueType::Integer {
                    return Err(LowerError {
                        message: "unary '-' requires an integer operand".to_owned(),
                        span: expr.span,
                    });
                }
                (ExprKind::UnaryMinus(Box::new(inner)), ValueType::Integer)
            }
            SyntaxExprKind::Binary { left, op, right } => {
                let (left, left_type) = self.lower_expr(left)?;
                let (right, right_type) = self.lower_expr(right)?;
                let result_type = match op {
                    BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide => {
                        if left_type != ValueType::Integer || right_type != ValueType::Integer {
                            return Err(LowerError {
                                message: "arithmetic operators require integer operands".to_owned(),
                                span: expr.span,
                            });
                        }
                        ValueType::Integer
                    }
                    BinaryOp::Equal | BinaryOp::NotEqual => {
                        if left_type != right_type {
                            return Err(LowerError {
                                message: "equality operands must have the same value type".to_owned(),
                                span: expr.span,
                            });
                        }
                        ValueType::Bool
                    }
                    BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => {
                        if left_type != ValueType::Integer || right_type != ValueType::Integer {
                            return Err(LowerError {
                                message: "ordering operators require integer operands".to_owned(),
                                span: expr.span,
                            });
                        }
                        ValueType::Bool
                    }
                };
                (
                    ExprKind::Binary {
                        left: Box::new(left),
                        op: *op,
                        right: Box::new(right),
                    },
                    result_type,
                )
            }
        };

        Ok((
            Expr {
                kind,
                span: expr.span,
            },
            value_type,
        ))
    }

    fn apply_mutability(&self, statements: &mut [Stmt]) {
        for statement in statements {
            match &mut statement.kind {
                StmtKind::Let { name, mutable, .. } => {
                    *mutable = self.mutable_bindings.contains(name);
                }
                StmtKind::Repeat { body, .. } => self.apply_mutability(body),
                StmtKind::If {
                    then_body,
                    else_body,
                    ..
                } => {
                    self.apply_mutability(then_body);
                    self.apply_mutability(else_body);
                }
                StmtKind::Assign { .. } | StmtKind::Print(_) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ExprKind, StmtKind, lower};
    use evo_lexer::lex;
    use evo_parser::parse;

    fn lower_source(source: &str) -> Result<super::Program, super::LowerError> {
        let tokens = lex(source).expect("lexing should succeed");
        let syntax = parse(&tokens).expect("parsing should succeed");
        lower(&syntax)
    }

    #[test]
    fn infers_mutability_for_reassignment_inside_repeat() {
        let program =
            lower_source("n = input_int\nsum = 0\nrepeat n\nsum = sum + 1\nend\nprint sum\n")
                .expect("lowering should succeed");

        assert!(matches!(
            &program.statements[0].kind,
            StmtKind::Let {
                name,
                mutable: false,
                expr: super::Expr {
                    kind: ExprKind::InputInt,
                    ..
                }
            } if name == "n"
        ));
        assert!(matches!(
            &program.statements[1].kind,
            StmtKind::Let {
                name,
                mutable: true,
                ..
            } if name == "sum"
        ));
    }

    #[test]
    fn lowers_boolean_condition_and_comparisons() {
        let program = lower_source("x = 1\nif x >= 1\nprint true\nelse\nprint false\nend\n")
            .expect("boolean control flow should lower");
        let StmtKind::If {
            condition,
            then_body,
            else_body,
        } = &program.statements[1].kind
        else {
            panic!("expected if statement");
        };
        assert!(matches!(condition.kind, ExprKind::Binary { .. }));
        assert!(matches!(
            then_body[0].kind,
            StmtKind::Print(super::Expr {
                kind: ExprKind::Bool(true),
                ..
            })
        ));
        assert!(matches!(
            else_body[0].kind,
            StmtKind::Print(super::Expr {
                kind: ExprKind::Bool(false),
                ..
            })
        ));
    }

    #[test]
    fn reassignments_in_conditional_branches_infer_mutability() {
        let program = lower_source(
            "x = 0\nflag = true\nif flag\nx = x + 1\nelse\nx = x - 1\nend\n",
        )
        .expect("branch reassignment should lower");
        assert!(matches!(
            &program.statements[0].kind,
            StmtKind::Let {
                name,
                mutable: true,
                ..
            } if name == "x"
        ));
    }

    #[test]
    fn equality_supports_same_type_integer_bool_and_string_values() {
        for source in [
            "print 1 == 1\n",
            "print true != false\n",
            "print \"a\" == \"a\"\n",
        ] {
            lower_source(source).expect("same-type equality should lower");
        }
    }

    #[test]
    fn rejects_mixed_type_equality() {
        let error = lower_source("print 1 == true\n").expect_err("mixed equality should fail");
        assert!(error.message.contains("same value type"));
    }

    #[test]
    fn rejects_non_integer_ordering() {
        let error = lower_source("print true < false\n").expect_err("boolean ordering should fail");
        assert!(error.message.contains("ordering operators require integer"));
    }

    #[test]
    fn rejects_non_boolean_if_condition() {
        let error = lower_source("if 1\nprint 1\nend\n").expect_err("integer condition should fail");
        assert!(error.message.contains("if condition must be a boolean"));
    }

    #[test]
    fn rejects_new_binding_inside_conditional_branch() {
        let error = lower_source("if true\nx = 1\nend\n")
            .expect_err("branch-local definition should fail in v0");
        assert!(error.message.contains("defined before entering a control-flow block"));
    }

    #[test]
    fn same_type_reassignment_marks_first_binding_mutable() {
        let program = lower_source("x = 1\nx = 2\n").expect("lowering should succeed");
        assert!(matches!(
            &program.statements[0].kind,
            StmtKind::Let {
                name,
                mutable: true,
                ..
            } if name == "x"
        ));
        assert!(matches!(
            &program.statements[1].kind,
            StmtKind::Assign { name, .. } if name == "x"
        ));
    }

    #[test]
    fn infers_multiple_mutable_locals() {
        let program = lower_source("a = 0\nb = 0\nrepeat 2\na = a + 1\nb = b + 2\nend\n")
            .expect("lowering should succeed");

        assert!(matches!(
            &program.statements[0].kind,
            StmtKind::Let {
                name,
                mutable: true,
                ..
            } if name == "a"
        ));
        assert!(matches!(
            &program.statements[1].kind,
            StmtKind::Let {
                name,
                mutable: true,
                ..
            } if name == "b"
        ));
    }

    #[test]
    fn rejects_use_before_definition() {
        let error = lower_source("x = x + 1\n").expect_err("undefined read should fail");
        assert!(error.message.contains("before definition"));
    }

    #[test]
    fn rejects_new_binding_inside_repeat() {
        let error = lower_source("repeat 1\nx = 1\nend\n")
            .expect_err("loop-local definition should fail in v0");
        assert!(error.message.contains("defined before entering"));
    }

    #[test]
    fn rejects_non_integer_repeat_count() {
        let error = lower_source("name = \"x\"\nrepeat name\nend\n")
            .expect_err("string repeat count should fail");
        assert!(error.message.contains("repeat count must be an integer"));
    }

    #[test]
    fn rejects_type_changing_reassignment() {
        let error = lower_source("x = 1\nx = \"later\"\n")
            .expect_err("type-changing assignment should fail");
        assert!(error.message.contains("different value type"));
    }

    #[test]
    fn accepts_negative_repeat_count_as_zero_iteration_range() {
        lower_source("repeat -1\nend\n").expect("negative repeat count is valid");
    }
}
