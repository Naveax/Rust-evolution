include!("incremental_build_research.rs");

const GRANULAR_CODEGEN_UNITS: &str = "256";

struct GranularityReportInput<'a> {
    control_prime: &'a [Duration],
    control_edit: &'a [Duration],
    incremental_prime: &'a [Duration],
    incremental_edit: &'a [Duration],
    state_after_prime: &'a [IncrementalDirStats],
    state_after_edit: &'a [IncrementalDirStats],
    rustc_version: &'a str,
}

fn granular_rustc_command(
    wrapper: &Path,
    rustc: &OsStr,
    counter: &Path,
    generated: &Path,
    output: &Path,
    incremental: Option<&Path>,
) -> Command {
    let mut command = Command::new(wrapper);
    command
        .arg(generated)
        .arg(format!("--edition={RUST_EDITION}"))
        .arg("--error-format=short")
        .arg("-C")
        .arg(format!("opt-level={RUST_OPT_LEVEL}"))
        .arg("-C")
        .arg(format!("codegen-units={GRANULAR_CODEGEN_UNITS}"));
    if let Some(session) = incremental {
        command
            .arg("-C")
            .arg(format!("incremental={}", session.display()));
    }
    command
        .arg("-o")
        .arg(output)
        .env("EVO_TEST_REAL_RUSTC", rustc)
        .env("EVO_TEST_RUSTC_COUNT", counter);
    command
}

fn push_granularity_csv_rows(
    csv: &mut String,
    arm: &str,
    phase: &str,
    samples: &[Duration],
    states: Option<&[IncrementalDirStats]>,
) {
    for (index, sample) in samples.iter().enumerate() {
        let state = states.map_or(IncrementalDirStats::default(), |values| values[index]);
        writeln!(
            csv,
            "{arm},{phase},{},{:.3},1,{},{}",
            index + 1,
            sample.as_secs_f64() * 1_000.0,
            state.files,
            state.bytes
        )
        .expect("writing CGU research CSV to String should not fail");
    }
}

