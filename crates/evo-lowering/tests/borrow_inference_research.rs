use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::{Expr, ExprKind, FunctionDef, Program, Stmt, StmtKind, TypeName};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UseMode {
    Inspect,
    Consume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Classification {
    SafeLocalInferenceCandidate,
    KeepByValue,
}

impl Classification {
    const fn as_str(self) -> &'static str {
        match self {
            Self::SafeLocalInferenceCandidate => "SAFE-LOCAL-INFERENCE-CANDIDATE",
            Self::KeepByValue => "KEEP-BY-VALUE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectedLowering {
    Pass,
    RejectMoved,
}

#[derive(Debug, Default)]
struct Effects {
    inspect_uses: usize,
    consume_uses: usize,
    reinitialized: bool,
}

#[derive(Debug)]
struct CaseSpec {
    name: &'static str,
    file: &'static str,
    function: &'static str,
    parameter: &'static str,
    expected_classification: Classification,
    expected_lowering: ExpectedLowering,
    rust_sketch: &'static str,
}

#[derive(Debug)]
struct Finding {
    name: &'static str,
    file: &'static str,
    function: &'static str,
    parameter: &'static str,
    classification: Classification,
    inspect_uses: usize,
    consume_uses: usize,
    reinitialized: bool,
    current_lowering: &'static str,
    rust_sketch: &'static str,
}

const CASES: &[CaseSpec] = &[
    CaseSpec {
        name: "direct-field-read",
        file: "direct_field_read.evo",
        function: "read_value",
        parameter: "item",
        expected_classification: Classification::SafeLocalInferenceCandidate,
        expected_lowering: ExpectedLowering::Pass,
        rust_sketch: "fn read_value(item: &Item) -> i64 { item.value }",
    },
    CaseSpec {
        name: "double-read-current-friction",
        file: "double_read_friction.evo",
        function: "read_value",
        parameter: "item",
        expected_classification: Classification::SafeLocalInferenceCandidate,
        expected_lowering: ExpectedLowering::RejectMoved,
        rust_sketch: "let first = read_value(&item); let second = read_value(&item);",
    },
    CaseSpec {
        name: "shared-read-branches",
        file: "branch_read.evo",
        function: "choose_value",
        parameter: "item",
        expected_classification: Classification::SafeLocalInferenceCandidate,
        expected_lowering: ExpectedLowering::Pass,
        rust_sketch: "fn choose_value(item: &Item, flag: bool) -> i64 { if flag { item.value } else { item.value + 1 } }",
    },
    CaseSpec {
        name: "nested-record-read",
        file: "nested_record_read.evo",
        function: "nested_value",
        parameter: "outer",
        expected_classification: Classification::SafeLocalInferenceCandidate,
        expected_lowering: ExpectedLowering::Pass,
        rust_sketch: "fn nested_value(outer: &Outer) -> i64 { outer.inner.value }",
    },
    CaseSpec {
        name: "repeat-read-only",
        file: "repeat_read_only.evo",
        function: "sum_reads",
        parameter: "item",
        expected_classification: Classification::SafeLocalInferenceCandidate,
        expected_lowering: ExpectedLowering::Pass,
        rust_sketch: "fn sum_reads(item: &Item, n: i64) -> i64 { /* repeated reads of item.value */ }",
    },
    CaseSpec {
        name: "read-then-move-current-friction",
        file: "read_then_move_friction.evo",
        function: "read_value",
        parameter: "item",
        expected_classification: Classification::SafeLocalInferenceCandidate,
        expected_lowering: ExpectedLowering::RejectMoved,
        rust_sketch: "let first = read_value(&item); let moved = item;",
    },
    CaseSpec {
        name: "inspect-then-owned-return",
        file: "inspect_then_return_owned.evo",
        function: "inspect_then_return",
        parameter: "item",
        expected_classification: Classification::KeepByValue,
        expected_lowering: ExpectedLowering::Pass,
        rust_sketch: "fn inspect_then_return(item: Item) -> Item { /* ownership must remain by value */ }",
    },
    CaseSpec {
        name: "forward-through-current-consuming-call",
        file: "forwarding_call.evo",
        function: "forward",
        parameter: "item",
        expected_classification: Classification::KeepByValue,
        expected_lowering: ExpectedLowering::Pass,
        rust_sketch: "fn forward(item: Item) -> i64 { read_value(item) }",
    },
    CaseSpec {
        name: "owned-match-scrutinee",
        file: "owned_match.evo",
        function: "inspect_step",
        parameter: "step",
        expected_classification: Classification::KeepByValue,
        expected_lowering: ExpectedLowering::Pass,
        rust_sketch: "fn inspect_step(step: Step) -> i64 { /* current match owns the scrutinee */ }",
    },
    CaseSpec {
        name: "parameter-reinitialization",
        file: "reinitialize_param.evo",
        function: "replace_value",
        parameter: "item",
        expected_classification: Classification::KeepByValue,
        expected_lowering: ExpectedLowering::Pass,
        rust_sketch: "fn replace_value(mut item: Item) -> i64 { item = Item { value: 9 }; item.value }",
    },
    CaseSpec {
        name: "repeat-read-then-owned-return",
        file: "repeat_then_return_owned.evo",
        function: "repeat_then_return",
        parameter: "item",
        expected_classification: Classification::KeepByValue,
        expected_lowering: ExpectedLowering::Pass,
        rust_sketch: "fn repeat_then_return(item: Item, n: i64) -> Item { /* inspect, then move */ }",
    },
];

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("borrow_inference_cases")
}

fn parse_fixture(path: &Path) -> Program {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let tokens =
        lex(&source).unwrap_or_else(|error| panic!("failed to lex {}: {error}", path.display()));
    evo_parser::parse(&tokens)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}

fn find_function<'a>(program: &'a Program, spec: &CaseSpec) -> &'a FunctionDef {
    program
        .functions
        .iter()
        .find(|function| function.name == spec.function)
        .unwrap_or_else(|| {
            panic!(
                "fixture {} is missing target function {:?}",
                spec.file, spec.function
            )
        })
}

