use evo_lexer::Span;
pub use evo_parser::BinaryOp;
use evo_parser::{
    Expr as SyntaxExpr, ExprKind as SyntaxExprKind, FunctionDef as SyntaxFunction,
    Program as SyntaxProgram, Stmt as SyntaxStmt, StmtKind as SyntaxStmtKind,
    TypeName as SyntaxTypeName,
};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub functions: Vec<Function>,
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: ValueType,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub name: String,
    pub value_type: ValueType,
    pub mutable: bool,
    pub span: Span,
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
    Return(Expr),
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
    Call {
        name: String,
        arguments: Vec<Expr>,
    },
    InputInt,
    LogicalNot(Box<Expr>),
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
pub enum ValueType {
    Integer,
    String,
    Bool,
}

#[derive(Debug, Clone, Copy)]
struct BindingState {
    value_type: ValueType,
    declaration_start: usize,
}

#[derive(Debug, Clone)]
struct FunctionSignature {
    parameter_types: Vec<ValueType>,
    return_type: ValueType,
}

pub fn lower(program: &SyntaxProgram) -> Result<Program, LowerError> {
    if let Some(record) = program.records.first() {
        return Err(LowerError {
            message: "record declarations are parsed but Records v0 semantic lowering is not implemented yet"
                .to_owned(),
            span: record.span,
        });
    }

    let signatures = collect_function_signatures(&program.functions)?;
    let mut functions = Vec::with_capacity(program.functions.len());
    for function in &program.functions {
        functions.push(lower_function(function, &signatures)?);
    }

    let mut top_level = Analyzer::new(&signatures, None);
    let mut statements = top_level.lower_statements(&program.statements)?;
    top_level.apply_mutability(&mut statements);

    Ok(Program {
        functions,
        statements,
    })
}

fn collect_function_signatures(
    functions: &[SyntaxFunction],
) -> Result<HashMap<String, FunctionSignature>, LowerError> {
    let mut signatures = HashMap::new();
    for function in functions {
        if signatures.contains_key(&function.name) {
            return Err(LowerError {
                message: format!("duplicate function name {:?}", function.name),
                span: function.span,
            });
        }

        let mut seen_parameters = HashSet::new();
        let mut parameter_types = Vec::with_capacity(function.parameters.len());
        for parameter in &function.parameters {
            if !seen_parameters.insert(parameter.name.clone()) {
                return Err(LowerError {
                    message: format!("duplicate parameter name {:?}", parameter.name),
                    span: parameter.span,
                });
            }
            parameter_types.push(value_type(parameter.type_name));
        }

        signatures.insert(
            function.name.clone(),
            FunctionSignature {
                parameter_types,
                return_type: value_type(function.return_type),
            },
        );
    }
    Ok(signatures)
}

fn lower_function(
    function: &SyntaxFunction,
    signatures: &HashMap<String, FunctionSignature>,
) -> Result<Function, LowerError> {
    let return_type = value_type(function.return_type);
    let mut analyzer = Analyzer::new(signatures, Some(return_type));
    let mut parameters = Vec::with_capacity(function.parameters.len());

    for parameter in &function.parameters {
        let parameter_type = value_type(parameter.type_name);
        analyzer.define_binding(parameter.name.clone(), parameter_type, parameter.span.start);
        parameters.push(Parameter {
            name: parameter.name.clone(),
            value_type: parameter_type,
            mutable: false,
            span: parameter.span,
        });
    }

    let mut body = analyzer.lower_statements(&function.body)?;
    analyzer.apply_mutability(&mut body);
    for parameter in &mut parameters {
        parameter.mutable = analyzer
            .mutable_declarations
            .contains(&parameter.span.start);
    }

    if !block_always_returns(&body) {
        return Err(LowerError {
            message: format!(
                "function {:?} must return {} on every terminal path",
                function.name,
                type_label(return_type)
            ),
            span: function.span,
        });
    }

    Ok(Function {
        name: function.name.clone(),
        parameters,
        return_type,
        body,
        span: function.span,
    })
}