fn write_granularity_report(out_dir: &Path, input: &GranularityReportInput<'_>) {
    fs::create_dir_all(out_dir).expect("CGU research output directory should be created");

    let control_prime = stats(input.control_prime);
    let control_edit = stats(input.control_edit);
    let incremental_prime = stats(input.incremental_prime);
    let incremental_edit = stats(input.incremental_edit);
    let control_to_incremental = control_edit.median_ms / incremental_edit.median_ms;
    let incremental_prime_to_edit = incremental_prime.median_ms / incremental_edit.median_ms;
    let git_sha = env::var("EVO_GIT_SHA")
        .or_else(|_| env::var("GITHUB_SHA"))
        .unwrap_or_else(|_| "local".to_owned());
    let host = rustc_host(input.rustc_version);
    let measured_rustc_invocations = RESEARCH_SAMPLES * 4;

    let markdown = format!(
        r#"# Incremental codegen-unit granularity research v0

- git_sha: `{git_sha}`
- platform: `{}-{}`
- rustc_host: `{host}`
- fixture: `benchmarks/cases/enums-v0/evolution.evo`
- samples_per_phase: `{RESEARCH_SAMPLES}`
- codegen_units: `{GRANULAR_CODEGEN_UNITS}`
- rustc_flags: `--edition={RUST_EDITION} --error-format=short -C opt-level={RUST_OPT_LEVEL} -C codegen-units={GRANULAR_CODEGEN_UNITS}`
- incremental_delta: `-C incremental=<per-sample-session-dir>`
- measured_rustc_invocations: `{measured_rustc_invocations}`
- correctness: `PASS`

| Arm | Prime median/min/max/p95 ms | Edited median/min/max/p95 ms |
| --- | ---: | ---: |
| stable path, direct rustc, CGU 256, no incremental | {:.3}/{:.3}/{:.3}/{:.3} | {:.3}/{:.3}/{:.3}/{:.3} |
| stable path, direct rustc, CGU 256, persistent incremental | {:.3}/{:.3}/{:.3}/{:.3} | {:.3}/{:.3}/{:.3}/{:.3} |

- matching-control -> incremental edited speedup: `{control_to_incremental:.3}x`
- incremental prime -> incremental edited speedup: `{incremental_prime_to_edit:.3}x`

This slice changes only codegen-unit granularity in the research harness. Both compared arms use the same `codegen-units=256`; the only difference between them is persistent incremental state. Every measured build invokes rustc exactly once and every produced binary must emit the committed expected stdout. No production compiler, cache, language, codegen, or runtime behavior is changed. Runtime-quality implications of CGU 256 are intentionally outside this timing-only slice and would require separate gates before any production proposal.
"#,
        env::consts::OS,
        env::consts::ARCH,
        control_prime.median_ms,
        control_prime.min_ms,
        control_prime.max_ms,
        control_prime.p95_ms,
        control_edit.median_ms,
        control_edit.min_ms,
        control_edit.max_ms,
        control_edit.p95_ms,
        incremental_prime.median_ms,
        incremental_prime.min_ms,
        incremental_prime.max_ms,
        incremental_prime.p95_ms,
        incremental_edit.median_ms,
        incremental_edit.min_ms,
        incremental_edit.max_ms,
        incremental_edit.p95_ms
    );
    fs::write(out_dir.join("report.md"), markdown).expect("CGU Markdown report should be written");

    let json = format!(
        r#"{{
  "git_sha": "{git_sha}",
  "platform": "{}-{}",
  "rustc_host": "{host}",
  "samples_per_phase": {RESEARCH_SAMPLES},
  "codegen_units": {GRANULAR_CODEGEN_UNITS},
  "rustc_compile_delta_per_measured_build": 1,
  "measured_rustc_invocations": {measured_rustc_invocations},
  "correctness": "PASS",
  "control_prime_samples_ms": [{}],
  "control_edit_samples_ms": [{}],
  "incremental_prime_samples_ms": [{}],
  "incremental_edit_samples_ms": [{}],
  "control_edit_median_ms": {:.3},
  "incremental_edit_median_ms": {:.3},
  "control_to_incremental_speedup": {:.6},
  "incremental_prime_to_edit_speedup": {:.6},
  "incremental_state_after_prime_files": [{}],
  "incremental_state_after_prime_bytes": [{}],
  "incremental_state_after_edit_files": [{}],
  "incremental_state_after_edit_bytes": [{}]
}}
"#,
        env::consts::OS,
        env::consts::ARCH,
        samples_json(input.control_prime),
        samples_json(input.control_edit),
        samples_json(input.incremental_prime),
        samples_json(input.incremental_edit),
        control_edit.median_ms,
        incremental_edit.median_ms,
        control_to_incremental,
        incremental_prime_to_edit,
        state_values_json(input.state_after_prime, |state| state.files),
        state_values_json(input.state_after_prime, |state| state.bytes),
        state_values_json(input.state_after_edit, |state| state.files),
        state_values_json(input.state_after_edit, |state| state.bytes)
    );
    fs::write(out_dir.join("report.json"), json).expect("CGU JSON report should be written");

    let mut csv =
        String::from("arm,phase,index,elapsed_ms,rustc_compile_delta,state_files,state_bytes\n");
    push_granularity_csv_rows(
        &mut csv,
        "stable-cgu256-no-incremental",
        "prime",
        input.control_prime,
        None,
    );
    push_granularity_csv_rows(
        &mut csv,
        "stable-cgu256-no-incremental",
        "edit",
        input.control_edit,
        None,
    );
    push_granularity_csv_rows(
        &mut csv,
        "stable-cgu256-incremental",
        "prime",
        input.incremental_prime,
        Some(input.state_after_prime),
    );
    push_granularity_csv_rows(
        &mut csv,
        "stable-cgu256-incremental",
        "edit",
        input.incremental_edit,
        Some(input.state_after_edit),
    );
    fs::write(out_dir.join("raw-samples.csv"), csv).expect("CGU CSV report should be written");
    fs::write(out_dir.join("rustc-vV.txt"), input.rustc_version)
        .expect("CGU rustc version evidence should be written");
}