fn parameter_is_nominal(program: &Program, function: &FunctionDef, parameter: &str) -> bool {
    let Some(parameter) = function
        .parameters
        .iter()
        .find(|candidate| candidate.name == parameter)
    else {
        return false;
    };
    let TypeName::Named(type_name) = &parameter.type_name else {
        return false;
    };
    program
        .records
        .iter()
        .any(|record| record.name == *type_name)
        || program
            .enums
            .iter()
            .any(|enum_def| enum_def.name == *type_name)
}

fn collect_statement_effects(statements: &[Stmt], parameter: &str, effects: &mut Effects) {
    for statement in statements {
        match &statement.kind {
            StmtKind::Bind { name, expr } => {
                if name == parameter {
                    effects.reinitialized = true;
                }
                collect_expr_effects(expr, parameter, UseMode::Consume, effects);
            }
            StmtKind::Print(expr) => {
                collect_expr_effects(expr, parameter, UseMode::Inspect, effects);
            }
            StmtKind::Return(expr) => {
                collect_expr_effects(expr, parameter, UseMode::Consume, effects);
            }
            StmtKind::Repeat { count, body } => {
                collect_expr_effects(count, parameter, UseMode::Inspect, effects);
                collect_statement_effects(body, parameter, effects);
            }
            StmtKind::If {
                condition,
                then_body,
                else_body,
            } => {
                collect_expr_effects(condition, parameter, UseMode::Inspect, effects);
                collect_statement_effects(then_body, parameter, effects);
                collect_statement_effects(else_body, parameter, effects);
            }
            StmtKind::Match { value, arms } => {
                collect_expr_effects(value, parameter, UseMode::Consume, effects);
                for arm in arms {
                    collect_statement_effects(&arm.body, parameter, effects);
                }
            }
        }
    }
}