fn block_always_returns(statements: &[Stmt]) -> bool {
    statements.iter().any(statement_always_returns)
}

fn statement_always_returns(statement: &Stmt) -> bool {
    match &statement.kind {
        StmtKind::Return(_) => true,
        StmtKind::If {
            then_body,
            else_body,
            ..
        } => {
            !else_body.is_empty()
                && block_always_returns(then_body)
                && block_always_returns(else_body)
        }
        StmtKind::Let { .. }
        | StmtKind::Assign { .. }
        | StmtKind::Print(_)
        | StmtKind::Repeat { .. } => false,
    }
}

fn value_type(type_name: SyntaxTypeName) -> ValueType {
    match type_name {
        SyntaxTypeName::Int => ValueType::Integer,
        SyntaxTypeName::Bool => ValueType::Bool,
        SyntaxTypeName::String => ValueType::String,
    }
}

fn type_label(value_type: ValueType) -> &'static str {
    match value_type {
        ValueType::Integer => "int",
        ValueType::Bool => "bool",
        ValueType::String => "string",
    }
}

struct Analyzer<'a> {
    scopes: Vec<HashMap<String, BindingState>>,
    mutable_declarations: HashSet<usize>,
    function_signatures: &'a HashMap<String, FunctionSignature>,
    expected_return: Option<ValueType>,
}

impl<'a> Analyzer<'a> {
    fn new(
        function_signatures: &'a HashMap<String, FunctionSignature>,
        expected_return: Option<ValueType>,
    ) -> Self {
        Self {
            scopes: vec![HashMap::new()],
            mutable_declarations: HashSet::new(),
            function_signatures,
            expected_return,
        }
    }

    fn visible_binding(&self, name: &str) -> Option<BindingState> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    fn define_binding(&mut self, name: String, value_type: ValueType, declaration_start: usize) {
        self.scopes
            .last_mut()
            .expect("analyzer always has a lexical scope")
            .insert(
                name,
                BindingState {
                    value_type,
                    declaration_start,
                },
            );
    }

