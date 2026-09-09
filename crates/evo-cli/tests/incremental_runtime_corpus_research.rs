include!("incremental_runtime_quality_research.rs");

const CORPUS_WARMUPS: usize = 3;
const CORPUS_SAMPLES: usize = 21;
const CORPUS_CASES: [&str; 7] = [
    "runtime-repeat-v0",
    "control-flow-branch-v0",
    "logical-operators-v0",
    "function-call-v0",
    "block-locals-v0",
    "records-v0",
    "enums-v0",
];

struct CorpusSampleSinks<'a> {
    reference: &'a mut Vec<Duration>,
    current: &'a mut Vec<Duration>,
    candidate: &'a mut Vec<Duration>,
}

struct CorpusCaseResult {
    name: String,
    reference: Vec<Duration>,
    current: Vec<Duration>,
    candidate: Vec<Duration>,
    reference_bytes: u64,
    current_bytes: u64,
    candidate_bytes: u64,
    current_equals_reference: bool,
    candidate_equals_reference: bool,
    candidate_equals_current: bool,
    generated_source_equals_reference: bool,
    state_files: u64,
    state_bytes: u64,
}

fn corpus_order(index: usize) -> [u8; 3] {
    const ORDERS: [[u8; 3]; 6] = [
        [0, 1, 2],
        [2, 1, 0],
        [1, 2, 0],
        [0, 2, 1],
        [2, 0, 1],
        [1, 0, 2],
    ];
    ORDERS[index % ORDERS.len()]
}

fn run_corpus_round(
    index: usize,
    stdin: &[u8],
    reference_binary: &Path,
    current_binary: &Path,
    candidate_binary: &Path,
    mut samples: Option<CorpusSampleSinks<'_>>,
) {
    for arm in corpus_order(index) {
        let binary = match arm {
            0 => reference_binary,
            1 => current_binary,
            2 => candidate_binary,
            _ => unreachable!("corpus runtime arm must be known"),
        };
        let elapsed = timed_runtime(binary, stdin);
        if let Some(values) = samples.as_mut() {
            match arm {
                0 => values.reference.push(elapsed),
                1 => values.current.push(elapsed),
                2 => values.candidate.push(elapsed),
                _ => unreachable!("corpus runtime arm must be known"),
            }
        }
    }
}

fn corpus_case_verdict(result: &CorpusCaseResult) -> &'static str {
    let reference = runtime_quality_stats(&result.reference);
    let current = runtime_quality_stats(&result.current);
    let candidate = runtime_quality_stats(&result.candidate);
    let stable = reference
        .relative_mad
        .max(current.relative_mad)
        .max(candidate.relative_mad)
        <= RUNTIME_MAX_RELATIVE_MAD;

    if result.candidate_equals_reference && result.candidate_equals_current {
        "PASS-BINARY-PARITY"
    } else if !stable {
        "INCONCLUSIVE"
    } else if candidate.median_ms / reference.median_ms <= 1.0
        && candidate.median_ms / current.median_ms <= 1.0
    {
        "PASS"
    } else {
        "FAIL"
    }
}

fn corpus_decision(results: &[CorpusCaseResult]) -> &'static str {
    if results
        .iter()
        .any(|result| corpus_case_verdict(result) == "FAIL")
    {
        "REJECT"
    } else if results
        .iter()
        .any(|result| corpus_case_verdict(result) == "INCONCLUSIVE")
    {
        "INCONCLUSIVE"
    } else {
        "RUNTIME-GATE-PASS"
    }
}