fn collect_expr_effects(expr: &Expr, parameter: &str, mode: UseMode, effects: &mut Effects) {
    match &expr.kind {
        ExprKind::Integer(_) | ExprKind::String(_) | ExprKind::Bool(_) | ExprKind::InputInt => {}
        ExprKind::Identifier(name) => {
            if name == parameter {
                match mode {
                    UseMode::Inspect => effects.inspect_uses += 1,
                    UseMode::Consume => effects.consume_uses += 1,
                }
            }
        }
        ExprKind::Call { arguments, .. } => {
            for argument in arguments {
                collect_expr_effects(argument, parameter, UseMode::Consume, effects);
            }
        }
        ExprKind::Construct { fields, .. } => {
            for field in fields {
                collect_expr_effects(&field.value, parameter, UseMode::Consume, effects);
            }
        }
        ExprKind::EnumConstruct { arguments, .. } => {
            for argument in arguments {
                collect_expr_effects(argument, parameter, UseMode::Consume, effects);
            }
        }
        ExprKind::FieldAccess { base, .. } => {
            collect_expr_effects(base, parameter, UseMode::Inspect, effects);
        }
        ExprKind::LogicalNot(inner) | ExprKind::UnaryMinus(inner) => {
            collect_expr_effects(inner, parameter, UseMode::Consume, effects);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_expr_effects(left, parameter, UseMode::Consume, effects);
            collect_expr_effects(right, parameter, UseMode::Consume, effects);
        }
    }
}

fn classify(effects: &Effects) -> Classification {
    if !effects.reinitialized && effects.consume_uses == 0 && effects.inspect_uses > 0 {
        Classification::SafeLocalInferenceCandidate
    } else {
        Classification::KeepByValue
    }
}

fn validate_current_lowering(program: &Program, expected: ExpectedLowering) -> &'static str {
    match (lower(program), expected) {
        (Ok(_), ExpectedLowering::Pass) => "PASS",
        (Err(error), ExpectedLowering::RejectMoved)
            if error.message.contains("use of moved record local") =>
        {
            "REJECTED-CURRENT-MOVE"
        }
        (Ok(_), ExpectedLowering::RejectMoved) => {
            panic!("fixture unexpectedly stopped demonstrating current move friction")
        }
        (Err(error), ExpectedLowering::Pass) => {
            panic!("fixture expected to lower successfully but failed: {error}")
        }
        (Err(error), ExpectedLowering::RejectMoved) => {
            panic!("fixture failed for an unexpected reason: {error}")
        }
    }
}

fn finding(spec: &'static CaseSpec) -> Finding {
    let path = fixture_dir().join(spec.file);
    let program = parse_fixture(&path);
    let function = find_function(&program, spec);
    assert!(
        parameter_is_nominal(&program, function, spec.parameter),
        "fixture {} target parameter {:?} must be nominal",
        spec.file,
        spec.parameter
    );

    let mut effects = Effects::default();
    collect_statement_effects(&function.body, spec.parameter, &mut effects);
    let classification = classify(&effects);
    assert_eq!(
        classification, spec.expected_classification,
        "fixture {} classification changed",
        spec.file
    );

    Finding {
        name: spec.name,
        file: spec.file,
        function: spec.function,
        parameter: spec.parameter,
        classification,
        inspect_uses: effects.inspect_uses,
        consume_uses: effects.consume_uses,
        reinitialized: effects.reinitialized,
        current_lowering: validate_current_lowering(&program, spec.expected_lowering),
        rust_sketch: spec.rust_sketch,
    }
}

fn json_string(value: &str) -> String {
    format!("{value:?}")
}

