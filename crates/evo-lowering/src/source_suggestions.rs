use evo_diagnostics::{best_suggestion, clear_help, set_help};
use evo_lexer::Span;
use evo_parser::{Expr, ExprKind, Program, RecordFieldType, Stmt, StmtKind, TypeName};
use std::collections::{HashMap, HashSet};

type Scope = HashMap<String, Option<String>>;

#[derive(Debug)]
struct SuggestionCatalog {
    record_names: Vec<String>,
    enum_names: Vec<String>,
    function_names: Vec<String>,
    record_fields: HashMap<String, HashMap<String, Option<String>>>,
    enum_variant_record_payloads: HashMap<String, HashMap<String, Option<String>>>,
    function_record_returns: HashMap<String, String>,
}

impl SuggestionCatalog {
    fn new(program: &Program) -> Self {
        let record_names: Vec<String> = program
            .records
            .iter()
            .map(|record| record.name.clone())
            .collect();
        let enum_names: Vec<String> = program
            .enums
            .iter()
            .map(|enum_def| enum_def.name.clone())
            .collect();
        let function_names = program
            .functions
            .iter()
            .map(|function| function.name.clone())
            .collect();
        let record_name_set: HashSet<&str> = record_names.iter().map(String::as_str).collect();

        let mut record_fields = HashMap::new();
        for record in &program.records {
            let fields = record
                .fields
                .iter()
                .map(|field| {
                    let record_type = match &field.type_name {
                        RecordFieldType::Named(name) if record_name_set.contains(name.as_str()) => {
                            Some(name.clone())
                        }
                        RecordFieldType::Int
                        | RecordFieldType::Bool
                        | RecordFieldType::String
                        | RecordFieldType::Named(_) => None,
                    };
                    (field.name.clone(), record_type)
                })
                .collect();
            record_fields.insert(record.name.clone(), fields);
        }

        let mut enum_variant_record_payloads = HashMap::new();
        for enum_def in &program.enums {
            let variants = enum_def
                .variants
                .iter()
                .map(|variant| {
                    let record_payload = variant.payload_type.as_ref().and_then(|type_name| {
                        named_type(type_name)
                            .filter(|name| record_name_set.contains(*name))
                            .map(str::to_owned)
                    });
                    (variant.name.clone(), record_payload)
                })
                .collect();
            enum_variant_record_payloads.insert(enum_def.name.clone(), variants);
        }

        let function_record_returns = program
            .functions
            .iter()
            .filter_map(|function| {
                named_type(&function.return_type)
                    .filter(|name| record_name_set.contains(*name))
                    .map(|name| (function.name.clone(), name.to_owned()))
            })
            .collect();

        Self {
            record_names,
            enum_names,
            function_names,
            record_fields,
            enum_variant_record_payloads,
            function_record_returns,
        }
    }

    fn record_names(&self) -> impl Iterator<Item = &str> {
        self.record_names.iter().map(String::as_str)
    }

    fn enum_names(&self) -> impl Iterator<Item = &str> {
        self.enum_names.iter().map(String::as_str)
    }

    fn function_names(&self) -> impl Iterator<Item = &str> {
        self.function_names.iter().map(String::as_str)
    }

    fn nominal_names(&self) -> impl Iterator<Item = &str> {
        self.record_names().chain(self.enum_names())
    }

    fn has_record(&self, name: &str) -> bool {
        self.record_fields.contains_key(name)
    }

    fn has_enum(&self, name: &str) -> bool {
        self.enum_variant_record_payloads.contains_key(name)
    }

    fn has_function(&self, name: &str) -> bool {
        self.function_names
            .iter()
            .any(|candidate| candidate == name)
    }

    fn register_declaration_suggestions(&self, program: &Program) {
        for record in &program.records {
            for field in &record.fields {
                if let RecordFieldType::Named(name) = &field.type_name {
                    if !self.has_record(name) {
                        let record_message = format!(
                            "unknown record type {name:?} for field {:?} in record {:?}",
                            field.name, record.name
                        );
                        register(&record_message, field.span, name, self.record_names());
                    }
                    if !self.has_record(name) && !self.has_enum(name) {
                        let nominal_message = format!(
                            "unknown nominal type {name:?} for field {:?} in record {:?}",
                            field.name, record.name
                        );
                        register(&nominal_message, field.span, name, self.nominal_names());
                    }
                }
            }
        }

        for enum_def in &program.enums {
            for variant in &enum_def.variants {
                if let Some(TypeName::Named(name)) = &variant.payload_type
                    && !self.has_record(name)
                    && !self.has_enum(name)
                {
                    let message = format!(
                        "unknown payload type {name:?} for variant {:?} in enum {:?}",
                        variant.name, enum_def.name
                    );
                    register(&message, variant.span, name, self.nominal_names());
                }
            }
        }

        for function in &program.functions {
            for parameter in &function.parameters {
                self.register_signature_type(&parameter.type_name, parameter.span);
            }
            self.register_signature_type(&function.return_type, function.span);
        }
    }