#[test]
#[ignore = "run explicitly after the single-CGU research slice on Ubuntu"]
fn incremental_codegen_unit_granularity_compares_matching_256_cgu_control() {
    let dir = temp_dir("incremental-cgu256-evidence");
    fs::create_dir_all(&dir).expect("CGU research directory should be created");

    let fixture = fixture_dir();
    let fixture_source =
        fs::read(fixture.join("evolution.evo")).expect("fixture source should read");
    let fixture_stdin = fs::read(fixture.join("stdin.bin")).expect("fixture stdin should read");
    let expected_stdout =
        fs::read(fixture.join("expected.stdout")).expect("fixture expected stdout should read");
    let baseline_source =
        String::from_utf8(fixture_source).expect("fixture source should be UTF-8");
    let edited_source = baseline_source.replacen(EDIT_FROM, EDIT_TO, 1);
    assert_ne!(baseline_source, edited_source);

    let source = dir.join("emission.evo");
    fs::write(&source, &baseline_source).expect("baseline source should write");
    let baseline_generated = evo_command("emit-rust", &source)
        .output()
        .expect("baseline emit-rust should execute");
    assert_success(&baseline_generated, "baseline emit-rust");
    fs::write(&source, &edited_source).expect("edited source should write");
    let edited_generated = evo_command("emit-rust", &source)
        .output()
        .expect("edited emit-rust should execute");
    assert_success(&edited_generated, "edited emit-rust");
    assert_ne!(baseline_generated.stdout, edited_generated.stdout);

    let counter = dir.join("rustc-count.txt");
    fs::write(&counter, "0").expect("counter should initialize");
    let rustc = real_rustc();
    let wrapper = compile_rustc_wrapper(&dir, &rustc);
    let version = rustc_version(&rustc);

    let mut control_prime = Vec::with_capacity(RESEARCH_SAMPLES);
    let mut control_edit = Vec::with_capacity(RESEARCH_SAMPLES);
    for index in 0..RESEARCH_SAMPLES {
        let sample_dir = dir.join(format!("control-{index}"));
        fs::create_dir_all(&sample_dir).expect("control sample directory should be created");
        let generated = sample_dir.join("main.rs");
        let output = sample_dir.join(format!("program{}", env::consts::EXE_SUFFIX));
        fs::write(&generated, &baseline_generated.stdout)
            .expect("control baseline generated Rust should write");

        let before = compile_count(&counter);
        let (prime, elapsed) = timed_output(&mut granular_rustc_command(
            &wrapper, &rustc, &counter, &generated, &output, None,
        ));
        assert_success(&prime, "CGU 256 control prime");
        assert_eq!(compile_count(&counter) - before, 1);
        run_binary(&output, &fixture_stdin, &expected_stdout);
        control_prime.push(elapsed);

        fs::write(&generated, &edited_generated.stdout)
            .expect("control edited generated Rust should write");
        let before = compile_count(&counter);
        let (edit, elapsed) = timed_output(&mut granular_rustc_command(
            &wrapper, &rustc, &counter, &generated, &output, None,
        ));
        assert_success(&edit, "CGU 256 control edit");
        assert_eq!(compile_count(&counter) - before, 1);
        run_binary(&output, &fixture_stdin, &expected_stdout);
        control_edit.push(elapsed);
    }

    let mut incremental_prime = Vec::with_capacity(RESEARCH_SAMPLES);
    let mut incremental_edit = Vec::with_capacity(RESEARCH_SAMPLES);
    let mut state_after_prime = Vec::with_capacity(RESEARCH_SAMPLES);
    let mut state_after_edit = Vec::with_capacity(RESEARCH_SAMPLES);
    for index in 0..RESEARCH_SAMPLES {
        let sample_dir = dir.join(format!("incremental-{index}"));
        fs::create_dir_all(&sample_dir).expect("incremental sample directory should be created");
        let generated = sample_dir.join("main.rs");
        let output = sample_dir.join(format!("program{}", env::consts::EXE_SUFFIX));
        let session = sample_dir.join("session");
        fs::write(&generated, &baseline_generated.stdout)
            .expect("incremental baseline generated Rust should write");

        let before = compile_count(&counter);
        let (prime, elapsed) = timed_output(&mut granular_rustc_command(
            &wrapper,
            &rustc,
            &counter,
            &generated,
            &output,
            Some(&session),
        ));
        assert_success(&prime, "CGU 256 incremental prime");
        assert_eq!(compile_count(&counter) - before, 1);
        run_binary(&output, &fixture_stdin, &expected_stdout);
        let state = incremental_dir_stats(&session);
        assert!(state.files > 0, "incremental state should be persisted");
        incremental_prime.push(elapsed);
        state_after_prime.push(state);

        fs::write(&generated, &edited_generated.stdout)
            .expect("incremental edited generated Rust should write");
        let before = compile_count(&counter);
        let (edit, elapsed) = timed_output(&mut granular_rustc_command(
            &wrapper,
            &rustc,
            &counter,
            &generated,
            &output,
            Some(&session),
        ));
        assert_success(&edit, "CGU 256 incremental edit");
        assert_eq!(compile_count(&counter) - before, 1);
        run_binary(&output, &fixture_stdin, &expected_stdout);
        incremental_edit.push(elapsed);
        state_after_edit.push(incremental_dir_stats(&session));
    }

    let root = env::var_os("EVO_INCREMENTAL_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| dir.join("incremental-build-research-report"));
    let out_dir = root.join("codegen-units-256");
    write_granularity_report(
        &out_dir,
        &GranularityReportInput {
            control_prime: &control_prime,
            control_edit: &control_edit,
            incremental_prime: &incremental_prime,
            incremental_edit: &incremental_edit,
            state_after_prime: &state_after_prime,
            state_after_edit: &state_after_edit,
            rustc_version: &version,
        },
    );

    let control = stats(&control_edit);
    let incremental = stats(&incremental_edit);
    println!("cgu256_control_edit_ms={:.3}", control.median_ms);
    println!("cgu256_incremental_edit_ms={:.3}", incremental.median_ms);
    println!(
        "cgu256_control_to_incremental_speedup={:.3}",
        control.median_ms / incremental.median_ms
    );

    let _ = fs::remove_dir_all(dir);
}