fn write_reports(findings: &[Finding], verdict: &str, out: &Path, git_sha: &str) {
    fs::create_dir_all(out)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", out.display()));

    let mut csv = String::from(
        "case,file,function,parameter,classification,inspect_uses,consume_uses,reinitialized,current_lowering\n",
    );
    for item in findings {
        writeln!(
            csv,
            "{},{},{},{},{},{},{},{},{}",
            item.name,
            item.file,
            item.function,
            item.parameter,
            item.classification.as_str(),
            item.inspect_uses,
            item.consume_uses,
            item.reinitialized,
            item.current_lowering
        )
        .expect("writing CSV to String cannot fail");
    }
    fs::write(out.join("raw-matrix.csv"), csv)
        .unwrap_or_else(|error| panic!("failed to write raw matrix: {error}"));

    let safe_count = findings
        .iter()
        .filter(|item| item.classification == Classification::SafeLocalInferenceCandidate)
        .count();
    let friction_count = findings
        .iter()
        .filter(|item| {
            item.classification == Classification::SafeLocalInferenceCandidate
                && item.current_lowering == "REJECTED-CURRENT-MOVE"
        })
        .count();

    let mut json = String::new();
    writeln!(json, "{{").expect("writing JSON to String cannot fail");
    writeln!(json, "  \"git_sha\": {},", json_string(git_sha))
        .expect("writing JSON to String cannot fail");
    writeln!(json, "  \"verdict\": {},", json_string(verdict))
        .expect("writing JSON to String cannot fail");
    writeln!(json, "  \"safe_candidate_count\": {safe_count},")
        .expect("writing JSON to String cannot fail");
    writeln!(json, "  \"current_friction_count\": {friction_count},")
        .expect("writing JSON to String cannot fail");
    writeln!(
        json,
        "  \"decision_gate\": \"IMPLEMENT-CANDIDATE requires >=2 local read-only nominal candidates, at least one demonstrated current move-friction case, and all consume/match/reinit negatives to fail closed\"," 
    )
    .expect("writing JSON to String cannot fail");
    writeln!(json, "  \"cases\": [").expect("writing JSON to String cannot fail");
    for (index, item) in findings.iter().enumerate() {
        let comma = if index + 1 == findings.len() { "" } else { "," };
        writeln!(
            json,
            "    {{\"name\": {}, \"file\": {}, \"function\": {}, \"parameter\": {}, \"classification\": {}, \"inspect_uses\": {}, \"consume_uses\": {}, \"reinitialized\": {}, \"current_lowering\": {}, \"rust_sketch\": {}}}{comma}",
            json_string(item.name),
            json_string(item.file),
            json_string(item.function),
            json_string(item.parameter),
            json_string(item.classification.as_str()),
            item.inspect_uses,
            item.consume_uses,
            item.reinitialized,
            json_string(item.current_lowering),
            json_string(item.rust_sketch),
        )
        .expect("writing JSON to String cannot fail");
    }
    writeln!(json, "  ],").expect("writing JSON to String cannot fail");
    writeln!(
        json,
        "  \"boundary_cases\": [\n    {{\"name\": \"borrowed-return-escape\", \"classification\": \"REQUIRES-LIFETIME-MODEL\", \"reason\": \"current Evolution syntax has no returned-reference surface; inferring an escaping borrow would require a caller-visible lifetime contract\"}},\n    {{\"name\": \"mutable-borrow-inference\", \"classification\": \"REQUIRES-EXPLICIT-BORROW-SYNTAX\", \"reason\": \"shared local inference cannot represent mutation or alias exclusivity\"}},\n    {{\"name\": \"borrow-overlap-move-conflict\", \"classification\": \"UNSAFE/AMBIGUOUS\", \"reason\": \"without an explicit stored-borrow/lifetime surface there is no sound basis to infer a borrow that outlives a single call\"}}\n  ]"
    )
    .expect("writing JSON to String cannot fail");
    writeln!(json, "}}").expect("writing JSON to String cannot fail");
    fs::write(out.join("report.json"), json)
        .unwrap_or_else(|error| panic!("failed to write report JSON: {error}"));

    let mut markdown = String::new();
    writeln!(markdown, "# Borrow inference feasibility v0").expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "- git_sha: `{git_sha}`").expect("writing Markdown cannot fail");
    writeln!(markdown, "- aggregate verdict: **{verdict}**").expect("writing Markdown cannot fail");
    writeln!(markdown, "- safe local candidates: **{safe_count}**")
        .expect("writing Markdown cannot fail");
    writeln!(
        markdown,
        "- demonstrated current move-friction cases: **{friction_count}**"
    )
    .expect("writing Markdown cannot fail");
    writeln!(
        markdown,
        "- rule: only direct nominal parameter uses that are all current `Inspect` contexts and never reinitialized qualify; current call boundaries, returns, owned matches and reinitialization fail closed."
    )
    .expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(
        markdown,
        "| Case | Classification | Inspect | Consume | Reinit | Current lowering |"
    )
    .expect("writing Markdown cannot fail");
    writeln!(markdown, "| --- | --- | ---: | ---: | --- | --- |")
        .expect("writing Markdown cannot fail");
    for item in findings {
        writeln!(
            markdown,
            "| `{}` | {} | {} | {} | {} | {} |",
            item.name,
            item.classification.as_str(),
            item.inspect_uses,
            item.consume_uses,
            item.reinitialized,
            item.current_lowering
        )
        .expect("writing Markdown cannot fail");
    }
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "## Safe-candidate Rust lowering sketches")
        .expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    for item in findings
        .iter()
        .filter(|item| item.classification == Classification::SafeLocalInferenceCandidate)
    {
        writeln!(markdown, "- `{}`: `{}`", item.name, item.rust_sketch)
            .expect("writing Markdown cannot fail");
    }
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "## Explicit boundaries").expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(
        markdown,
        "- `borrowed-return-escape`: **REQUIRES-LIFETIME-MODEL**. A reference escaping a call is deliberately outside local inference v0."
    )
    .expect("writing Markdown cannot fail");
    writeln!(
        markdown,
        "- `mutable-borrow-inference`: **REQUIRES-EXPLICIT-BORROW-SYNTAX**. Mutation needs an exclusivity contract, not silent shared-borrow inference."
    )
    .expect("writing Markdown cannot fail");
    writeln!(
        markdown,
        "- `borrow-overlap-move-conflict`: **UNSAFE/AMBIGUOUS**. The research permits only call-duration shared borrows; no stored/escaping inferred borrow is proposed."
    )
    .expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(
        markdown,
        "This report is research evidence only. It does not change Evolution ownership semantics or generated Rust."
    )
    .expect("writing Markdown cannot fail");
    fs::write(out.join("report.md"), &markdown)
        .unwrap_or_else(|error| panic!("failed to write report Markdown: {error}"));
    print!("{markdown}");
}