    fn register_signature_type(&self, type_name: &TypeName, span: Span) {
        let TypeName::Named(name) = type_name else {
            return;
        };
        if self.has_record(name) || self.has_enum(name) {
            return;
        }
        let message = format!("unknown nominal type {name:?} in function signature");
        register(&message, span, name, self.nominal_names());
    }

    fn walk_statements(&self, statements: &[Stmt], scopes: &mut Vec<Scope>) {
        for statement in statements {
            match &statement.kind {
                StmtKind::Bind { name, expr } => {
                    let record_hint = self.walk_expr(expr, scopes);
                    if visible(scopes, name).is_none() {
                        scopes
                            .last_mut()
                            .expect("suggestion traversal always has a lexical scope")
                            .insert(name.clone(), record_hint);
                    }
                }
                StmtKind::Print(expr) | StmtKind::Return(expr) => {
                    let _ = self.walk_expr(expr, scopes);
                }
                StmtKind::Repeat { count, body } => {
                    let _ = self.walk_expr(count, scopes);
                    self.walk_child(body, scopes, None);
                }
                StmtKind::If {
                    condition,
                    then_body,
                    else_body,
                } => {
                    let _ = self.walk_expr(condition, scopes);
                    self.walk_child(then_body, scopes, None);
                    self.walk_child(else_body, scopes, None);
                }
                StmtKind::Match { value, arms } => {
                    let _ = self.walk_expr(value, scopes);
                    for arm in arms {
                        let enum_name = arm.pattern.enum_name.as_str();
                        let variant_name = arm.pattern.variant_name.as_str();
                        if !self.has_enum(enum_name) {
                            let message = format!(
                                "unknown enum {:?} in match pattern",
                                arm.pattern.enum_name
                            );
                            register(&message, arm.pattern.span, enum_name, self.enum_names());
                        } else if !self
                            .enum_variant_record_payloads
                            .get(enum_name)
                            .is_some_and(|variants| variants.contains_key(variant_name))
                        {
                            let message = format!(
                                "unknown variant {:?} for enum {enum_name:?} in match pattern",
                                arm.pattern.variant_name
                            );
                            let candidates = self
                                .enum_variant_record_payloads
                                .get(enum_name)
                                .expect("known enum has a variant table")
                                .keys()
                                .map(String::as_str);
                            register(&message, arm.pattern.span, variant_name, candidates);
                        }

                        let binding = arm.pattern.binding.as_ref().map(|name| {
                            let record_hint = self
                                .enum_variant_record_payloads
                                .get(enum_name)
                                .and_then(|variants| variants.get(variant_name))
                                .cloned()
                                .flatten();
                            (name.clone(), record_hint)
                        });
                        self.walk_child(&arm.body, scopes, binding);
                    }
                }
            }
        }
    }

    fn walk_child(
        &self,
        statements: &[Stmt],
        scopes: &mut Vec<Scope>,
        binding: Option<(String, Option<String>)>,
    ) {
        scopes.push(HashMap::new());
        if let Some((name, record_hint)) = binding {
            scopes
                .last_mut()
                .expect("child scope was just pushed")
                .insert(name, record_hint);
        }
        self.walk_statements(statements, scopes);
        let _ = scopes.pop().expect("child scope must be present");
    }