fn json_bool(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn write_corpus_report(out_dir: &Path, results: &[CorpusCaseResult], rustc_version: &str) {
    fs::create_dir_all(out_dir).expect("runtime-corpus output directory should be created");
    let git_sha = env::var("EVO_GIT_SHA")
        .or_else(|_| env::var("GITHUB_SHA"))
        .unwrap_or_else(|_| "local".to_owned());
    let host = rustc_host(rustc_version);
    let decision = corpus_decision(results);

    let mut markdown = format!(
        "# Incremental runtime corpus research v0\n\n\
- git_sha: `{git_sha}`\n\
- platform: `{}-{}`\n\
- rustc_host: `{host}`\n\
- cases: `{}`\n\
- warmups_per_arm: `{CORPUS_WARMUPS}`\n\
- samples_per_arm: `{CORPUS_SAMPLES}`\n\
- current_flags: `--edition={RUST_EDITION} --error-format=short -C opt-level={RUST_OPT_LEVEL} -C codegen-units={RUST_CODEGEN_UNITS}`\n\
- candidate_flags: `--edition={RUST_EDITION} --error-format=short -C opt-level={RUST_OPT_LEVEL} -C codegen-units={RUNTIME_CANDIDATE_CGU} -C incremental=<per-case-session>`\n\
- correctness: `PASS`\n\
- aggregate_decision: `{decision}`\n\n\
| Case | Ref median ms | Current median ms | Candidate median ms | Current/ref | Candidate/ref | Candidate/current | Max rel MAD | Candidate bytes | Verdict |\n\
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |\n",
        env::consts::OS,
        env::consts::ARCH,
        results.len(),
    );

    for result in results {
        let reference = runtime_quality_stats(&result.reference);
        let current = runtime_quality_stats(&result.current);
        let candidate = runtime_quality_stats(&result.candidate);
        let current_to_reference = current.median_ms / reference.median_ms;
        let candidate_to_reference = candidate.median_ms / reference.median_ms;
        let candidate_to_current = candidate.median_ms / current.median_ms;
        let max_relative_mad = reference
            .relative_mad
            .max(current.relative_mad)
            .max(candidate.relative_mad);
        markdown.push_str(&format!(
            "| {} | {:.3} | {:.3} | {:.3} | {:.6} | {:.6} | {:.6} | {:.6} | {} | {} |\n",
            result.name,
            reference.median_ms,
            current.median_ms,
            candidate.median_ms,
            current_to_reference,
            candidate_to_reference,
            candidate_to_current,
            max_relative_mad,
            result.candidate_bytes,
            corpus_case_verdict(result),
        ));
    }

    markdown.push_str(
        "\nEvery case uses the committed `reference.rs`, Evolution source, stdin and expected stdout. Reference and current Evolution generated Rust are staged through the same stable `main.rs` path and compiled with production-equivalent CGU1 flags. The candidate stages the same generated Rust and enables CGU256 plus a fresh per-case incremental directory. Every binary must emit the exact committed stdout before timing. Timing order rotates among reference/current/candidate. No performance assertion is embedded in the test: a stable candidate regression is retained as `FAIL` evidence and makes the aggregate research decision `REJECT`; noisy cases remain `INCONCLUSIVE`.\n",
    );
    fs::write(out_dir.join("report.md"), markdown)
        .expect("runtime-corpus Markdown report should be written");

    let mut json = format!(
        "{{\n  \"git_sha\": \"{git_sha}\",\n  \"platform\": \"{}-{}\",\n  \"rustc_host\": \"{host}\",\n  \"warmups_per_arm\": {CORPUS_WARMUPS},\n  \"samples_per_arm\": {CORPUS_SAMPLES},\n  \"aggregate_decision\": \"{decision}\",\n  \"correctness\": \"PASS\",\n  \"cases\": [\n",
        env::consts::OS,
        env::consts::ARCH,
    );

    for (index, result) in results.iter().enumerate() {
        let reference = runtime_quality_stats(&result.reference);
        let current = runtime_quality_stats(&result.current);
        let candidate = runtime_quality_stats(&result.candidate);
        let max_relative_mad = reference
            .relative_mad
            .max(current.relative_mad)
            .max(candidate.relative_mad);
        let comma = if index + 1 == results.len() { "" } else { "," };
        json.push_str(&format!(
            "    {{\n      \"name\": \"{}\",\n      \"verdict\": \"{}\",\n      \"reference_samples_ms\": [{}],\n      \"current_samples_ms\": [{}],\n      \"candidate_samples_ms\": [{}],\n      \"reference_median_ms\": {:.3},\n      \"current_median_ms\": {:.3},\n      \"candidate_median_ms\": {:.3},\n      \"current_to_reference_ratio\": {:.9},\n      \"candidate_to_reference_ratio\": {:.9},\n      \"candidate_to_current_ratio\": {:.9},\n      \"max_relative_mad\": {:.9},\n      \"reference_binary_bytes\": {},\n      \"current_binary_bytes\": {},\n      \"candidate_binary_bytes\": {},\n      \"current_equals_reference\": {},\n      \"candidate_equals_reference\": {},\n      \"candidate_equals_current\": {},\n      \"generated_source_equals_reference\": {},\n      \"candidate_state_files\": {},\n      \"candidate_state_bytes\": {}\n    }}{comma}\n",
            result.name,
            corpus_case_verdict(result),
            samples_json(&result.reference),
            samples_json(&result.current),
            samples_json(&result.candidate),
            reference.median_ms,
            current.median_ms,
            candidate.median_ms,
            current.median_ms / reference.median_ms,
            candidate.median_ms / reference.median_ms,
            candidate.median_ms / current.median_ms,
            max_relative_mad,
            result.reference_bytes,
            result.current_bytes,
            result.candidate_bytes,
            json_bool(result.current_equals_reference),
            json_bool(result.candidate_equals_reference),
            json_bool(result.candidate_equals_current),
            json_bool(result.generated_source_equals_reference),
            result.state_files,
            result.state_bytes,
        ));
    }
    json.push_str("  ]\n}\n");
    fs::write(out_dir.join("report.json"), json)
        .expect("runtime-corpus JSON report should be written");

    let mut csv = String::from("case,arm,index,elapsed_ms\n");
    for result in results {
        for (arm, samples) in [
            ("reference-cgu1", &result.reference),
            ("evolution-current-cgu1", &result.current),
            ("evolution-cgu256-incremental", &result.candidate),
        ] {
            for (index, sample) in samples.iter().enumerate() {
                writeln!(
                    csv,
                    "{},{arm},{},{:.3}",
                    result.name,
                    index + 1,
                    sample.as_secs_f64() * 1_000.0
                )
                .expect("writing runtime-corpus CSV to String should not fail");
            }
        }
    }
    fs::write(out_dir.join("raw-samples.csv"), csv)
        .expect("runtime-corpus CSV report should be written");
    fs::write(out_dir.join("rustc-vV.txt"), rustc_version)
        .expect("runtime-corpus rustc version evidence should be written");
}

#[test]
#[ignore = "run explicitly on Ubuntu after single-fixture runtime isolation"]
fn incremental_runtime_candidate_is_measured_across_committed_corpus() {
    let root = repo_root().join("benchmarks/cases");
    let dir = temp_dir("incremental-runtime-corpus");
    fs::create_dir_all(&dir).expect("runtime-corpus directory should be created");
    let rustc = real_rustc();
    let version = rustc_version(&rustc);
    let mut results = Vec::with_capacity(CORPUS_CASES.len());

    for case_name in CORPUS_CASES {
        let fixture = root.join(case_name);
        let evolution_source =
            fs::read(fixture.join("evolution.evo")).expect("corpus Evolution source should read");
        let reference_source =
            fs::read(fixture.join("reference.rs")).expect("corpus reference Rust should read");
        let fixture_stdin =
            fs::read(fixture.join("stdin.bin")).expect("corpus stdin should read");
        let expected_stdout =
            fs::read(fixture.join("expected.stdout")).expect("corpus expected stdout should read");

        let case_dir = dir.join(case_name);
        fs::create_dir_all(&case_dir).expect("corpus case directory should be created");
        let evo_path = case_dir.join("program.evo");
        fs::write(&evo_path, &evolution_source).expect("corpus Evolution source should stage");
        let generated = evo_command("emit-rust", &evo_path)
            .output()
            .expect("corpus emit-rust should execute");
        assert_success(&generated, "corpus emit-rust");

        let source = case_dir.join("main.rs");
        let reference_binary = case_dir.join(format!("reference{}", env::consts::EXE_SUFFIX));
        let current_binary = case_dir.join(format!("current{}", env::consts::EXE_SUFFIX));
        let candidate_binary = case_dir.join(format!("candidate{}", env::consts::EXE_SUFFIX));
        let session = case_dir.join("candidate-session");

        fs::write(&source, &reference_source).expect("corpus reference Rust should stage");
        compile_runtime_binary(
            &rustc,
            &source,
            &reference_binary,
            RUST_CODEGEN_UNITS,
            None,
            "corpus reference compile",
        );
        run_binary(&reference_binary, &fixture_stdin, &expected_stdout);

        fs::write(&source, &generated.stdout).expect("corpus generated Rust should stage");
        compile_runtime_binary(
            &rustc,
            &source,
            &current_binary,
            RUST_CODEGEN_UNITS,
            None,
            "corpus current compile",
        );
        run_binary(&current_binary, &fixture_stdin, &expected_stdout);

        compile_runtime_binary(
            &rustc,
            &source,
            &candidate_binary,
            RUNTIME_CANDIDATE_CGU,
            Some(&session),
            "corpus candidate incremental compile",
        );
        run_binary(&candidate_binary, &fixture_stdin, &expected_stdout);
        let state = incremental_dir_stats(&session);
        assert!(state.files > 0, "corpus candidate must persist incremental state");

        for index in 0..CORPUS_WARMUPS {
            run_corpus_round(
                index,
                &fixture_stdin,
                &reference_binary,
                &current_binary,
                &candidate_binary,
                None,
            );
        }

        let mut reference_samples = Vec::with_capacity(CORPUS_SAMPLES);
        let mut current_samples = Vec::with_capacity(CORPUS_SAMPLES);
        let mut candidate_samples = Vec::with_capacity(CORPUS_SAMPLES);
        for index in 0..CORPUS_SAMPLES {
            run_corpus_round(
                index + CORPUS_WARMUPS,
                &fixture_stdin,
                &reference_binary,
                &current_binary,
                &candidate_binary,
                Some(CorpusSampleSinks {
                    reference: &mut reference_samples,
                    current: &mut current_samples,
                    candidate: &mut candidate_samples,
                }),
            );
        }

        let reference_bytes =
            fs::read(&reference_binary).expect("corpus reference binary should read");
        let current_bytes = fs::read(&current_binary).expect("corpus current binary should read");
        let candidate_bytes =
            fs::read(&candidate_binary).expect("corpus candidate binary should read");

        results.push(CorpusCaseResult {
            name: case_name.to_owned(),
            reference: reference_samples,
            current: current_samples,
            candidate: candidate_samples,
            reference_bytes: reference_bytes.len() as u64,
            current_bytes: current_bytes.len() as u64,
            candidate_bytes: candidate_bytes.len() as u64,
            current_equals_reference: current_bytes == reference_bytes,
            candidate_equals_reference: candidate_bytes == reference_bytes,
            candidate_equals_current: candidate_bytes == current_bytes,
            generated_source_equals_reference: generated.stdout == reference_source,
            state_files: state.files,
            state_bytes: state.bytes,
        });
    }

    let out_root = env::var_os("EVO_INCREMENTAL_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| dir.join("incremental-build-research-report"));
    let out_dir = out_root.join("runtime-corpus");
    write_corpus_report(&out_dir, &results, &version);

    println!("runtime_corpus_decision={}", corpus_decision(&results));
    for result in &results {
        let reference = runtime_quality_stats(&result.reference);
        let current = runtime_quality_stats(&result.current);
        let candidate = runtime_quality_stats(&result.candidate);
        println!(
            "runtime_corpus_case={} verdict={} reference_ms={:.3} current_ms={:.3} candidate_ms={:.3} candidate_to_reference={:.6} candidate_to_current={:.6}",
            result.name,
            corpus_case_verdict(result),
            reference.median_ms,
            current.median_ms,
            candidate.median_ms,
            candidate.median_ms / reference.median_ms,
            candidate.median_ms / current.median_ms,
        );
    }

    let _ = fs::remove_dir_all(dir);
}