#[test]
#[ignore = "research evidence; dedicated workflow runs this exact test"]
fn borrow_inference_research_classifies_local_nominal_parameter_effects() {
    let findings: Vec<Finding> = CASES.iter().map(finding).collect();

    let safe_count = findings
        .iter()
        .filter(|item| item.classification == Classification::SafeLocalInferenceCandidate)
        .count();
    let friction_count = findings
        .iter()
        .filter(|item| {
            item.classification == Classification::SafeLocalInferenceCandidate
                && item.current_lowering == "REJECTED-CURRENT-MOVE"
        })
        .count();
    let negatives_fail_closed = findings
        .iter()
        .filter(|item| {
            matches!(
                item.name,
                "inspect-then-owned-return"
                    | "forward-through-current-consuming-call"
                    | "owned-match-scrutinee"
                    | "parameter-reinitialization"
                    | "repeat-read-then-owned-return"
            )
        })
        .all(|item| item.classification == Classification::KeepByValue);

    let verdict = if safe_count >= 2 && friction_count >= 1 && negatives_fail_closed {
        "IMPLEMENT-CANDIDATE"
    } else if safe_count > 0 {
        "EXPLICIT-SYNTAX-FIRST"
    } else {
        "DEFER"
    };

    assert!(
        safe_count >= 2,
        "pre-registered implementation gate needs at least two representative local candidates"
    );
    assert!(
        friction_count >= 1,
        "at least one candidate must demonstrate current move friction"
    );
    assert!(
        negatives_fail_closed,
        "consume/match/reinit negative controls must fail closed"
    );

    let git_sha = env::var("EVO_GIT_SHA").unwrap_or_else(|_| "local".to_owned());
    let out = env::var_os("EVO_BORROW_INFERENCE_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("evo-borrow-inference-research")
        });
    write_reports(&findings, verdict, &out, &git_sha);
}
