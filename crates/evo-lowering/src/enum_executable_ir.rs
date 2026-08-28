use evo_lexer::Span;
use evo_parser::{
    Expr as SyntaxExpr, ExprKind as SyntaxExprKind, FunctionDef as SyntaxFunction,
    Program as SyntaxProgram, Stmt as SyntaxStmt, StmtKind as SyntaxStmtKind, TypeName,
};
use std::collections::{HashMap, HashSet};

use super::{
    ir::{EnumConstructorIr, MatchArmIr, MatchIr, SchemaType},
    ownership_ir::{OwnershipUseIr, OwnershipUseModeIr},
    program_ir::EnumProgramIr,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExecutableValueType {
    Integer,
    String,
    Bool,
    Record(String),
    Enum(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutableOwnershipMode {
    Inspect,
    Consume,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutableEnumVariantIr {
    pub(crate) name: String,
    pub(crate) payload_type: Option<ExecutableValueType>,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutableEnumIr {
    pub(crate) name: String,
    pub(crate) variants: Vec<ExecutableEnumVariantIr>,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutableRecordFieldIr {
    pub(crate) name: String,
    pub(crate) value_type: ExecutableValueType,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutableRecordIr {
    pub(crate) name: String,
    pub(crate) fields: Vec<ExecutableRecordFieldIr>,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutableParameterIr {
    pub(crate) name: String,
    pub(crate) value_type: ExecutableValueType,
    pub(crate) mutable: bool,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutableFunctionIr {
    pub(crate) name: String,
    pub(crate) parameters: Vec<ExecutableParameterIr>,
    pub(crate) return_type: ExecutableValueType,
    pub(crate) body: Vec<ExecutableStmtIr>,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutableStmtIr {
    pub(crate) kind: ExecutableStmtKind,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExecutableStmtKind {
    Let {
        name: String,
        mutable: bool,
        expr: ExecutableExprIr,
    },
    Assign {
        name: String,
        expr: ExecutableExprIr,
    },
    Print(ExecutableExprIr),
    Return(ExecutableExprIr),
    Repeat {
        count: ExecutableExprIr,
        body: Vec<ExecutableStmtIr>,
    },
    If {
        condition: ExecutableExprIr,
        then_body: Vec<ExecutableStmtIr>,
        else_body: Vec<ExecutableStmtIr>,
    },
    Match {
        value: ExecutableExprIr,
        enum_name: String,
        arms: Vec<ExecutableMatchArmIr>,
        all_arms_return: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutableMatchBindingIr {
    pub(crate) name: String,
    pub(crate) value_type: ExecutableValueType,
    pub(crate) mutable: bool,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutableMatchArmIr {
    pub(crate) enum_name: String,
    pub(crate) variant_name: String,
    pub(crate) binding: Option<ExecutableMatchBindingIr>,
    pub(crate) body: Vec<ExecutableStmtIr>,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutableExprIr {
    pub(crate) kind: ExecutableExprKind,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutableRecordFieldValueIr {
    pub(crate) name: String,
    pub(crate) value: ExecutableExprIr,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExecutableExprKind {
    Integer(i64),
    String(String),
    Bool(bool),
    Local {
        name: String,
        value_type: ExecutableValueType,
        ownership: ExecutableOwnershipMode,
    },
    Call {
        name: String,
        arguments: Vec<ExecutableExprIr>,
    },
    RecordConstruct {
        name: String,
        fields: Vec<ExecutableRecordFieldValueIr>,
    },
    EnumConstruct {
        enum_name: String,
        variant_name: String,
        payload_type: Option<ExecutableValueType>,
        payload: Option<Box<ExecutableExprIr>>,
    },
    FieldAccess {
        base: Box<ExecutableExprIr>,
        field: String,
    },
    InputInt,
    LogicalNot(Box<ExecutableExprIr>),
    UnaryMinus(Box<ExecutableExprIr>),
    Binary {
        left: Box<ExecutableExprIr>,
        op: evo_parser::BinaryOp,
        right: Box<ExecutableExprIr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutableEnumProgramIr {
    pub(crate) enums: Vec<ExecutableEnumIr>,
    pub(crate) records: Vec<ExecutableRecordIr>,
    pub(crate) functions: Vec<ExecutableFunctionIr>,
    pub(crate) statements: Vec<ExecutableStmtIr>,
}

pub(super) fn lower_executable_enum_program(
    syntax: &SyntaxProgram,
    validated: &EnumProgramIr,
) -> ExecutableEnumProgramIr {
    let enums = validated
        .enums
        .iter()
        .map(|schema| ExecutableEnumIr {
            name: schema.name.clone(),
            variants: schema
                .variants
                .iter()
                .map(|variant| ExecutableEnumVariantIr {
                    name: variant.name.clone(),
                    payload_type: variant.payload_type.as_ref().map(lower_value_type),
                    span: variant.span,
                })
                .collect(),
            span: schema.span,
        })
        .collect();
    let records = validated
        .records
        .iter()
        .map(|record| ExecutableRecordIr {
            name: record.name.clone(),
            fields: record
                .fields
                .iter()
                .map(|field| ExecutableRecordFieldIr {
                    name: field.name.clone(),
                    value_type: lower_value_type(&field.value_type),
                    span: field.span,
                })
                .collect(),
            span: record.span,
        })
        .collect();

    let mut functions = Vec::with_capacity(syntax.functions.len());
    for function in &syntax.functions {
        functions.push(BodyPromoter::new(validated).lower_function(function));
    }

    let mut top_level = BodyPromoter::new(validated);
    let mut statements = top_level.lower_statements(&syntax.statements);
    top_level.apply_mutability(&mut statements);

    ExecutableEnumProgramIr {
        enums,
        records,
        functions,
        statements,
    }
}

struct BodyPromoter<'a> {
    validated: &'a EnumProgramIr,
    scopes: Vec<HashMap<String, usize>>,
    mutable_declarations: HashSet<usize>,
}

impl<'a> BodyPromoter<'a> {
    fn new(validated: &'a EnumProgramIr) -> Self {
        Self {
            validated,
            scopes: vec![HashMap::new()],
            mutable_declarations: HashSet::new(),
        }
    }

    fn lower_function(&mut self, function: &SyntaxFunction) -> ExecutableFunctionIr {
        let mut parameters = Vec::with_capacity(function.parameters.len());
        for parameter in &function.parameters {
            self.define_binding(parameter.name.clone(), parameter.span.start);
            parameters.push(ExecutableParameterIr {
                name: parameter.name.clone(),
                value_type: self.lower_type_name(&parameter.type_name),
                mutable: false,
                span: parameter.span,
            });
        }

        let mut body = self.lower_statements(&function.body);
        self.apply_mutability(&mut body);
        for parameter in &mut parameters {
            parameter.mutable = self
                .mutable_declarations
                .contains(&parameter.span.start);
        }

        ExecutableFunctionIr {
            name: function.name.clone(),
            parameters,
            return_type: self.lower_type_name(&function.return_type),
            body,
            span: function.span,
        }
    }

    fn lower_statements(&mut self, statements: &[SyntaxStmt]) -> Vec<ExecutableStmtIr> {
        statements
            .iter()
            .map(|statement| self.lower_statement(statement))
            .collect()
    }

    fn lower_statement(&mut self, statement: &SyntaxStmt) -> ExecutableStmtIr {
        let kind = match &statement.kind {
            SyntaxStmtKind::Bind { name, expr } => {
                let expr = self.lower_expr(expr);
                if let Some(declaration_start) = self.visible_declaration(name) {
                    self.mutable_declarations.insert(declaration_start);
                    ExecutableStmtKind::Assign {
                        name: name.clone(),
                        expr,
                    }
                } else {
                    self.define_binding(name.clone(), statement.span.start);
                    ExecutableStmtKind::Let {
                        name: name.clone(),
                        mutable: false,
                        expr,
                    }
                }
            }
            SyntaxStmtKind::Print(expr) => ExecutableStmtKind::Print(self.lower_expr(expr)),
            SyntaxStmtKind::Return(expr) => ExecutableStmtKind::Return(self.lower_expr(expr)),
            SyntaxStmtKind::Repeat { count, body } => ExecutableStmtKind::Repeat {
                count: self.lower_expr(count),
                body: self.lower_child_scope(body),
            },
            SyntaxStmtKind::If {
                condition,
                then_body,
                else_body,
            } => ExecutableStmtKind::If {
                condition: self.lower_expr(condition),
                then_body: self.lower_child_scope(then_body),
                else_body: self.lower_child_scope(else_body),
            },
            SyntaxStmtKind::Match { value, arms } => {
                let resolved = self.match_at(statement.span.start).clone();
                debug_assert_eq!(resolved.arms.len(), arms.len());
                let value = self.lower_expr(value);
                let lowered_arms = arms
                    .iter()
                    .zip(&resolved.arms)
                    .map(|(arm, resolved_arm)| self.lower_match_arm(arm, resolved_arm))
                    .collect();
                ExecutableStmtKind::Match {
                    value,
                    enum_name: resolved.enum_name,
                    arms: lowered_arms,
                    all_arms_return: resolved.all_arms_return,
                }
            }
        };

        ExecutableStmtIr {
            kind,
            span: statement.span,
        }
    }

    fn lower_match_arm(
        &mut self,
        arm: &evo_parser::MatchArm,
        resolved: &MatchArmIr,
    ) -> ExecutableMatchArmIr {
        debug_assert_eq!(arm.pattern.enum_name, resolved.enum_name);
        debug_assert_eq!(arm.pattern.variant_name, resolved.variant_name);
        self.scopes.push(HashMap::new());
        let binding = resolved.binding.as_ref().map(|binding| {
            self.define_binding(binding.name.clone(), binding.span.start);
            ExecutableMatchBindingIr {
                name: binding.name.clone(),
                value_type: lower_value_type(&binding.value_type),
                mutable: false,
                span: binding.span,
            }
        });
        let body = self.lower_statements(&arm.body);
        self.scopes
            .pop()
            .expect("match arm promotion must retain its lexical scope");

        ExecutableMatchArmIr {
            enum_name: resolved.enum_name.clone(),
            variant_name: resolved.variant_name.clone(),
            binding,
            body,
            span: resolved.span,
        }
    }

    fn lower_child_scope(&mut self, statements: &[SyntaxStmt]) -> Vec<ExecutableStmtIr> {
        self.scopes.push(HashMap::new());
        let body = self.lower_statements(statements);
        self.scopes
            .pop()
            .expect("child promotion must retain its lexical scope");
        body
    }

    fn lower_expr(&mut self, expr: &SyntaxExpr) -> ExecutableExprIr {
        let kind = match &expr.kind {
            SyntaxExprKind::Integer(value) => ExecutableExprKind::Integer(*value),
            SyntaxExprKind::String(value) => ExecutableExprKind::String(value.clone()),
            SyntaxExprKind::Bool(value) => ExecutableExprKind::Bool(*value),
            SyntaxExprKind::Identifier(name) => {
                let ownership = self.ownership_use_at(expr.span.start);
                debug_assert_eq!(ownership.name, *name);
                ExecutableExprKind::Local {
                    name: name.clone(),
                    value_type: lower_value_type(&ownership.value_type),
                    ownership: match ownership.mode {
                        OwnershipUseModeIr::Inspect => ExecutableOwnershipMode::Inspect,
                        OwnershipUseModeIr::Consume => ExecutableOwnershipMode::Consume,
                    },
                }
            }
            SyntaxExprKind::Call { name, arguments } => {
                if let Some(record) = self
                    .validated
                    .records
                    .iter()
                    .find(|record| record.name == *name)
                {
                    debug_assert!(arguments.is_empty());
                    debug_assert!(record.fields.is_empty());
                    ExecutableExprKind::RecordConstruct {
                        name: name.clone(),
                        fields: Vec::new(),
                    }
                } else {
                    ExecutableExprKind::Call {
                        name: name.clone(),
                        arguments: arguments
                            .iter()
                            .map(|argument| self.lower_expr(argument))
                            .collect(),
                    }
                }
            }
            SyntaxExprKind::Construct { name, fields } => ExecutableExprKind::RecordConstruct {
                name: name.clone(),
                fields: fields
                    .iter()
                    .map(|field| ExecutableRecordFieldValueIr {
                        name: field.name.clone(),
                        value: self.lower_expr(&field.value),
                        span: field.span,
                    })
                    .collect(),
            },
            SyntaxExprKind::EnumConstruct {
                enum_name,
                variant_name,
                arguments,
            } => {
                let resolved = self.constructor_at(expr.span.start).clone();
                debug_assert_eq!(resolved.enum_name, *enum_name);
                debug_assert_eq!(resolved.variant_name, *variant_name);
                debug_assert!(arguments.len() <= 1);
                ExecutableExprKind::EnumConstruct {
                    enum_name: resolved.enum_name,
                    variant_name: resolved.variant_name,
                    payload_type: resolved.payload_type.as_ref().map(lower_value_type),
                    payload: arguments
                        .first()
                        .map(|argument| Box::new(self.lower_expr(argument))),
                }
            }
            SyntaxExprKind::FieldAccess { base, field } => ExecutableExprKind::FieldAccess {
                base: Box::new(self.lower_expr(base)),
                field: field.clone(),
            },
            SyntaxExprKind::InputInt => ExecutableExprKind::InputInt,
            SyntaxExprKind::LogicalNot(inner) => {
                ExecutableExprKind::LogicalNot(Box::new(self.lower_expr(inner)))
            }
            SyntaxExprKind::UnaryMinus(inner) => {
                ExecutableExprKind::UnaryMinus(Box::new(self.lower_expr(inner)))
            }
            SyntaxExprKind::Binary { left, op, right } => ExecutableExprKind::Binary {
                left: Box::new(self.lower_expr(left)),
                op: *op,
                right: Box::new(self.lower_expr(right)),
            },
        };

        ExecutableExprIr {
            kind,
            span: expr.span,
        }
    }

    fn lower_type_name(&self, type_name: &TypeName) -> ExecutableValueType {
        match type_name {
            TypeName::Int => ExecutableValueType::Integer,
            TypeName::Bool => ExecutableValueType::Bool,
            TypeName::String => ExecutableValueType::String,
            TypeName::Named(name)
                if self
                    .validated
                    .enums
                    .iter()
                    .any(|schema| schema.name == *name) =>
            {
                ExecutableValueType::Enum(name.clone())
            }
            TypeName::Named(name) => {
                debug_assert!(
                    self.validated
                        .records
                        .iter()
                        .any(|record| record.name == *name)
                );
                ExecutableValueType::Record(name.clone())
            }
        }
    }

    fn visible_declaration(&self, name: &str) -> Option<usize> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    fn define_binding(&mut self, name: String, declaration_start: usize) {
        self.scopes
            .last_mut()
            .expect("executable promotion always has a lexical scope")
            .insert(name, declaration_start);
    }

    fn constructor_at(&self, start: usize) -> &EnumConstructorIr {
        self.validated
            .constructors
            .iter()
            .find(|constructor| constructor.span.start == start)
            .expect("executable constructor promotion requires validated constructor metadata")
    }

    fn match_at(&self, start: usize) -> &MatchIr {
        self.validated
            .matches
            .iter()
            .find(|resolved| resolved.span.start == start)
            .expect("executable match promotion requires validated match metadata")
    }

    fn ownership_use_at(&self, start: usize) -> &OwnershipUseIr {
        self.validated
            .ownership_uses
            .iter()
            .find(|usage| usage.span.start == start)
            .expect("executable local promotion requires validated ownership metadata")
    }

    fn apply_mutability(&self, statements: &mut [ExecutableStmtIr]) {
        for statement in statements {
            match &mut statement.kind {
                ExecutableStmtKind::Let { mutable, .. } => {
                    *mutable = self.mutable_declarations.contains(&statement.span.start);
                }
                ExecutableStmtKind::Repeat { body, .. } => self.apply_mutability(body),
                ExecutableStmtKind::If {
                    then_body,
                    else_body,
                    ..
                } => {
                    self.apply_mutability(then_body);
                    self.apply_mutability(else_body);
                }
                ExecutableStmtKind::Match { arms, .. } => {
                    for arm in arms {
                        if let Some(binding) = &mut arm.binding {
                            binding.mutable = self
                                .mutable_declarations
                                .contains(&binding.span.start);
                        }
                        self.apply_mutability(&mut arm.body);
                    }
                }
                ExecutableStmtKind::Assign { .. }
                | ExecutableStmtKind::Print(_)
                | ExecutableStmtKind::Return(_) => {}
            }
        }
    }
}

fn lower_value_type(value_type: &SchemaType) -> ExecutableValueType {
    match value_type {
        SchemaType::Integer => ExecutableValueType::Integer,
        SchemaType::Bool => ExecutableValueType::Bool,
        SchemaType::String => ExecutableValueType::String,
        SchemaType::Record(name) => ExecutableValueType::Record(name.clone()),
        SchemaType::Enum(name) => ExecutableValueType::Enum(name.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ExecutableExprKind, ExecutableOwnershipMode, ExecutableStmtKind, ExecutableValueType,
        lower_executable_enum_program,
    };
    use evo_lexer::lex;
    use evo_parser::parse;

    fn lower(source: &str) -> super::ExecutableEnumProgramIr {
        let tokens = lex(source).expect("executable enum IR source should lex");
        let syntax = parse(&tokens).expect("executable enum IR source should parse");
        let validated = super::super::collect_validated_enum_program_ir(&syntax)
            .expect("executable enum IR source should pass pre-codegen validation");
        lower_executable_enum_program(&syntax, &validated)
    }

    #[test]
    fn executable_ir_embeds_structured_types_constructor_payloads_matches_and_ownership() {
        let lowered = lower(
            "enum Inner\nA int\nend\nenum Wrapped\nNone\nSome Inner\nend\nrecord Holder\nvalue Wrapped\nend\nfn identity(value Inner) Inner\nreturn value\nend\nvalue = Wrapped.Some(Inner.A(1))\nmatch value\ncase Wrapped.None\nprint 0\ncase Wrapped.Some(x)\nmatch x\ncase Inner.A(y)\nprint y\nend\nend\n",
        );

        assert_eq!(lowered.enums.len(), 2);
        assert_eq!(
            lowered.enums[1].variants[1].payload_type,
            Some(ExecutableValueType::Enum("Inner".to_owned()))
        );
        assert_eq!(lowered.records.len(), 1);
        assert_eq!(
            lowered.records[0].fields[0].value_type,
            ExecutableValueType::Enum("Wrapped".to_owned())
        );

        assert_eq!(lowered.functions.len(), 1);
        assert_eq!(
            lowered.functions[0].parameters[0].value_type,
            ExecutableValueType::Enum("Inner".to_owned())
        );
        assert_eq!(
            lowered.functions[0].return_type,
            ExecutableValueType::Enum("Inner".to_owned())
        );
        let ExecutableStmtKind::Return(returned) = &lowered.functions[0].body[0].kind else {
            panic!("identity body should remain a return");
        };
        let ExecutableExprKind::Local {
            name,
            value_type,
            ownership,
        } = &returned.kind
        else {
            panic!("identity return should retain local ownership metadata");
        };
        assert_eq!(name, "value");
        assert_eq!(value_type, &ExecutableValueType::Enum("Inner".to_owned()));
        assert_eq!(*ownership, ExecutableOwnershipMode::Consume);

        let ExecutableStmtKind::Let { expr, .. } = &lowered.statements[0].kind else {
            panic!("top-level enum construction should become a let");
        };
        let ExecutableExprKind::EnumConstruct {
            enum_name,
            variant_name,
            payload_type,
            payload,
        } = &expr.kind
        else {
            panic!("top-level value should retain enum constructor identity");
        };
        assert_eq!(enum_name, "Wrapped");
        assert_eq!(variant_name, "Some");
        assert_eq!(
            payload_type,
            &Some(ExecutableValueType::Enum("Inner".to_owned()))
        );
        let nested = payload.as_ref().expect("Some should retain its payload");
        assert!(matches!(
            &nested.kind,
            ExecutableExprKind::EnumConstruct { .. }
        ));

        let ExecutableStmtKind::Match {
            value,
            enum_name,
            arms,
            ..
        } = &lowered.statements[1].kind
        else {
            panic!("top-level match should remain structured");
        };
        assert_eq!(enum_name, "Wrapped");
        let ExecutableExprKind::Local { ownership, .. } = &value.kind else {
            panic!("match scrutinee should retain local ownership metadata");
        };
        assert_eq!(*ownership, ExecutableOwnershipMode::Consume);
        let binding = arms[1]
            .binding
            .as_ref()
            .expect("payload arm should retain typed binding");
        assert_eq!(binding.name, "x");
        assert_eq!(
            binding.value_type,
            ExecutableValueType::Enum("Inner".to_owned())
        );
        let ExecutableStmtKind::Match {
            enum_name: nested_enum,
            arms: nested_arms,
            ..
        } = &arms[1].body[0].kind
        else {
            panic!("payload arm should retain nested match");
        };
        assert_eq!(nested_enum, "Inner");
        assert_eq!(
            nested_arms[0]
                .binding
                .as_ref()
                .expect("Inner.A should retain scalar binding")
                .value_type,
            ExecutableValueType::Integer
        );
    }

    #[test]
    fn executable_ir_resolves_assignment_and_match_binding_mutability_once() {
        let lowered = lower(
            "enum Maybe\nNone\nSome int\nend\nvalue = Maybe.Some(1)\nvalue = Maybe.None()\nmatch value\ncase Maybe.None\nprint 0\ncase Maybe.Some(x)\nx = 2\nprint x\nend\n",
        );

        let ExecutableStmtKind::Let { mutable, .. } = &lowered.statements[0].kind else {
            panic!("first value binding should become a let");
        };
        assert!(*mutable);
        assert!(matches!(
            &lowered.statements[1].kind,
            ExecutableStmtKind::Assign { .. }
        ));

        let ExecutableStmtKind::Match { arms, .. } = &lowered.statements[2].kind else {
            panic!("third statement should remain a match");
        };
        let binding = arms[1]
            .binding
            .as_ref()
            .expect("Some arm should retain payload binding");
        assert!(binding.mutable);
        assert!(matches!(
            &arms[1].body[0].kind,
            ExecutableStmtKind::Assign { .. }
        ));
        let ExecutableStmtKind::Print(printed) = &arms[1].body[1].kind else {
            panic!("payload should remain printable after scalar reassignment");
        };
        let ExecutableExprKind::Local { ownership, .. } = &printed.kind else {
            panic!("print should retain local ownership metadata");
        };
        assert_eq!(*ownership, ExecutableOwnershipMode::Inspect);
    }
}