    fn lower_child_scope(&mut self, statements: &[SyntaxStmt]) -> Result<Vec<Stmt>, LowerError> {
        self.scopes.push(HashMap::new());
        let result = self.lower_statements(statements);
        let popped = self.scopes.pop();
        debug_assert!(popped.is_some());
        result
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
                let (expr, expression_type) = self.lower_expr(expr)?;
                if let Some(binding) = self.visible_binding(name) {
                    if binding.value_type != expression_type {
                        return Err(LowerError {
                            message: format!(
                                "cannot assign a different value type to existing local {name:?}"
                            ),
                            span: statement.span,
                        });
                    }
                    self.mutable_declarations.insert(binding.declaration_start);
                    StmtKind::Assign {
                        name: name.clone(),
                        expr,
                    }
                } else {
                    self.define_binding(name.clone(), expression_type, statement.span.start);
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
            SyntaxStmtKind::Return(expr) => {
                let expected_return = self.expected_return.ok_or_else(|| LowerError {
                    message: "return is only valid inside a function".to_owned(),
                    span: statement.span,
                })?;
                let (expr, actual_type) = self.lower_expr(expr)?;
                if actual_type != expected_return {
                    return Err(LowerError {
                        message: format!(
                            "return type mismatch: expected {}, found {}",
                            type_label(expected_return),
                            type_label(actual_type)
                        ),
                        span: statement.span,
                    });
                }
                StmtKind::Return(expr)
            }
            SyntaxStmtKind::Repeat { count, body } => {
                let (count, count_type) = self.lower_expr(count)?;
                if count_type != ValueType::Integer {
                    return Err(LowerError {
                        message: "repeat count must be an integer".to_owned(),
                        span: count.span,
                    });
                }
                let body = self.lower_child_scope(body)?;
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
                let then_body = self.lower_child_scope(then_body)?;
                let else_body = self.lower_child_scope(else_body)?;
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
        let (kind, expression_type) = match &expr.kind {
            SyntaxExprKind::Integer(value) => (ExprKind::Integer(*value), ValueType::Integer),
            SyntaxExprKind::String(value) => (ExprKind::String(value.clone()), ValueType::String),
            SyntaxExprKind::Bool(value) => (ExprKind::Bool(*value), ValueType::Bool),
            SyntaxExprKind::Identifier(name) => {
                let binding = self.visible_binding(name).ok_or_else(|| LowerError {
                    message: format!(
                        "use of local {name:?} before definition or outside its scope"
                    ),
                    span: expr.span,
                })?;
                (ExprKind::Local(name.clone()), binding.value_type)
            }
            SyntaxExprKind::Call { name, arguments } => {
                let signature = self
                    .function_signatures
                    .get(name)
                    .ok_or_else(|| LowerError {
                        message: format!("unknown function {name:?}"),
                        span: expr.span,
                    })?;
                if arguments.len() != signature.parameter_types.len() {
                    return Err(LowerError {
                        message: format!(
                            "function {name:?} expects {} arguments, found {}",
                            signature.parameter_types.len(),
                            arguments.len()
                        ),
                        span: expr.span,
                    });
                }
                let mut lowered_arguments = Vec::with_capacity(arguments.len());
                for (index, (argument, expected_type)) in
                    arguments.iter().zip(&signature.parameter_types).enumerate()
                {
                    let (argument, actual_type) = self.lower_expr(argument)?;
                    if actual_type != *expected_type {
                        return Err(LowerError {
                            message: format!(
                                "argument {} for function {name:?} expects {}, found {}",
                                index + 1,
                                type_label(*expected_type),
                                type_label(actual_type)
                            ),
                            span: argument.span,
                        });
                    }
                    lowered_arguments.push(argument);
                }
                (
                    ExprKind::Call {
                        name: name.clone(),
                        arguments: lowered_arguments,
                    },
                    signature.return_type,
                )
            }
            SyntaxExprKind::InputInt => (ExprKind::InputInt, ValueType::Integer),
            SyntaxExprKind::LogicalNot(inner) => {
                let (inner, inner_type) = self.lower_expr(inner)?;
                if inner_type != ValueType::Bool {
                    return Err(LowerError {
                        message: "logical 'not' requires a boolean operand".to_owned(),
                        span: expr.span,
                    });
                }
                (ExprKind::LogicalNot(Box::new(inner)), ValueType::Bool)
            }
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
                    BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide => {
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
                                message: "equality operands must have the same value type"
                                    .to_owned(),
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
                    BinaryOp::And | BinaryOp::Or => {
                        if left_type != ValueType::Bool || right_type != ValueType::Bool {
                            return Err(LowerError {
                                message: "logical 'and'/'or' operators require boolean operands"
                                    .to_owned(),
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
            expression_type,
        ))
    }

    fn apply_mutability(&self, statements: &mut [Stmt]) {
        for statement in statements {
            match &mut statement.kind {
                StmtKind::Let { mutable, .. } => {
                    *mutable = self.mutable_declarations.contains(&statement.span.start);
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
                StmtKind::Assign { .. } | StmtKind::Print(_) | StmtKind::Return(_) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ExprKind, StmtKind, ValueType, lower};
    use evo_lexer::lex;
    use evo_parser::parse;

    fn lower_source(source: &str) -> Result<super::Program, super::LowerError> {
        let tokens = lex(source).expect("lexing should succeed");
        let syntax = parse(&tokens).expect("parsing should succeed");
        lower(&syntax)
    }

    #[test]
    fn rejects_parsed_records_until_semantic_lowering_lands() {
        let error = lower_source("record Point\nx int\ny int\nend\nprint 1\n")
            .expect_err("record declarations must not be silently ignored");
        assert!(error.message.contains("Records v0 semantic lowering"));
        assert_eq!(error.span.line, 1);
    }

    #[test]
    fn lowers_forward_calls_and_explicit_signatures() {
        let program =
            lower_source("print add(2, 3)\nfn add(a int, b int) int\nreturn a + b\nend\n")
                .expect("forward call should lower");
        assert_eq!(program.functions.len(), 1);
        assert_eq!(program.functions[0].return_type, ValueType::Integer);
        assert_eq!(program.functions[0].parameters.len(), 2);
        let StmtKind::Print(expr) = &program.statements[0].kind else {
            panic!("expected print");
        };
        assert!(matches!(expr.kind, ExprKind::Call { .. }));
    }

    #[test]
    fn direct_recursion_is_allowed_by_explicit_signature() {
        lower_source(
            "fn countdown(n int) int\nif n <= 0\nreturn 0\nelse\nreturn countdown(n - 1)\nend\nend\n",
        )
        .expect("recursive function should lower");
    }

    #[test]
    fn rejects_duplicate_function_names() {
        let error = lower_source("fn a() int\nreturn 1\nend\nfn a() int\nreturn 2\nend\n")
            .expect_err("duplicate function should fail");
        assert!(error.message.contains("duplicate function"));
    }

    #[test]
    fn rejects_duplicate_parameter_names() {
        let error = lower_source("fn add(a int, a int) int\nreturn a\nend\n")
            .expect_err("duplicate parameter should fail");
        assert!(error.message.contains("duplicate parameter"));
    }

    #[test]
    fn rejects_unknown_function_and_wrong_arity() {
        let error = lower_source("print missing(1)\n").expect_err("unknown call should fail");
        assert!(error.message.contains("unknown function"));

        let error = lower_source("fn id(x int) int\nreturn x\nend\nprint id(1, 2)\n")
            .expect_err("wrong arity should fail");
        assert!(error.message.contains("expects 1 arguments"));
    }

    #[test]
    fn rejects_wrong_argument_and_return_types() {
        let error = lower_source("fn id(x int) int\nreturn x\nend\nprint id(true)\n")
            .expect_err("wrong argument type should fail");
        assert!(error.message.contains("argument 1"));

        let error = lower_source("fn bad() int\nreturn true\nend\n")
            .expect_err("wrong return type should fail");
        assert!(error.message.contains("return type mismatch"));
    }

    #[test]
    fn requires_return_on_every_terminal_path() {
        let error = lower_source("fn maybe(flag bool) int\nif flag\nreturn 1\nend\nend\n")
            .expect_err("missing path return should fail");
        assert!(error.message.contains("every terminal path"));

        lower_source("fn choose(flag bool) int\nif flag\nreturn 1\nelse\nreturn 2\nend\nend\n")
            .expect("both branches return");
    }

    #[test]
    fn top_level_locals_are_not_captured() {
        let error = lower_source("x = 7\nfn get() int\nreturn x\nend\n")
            .expect_err("function should not capture top level");
        assert!(error.message.contains("before definition"));
    }

    #[test]
    fn parameter_reassignment_infers_parameter_mutability() {
        let program = lower_source("fn bump(x int) int\nx = x + 1\nreturn x\nend\n")
            .expect("parameter reassignment should lower");
        assert!(program.functions[0].parameters[0].mutable);
    }

    #[test]
    fn existing_top_level_runtime_semantics_still_lower() {
        let program =
            lower_source("n = input_int\nsum = 0\nrepeat n\nsum = sum + 1\nend\nprint sum\n")
                .expect("existing top-level program should lower");
        assert!(program.functions.is_empty());
        assert_eq!(program.statements.len(), 4);
    }
}

#[cfg(test)]
mod block_local_scope_tests {
    use super::{StmtKind, lower};
    use evo_lexer::lex;
    use evo_parser::parse;

    fn lower_source(source: &str) -> Result<super::Program, super::LowerError> {
        let tokens = lex(source).expect("lexing should succeed");
        let syntax = parse(&tokens).expect("parsing should succeed");
        lower(&syntax)
    }

    #[test]
    fn if_local_is_usable_only_inside_branch() {
        let program = lower_source("if true\ninside = 1\nprint inside\nend\n")
            .expect("branch local should lower");
        let StmtKind::If { then_body, .. } = &program.statements[0].kind else {
            panic!("expected if statement");
        };
        assert!(matches!(&then_body[0].kind, StmtKind::Let { name, .. } if name == "inside"));

        let error = lower_source("if true\ninside = 1\nend\nprint inside\n")
            .expect_err("branch local must not leak");
        assert!(error.message.contains("outside its scope"));
    }

    #[test]
    fn sibling_branches_get_independent_local_bindings() {
        let program = lower_source(
            "flag = true\nif flag\ntemp = 1\nprint temp\nelse\ntemp = 2\ntemp = temp + 1\nprint temp\nend\n",
        )
        .expect("sibling locals should lower independently");
        let StmtKind::If {
            then_body,
            else_body,
            ..
        } = &program.statements[1].kind
        else {
            panic!("expected if statement");
        };
        assert!(matches!(
            &then_body[0].kind,
            StmtKind::Let {
                name,
                mutable: false,
                ..
            } if name == "temp"
        ));
        assert!(matches!(
            &else_body[0].kind,
            StmtKind::Let {
                name,
                mutable: true,
                ..
            } if name == "temp"
        ));
    }

    #[test]
    fn repeat_local_can_reassign_and_does_not_escape() {
        let program =
            lower_source("x = 0\nrepeat 2\ntemp = x + 1\ntemp = temp + 1\nx = x + temp\nend\n")
                .expect("repeat local should lower");
        assert!(matches!(
            &program.statements[0].kind,
            StmtKind::Let {
                name,
                mutable: true,
                ..
            } if name == "x"
        ));
        let StmtKind::Repeat { body, .. } = &program.statements[1].kind else {
            panic!("expected repeat statement");
        };
        assert!(matches!(
            &body[0].kind,
            StmtKind::Let {
                name,
                mutable: true,
                ..
            } if name == "temp"
        ));

        let error = lower_source("repeat 1\ntemp = 1\nend\nprint temp\n")
            .expect_err("repeat local must not leak");
        assert!(error.message.contains("outside its scope"));
    }

    #[test]
    fn nested_child_can_read_parent_block_local_but_not_leak_back() {
        lower_source("if true\na = 1\nif true\nb = a + 1\nprint b\nend\nprint a\nend\n")
            .expect("nested child should read parent block local");

        let error = lower_source("if true\na = 1\nif true\nb = a + 1\nend\nprint b\nend\n")
            .expect_err("nested child local must not leak to parent");
        assert!(error.message.contains("outside its scope"));
    }

    #[test]
    fn function_control_flow_uses_lexical_child_scopes() {
        lower_source(
            "fn choose(flag bool, x int) int\nif flag\ny = x + 1\nreturn y\nelse\nz = x - 1\nreturn z\nend\nend\nprint choose(true, 2)\n",
        )
        .expect("function block locals should lower");
    }

    #[test]
    fn visible_outer_name_remains_reassignment_not_shadowing() {
        let program = lower_source("x = 1\nif true\nx = 2\nend\n")
            .expect("visible outer assignment should lower");
        assert!(matches!(
            &program.statements[0].kind,
            StmtKind::Let {
                name,
                mutable: true,
                ..
            } if name == "x"
        ));
        let StmtKind::If { then_body, .. } = &program.statements[1].kind else {
            panic!("expected if statement");
        };
        assert!(matches!(&then_body[0].kind, StmtKind::Assign { name, .. } if name == "x"));
    }
}