    fn walk_expr(&self, expr: &Expr, scopes: &mut Vec<Scope>) -> Option<String> {
        match &expr.kind {
            ExprKind::Integer(_) | ExprKind::String(_) | ExprKind::Bool(_) | ExprKind::InputInt => {
                None
            }
            ExprKind::Identifier(name) => {
                if let Some(record_hint) = visible(scopes, name) {
                    return record_hint.clone();
                }
                let message =
                    format!("use of local {name:?} before definition or outside its scope");
                register(&message, expr.span, name, visible_names(scopes));
                None
            }
            ExprKind::Call { name, arguments } => {
                for argument in arguments {
                    let _ = self.walk_expr(argument, scopes);
                }
                if self.has_record(name) {
                    return Some(name.clone());
                }
                if !self.has_function(name) {
                    let message = format!("unknown function {name:?}");
                    register(&message, expr.span, name, self.function_names());
                    return None;
                }
                self.function_record_returns.get(name).cloned()
            }
            ExprKind::Construct { name, fields } => {
                for field in fields {
                    let _ = self.walk_expr(&field.value, scopes);
                }
                let Some(declared_fields) = self.record_fields.get(name) else {
                    let message = format!("unknown record constructor {name:?}");
                    register(&message, expr.span, name, self.record_names());
                    return None;
                };
                for field in fields {
                    if !declared_fields.contains_key(&field.name) {
                        let message = format!(
                            "unknown constructor field {:?} for record {:?}",
                            field.name, name
                        );
                        register(
                            &message,
                            field.span,
                            &field.name,
                            declared_fields.keys().map(String::as_str),
                        );
                    }
                }
                Some(name.clone())
            }
            ExprKind::EnumConstruct {
                enum_name,
                variant_name,
                arguments,
            } => {
                for argument in arguments {
                    let _ = self.walk_expr(argument, scopes);
                }
                let Some(variants) = self.enum_variant_record_payloads.get(enum_name) else {
                    let message = format!("unknown enum constructor {enum_name:?}");
                    register(&message, expr.span, enum_name, self.enum_names());
                    return None;
                };
                if !variants.contains_key(variant_name) {
                    let message =
                        format!("unknown variant {variant_name:?} for enum {enum_name:?}");
                    register(
                        &message,
                        expr.span,
                        variant_name,
                        variants.keys().map(String::as_str),
                    );
                }
                None
            }
            ExprKind::FieldAccess { base, field } => {
                let record_name = self.walk_expr(base, scopes)?;
                let fields = self
                    .record_fields
                    .get(&record_name)
                    .expect("record hints originate from declared records");
                if let Some(record_hint) = fields.get(field) {
                    return record_hint.clone();
                }
                let message = format!("unknown field {field:?} on record {record_name:?}");
                register(
                    &message,
                    expr.span,
                    field,
                    fields.keys().map(String::as_str),
                );
                None
            }
            ExprKind::SharedBorrow(inner) => self.walk_expr(inner, scopes),
            ExprKind::LogicalNot(inner) | ExprKind::UnaryMinus(inner) => {
                let _ = self.walk_expr(inner, scopes);
                None
            }
            ExprKind::Binary { left, right, .. } => {
                let _ = self.walk_expr(left, scopes);
                let _ = self.walk_expr(right, scopes);
                None
            }
        }
    }
}

pub(crate) fn register_program_suggestions(program: &Program) {
    clear_help();
    let catalog = SuggestionCatalog::new(program);
    catalog.register_declaration_suggestions(program);

    for function in &program.functions {
        let mut root = HashMap::new();
        for parameter in &function.parameters {
            let record_hint = named_type(&parameter.type_name)
                .filter(|name| catalog.has_record(name))
                .map(str::to_owned);
            root.insert(parameter.name.clone(), record_hint);
        }
        let mut scopes = vec![root];
        catalog.walk_statements(&function.body, &mut scopes);
    }

    let mut top_level_scopes = vec![HashMap::new()];
    catalog.walk_statements(&program.statements, &mut top_level_scopes);
}

fn register<'a>(
    message: &str,
    span: Span,
    input: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) {
    if let Some(candidate) = best_suggestion(input, candidates) {
        set_help(message, span, &format!("did you mean {candidate:?}?"));
    }
}

fn named_type(type_name: &TypeName) -> Option<&str> {
    match type_name {
        TypeName::Named(name) => Some(name),
        TypeName::SharedRef(inner) => named_type(inner),
        TypeName::Int | TypeName::Bool | TypeName::String => None,
    }
}

fn visible<'a>(scopes: &'a [Scope], name: &str) -> Option<&'a Option<String>> {
    scopes.iter().rev().find_map(|scope| scope.get(name))
}

fn visible_names(scopes: &[Scope]) -> impl Iterator<Item = &str> {
    scopes
        .iter()
        .rev()
        .flat_map(|scope| scope.keys().map(String::as_str))
}
