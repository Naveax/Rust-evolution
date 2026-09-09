include!("build_latency_baseline.rs");

use std::fmt::Write as _;

const RESEARCH_SAMPLES: usize = 7;
const ACCEPTED_76_EDIT_BASELINE_MS: f64 = 95.515;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct IncrementalDirStats {
    files: u64,
    bytes: u64,
}

struct ResearchReportInput<'a> {
    evo_prime: &'a [Duration],
    evo_edit: &'a [Duration],
    stable_prime: &'a [Duration],
    stable_edit: &'a [Duration],
    incremental_prime: &'a [Duration],
    incremental_edit: &'a [Duration],
    state_after_prime: &'a [IncrementalDirStats],
    state_after_edit: &'a [IncrementalDirStats],
    baseline_generated: &'a [u8],
    edited_generated: &'a [u8],
    baseline_source: &'a [u8],
    edited_source: &'a [u8],
    fixture_stdin: &'a [u8],
    expected_stdout: &'a [u8],
    rustc_version: &'a str,
    rustc_c_help: &'a str,
}

fn incremental_rustc_command(
    wrapper: &Path,
    rustc: &OsStr,
    counter: &Path,
    generated: &Path,
    output: &Path,
    session: &Path,
) -> Command {
    let mut command = Command::new(wrapper);
    command
        .arg(generated)
        .arg(format!("--edition={RUST_EDITION}"))
        .arg("--error-format=short")
        .arg("-C")
        .arg(format!("opt-level={RUST_OPT_LEVEL}"))
        .arg("-C")
        .arg(format!("codegen-units={RUST_CODEGEN_UNITS}"))
        .arg("-C")
        .arg(format!("incremental={}", session.display()))
        .arg("-o")
        .arg(output)
        .env("EVO_TEST_REAL_RUSTC", rustc)
        .env("EVO_TEST_RUSTC_COUNT", counter);
    command
}

fn rustc_codegen_help(rustc: &OsStr) -> String {
    let output = Command::new(rustc)
        .arg("-C")
        .arg("help")
        .output()
        .expect("rustc -C help should execute");
    assert_success(&output, "rustc -C help");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn incremental_dir_stats(path: &Path) -> IncrementalDirStats {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return IncrementalDirStats::default();
    };
    if metadata.file_type().is_symlink() {
        return IncrementalDirStats::default();
    }
    if metadata.is_file() {
        return IncrementalDirStats {
            files: 1,
            bytes: metadata.len(),
        };
    }
    if !metadata.is_dir() {
        return IncrementalDirStats::default();
    }

    let mut total = IncrementalDirStats::default();
    for entry in fs::read_dir(path).expect("incremental state directory should be readable") {
        let entry = entry.expect("incremental state entry should be readable");
        let child = incremental_dir_stats(&entry.path());
        total.files += child.files;
        total.bytes += child.bytes;
    }
    total
}

fn state_values_json(
    states: &[IncrementalDirStats],
    value: impl Fn(IncrementalDirStats) -> u64,
) -> String {
    states
        .iter()
        .copied()
        .map(value)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn push_research_csv_rows(
    csv: &mut String,
    arm: &str,
    phase: &str,
    samples: &[Duration],
    states: Option<&[IncrementalDirStats]>,
    notes: &str,
) {
    for (index, sample) in samples.iter().enumerate() {
        let state = states.map_or(IncrementalDirStats::default(), |values| values[index]);
        writeln!(
            csv,
            "{arm},{phase},{},{:.3},1,{},{},{notes}",
            index + 1,
            sample.as_secs_f64() * 1_000.0,
            state.files,
            state.bytes
        )
        .expect("writing research CSV to String should not fail");
    }
}

fn write_research_report(out_dir: &Path, input: &ResearchReportInput<'_>) {
    fs::create_dir_all(out_dir).expect("incremental research output directory should be created");

    let evo_prime = stats(input.evo_prime);
    let evo_edit = stats(input.evo_edit);
    let stable_prime = stats(input.stable_prime);
    let stable_edit = stats(input.stable_edit);
    let incremental_prime = stats(input.incremental_prime);
    let incremental_edit = stats(input.incremental_edit);
    let stable_to_incremental = stable_edit.median_ms / incremental_edit.median_ms;
    let evo_to_incremental = evo_edit.median_ms / incremental_edit.median_ms;
    let accepted_to_incremental = ACCEPTED_76_EDIT_BASELINE_MS / incremental_edit.median_ms;
    let git_sha = env::var("EVO_GIT_SHA")
        .or_else(|_| env::var("GITHUB_SHA"))
        .unwrap_or_else(|_| "local".to_owned());
    let host = rustc_host(input.rustc_version);
    let total_measured_rustc_invocations = RESEARCH_SAMPLES * 6;

    let markdown = format!(
        r#"# Changed-source incremental build research v0

- git_sha: `{git_sha}`
- platform: `{}-{}`
- rustc_host: `{host}`
- fixture: `benchmarks/cases/enums-v0/evolution.evo`
- samples_per_phase: `{RESEARCH_SAMPLES}`
- accepted_76_edit_baseline_ms: `{ACCEPTED_76_EDIT_BASELINE_MS:.3}`
- rustc_flags: `--edition={RUST_EDITION} --error-format=short -C opt-level={RUST_OPT_LEVEL} -C codegen-units={RUST_CODEGEN_UNITS}`
- incremental_delta: `-C incremental=<per-sample-session-dir>`
- measured_rustc_invocations: `{total_measured_rustc_invocations}`
- correctness: `PASS`

| Arm | Prime median/min/max/p95 ms | Edited median/min/max/p95 ms |
| --- | ---: | ---: |
| current `evo build --no-cache` fresh-workdir | {:.3}/{:.3}/{:.3}/{:.3} | {:.3}/{:.3}/{:.3}/{:.3} |
| stable generated path, direct rustc, no incremental | {:.3}/{:.3}/{:.3}/{:.3} | {:.3}/{:.3}/{:.3}/{:.3} |
| stable generated path, direct rustc, persistent incremental | {:.3}/{:.3}/{:.3}/{:.3} | {:.3}/{:.3}/{:.3}/{:.3} |

- stable-no-incremental -> incremental edited speedup: `{stable_to_incremental:.3}x`
- current-evo-edit -> incremental edited speedup: `{evo_to_incremental:.3}x`
- accepted-#76-edit -> incremental edited speedup: `{accepted_to_incremental:.3}x`

Every measured prime and edited build invokes rustc exactly once through the counting wrapper. Every produced native binary is executed with the committed fixture input and must emit the exact committed expected stdout. The incremental arm deliberately keeps `codegen-units=1`. Each sample receives a fresh incremental session directory, primes baseline generated Rust, then overwrites the same stable `main.rs` path with the edited generated Rust and reuses only that sample's compiler state. No production compiler, cache, language, codegen, or runtime behavior is changed by this research harness. Unfavorable samples remain in the raw evidence.
"#,
        env::consts::OS,
        env::consts::ARCH,
        evo_prime.median_ms,
        evo_prime.min_ms,
        evo_prime.max_ms,
        evo_prime.p95_ms,
        evo_edit.median_ms,
        evo_edit.min_ms,
        evo_edit.max_ms,
        evo_edit.p95_ms,
        stable_prime.median_ms,
        stable_prime.min_ms,
        stable_prime.max_ms,
        stable_prime.p95_ms,
        stable_edit.median_ms,
        stable_edit.min_ms,
        stable_edit.max_ms,
        stable_edit.p95_ms,
        incremental_prime.median_ms,
        incremental_prime.min_ms,
        incremental_prime.max_ms,
        incremental_prime.p95_ms,
        incremental_edit.median_ms,
        incremental_edit.min_ms,
        incremental_edit.max_ms,
        incremental_edit.p95_ms
    );
    fs::write(out_dir.join("report.md"), markdown).expect("Markdown report should be written");

    let json = format!(
        r#"{{
  "git_sha": "{git_sha}",
  "platform": "{}-{}",
  "rustc_host": "{host}",
  "fixture": "benchmarks/cases/enums-v0/evolution.evo",
  "samples_per_phase": {RESEARCH_SAMPLES},
  "accepted_76_edit_baseline_ms": {ACCEPTED_76_EDIT_BASELINE_MS:.3},
  "rustc_compile_delta_per_measured_build": 1,
  "measured_rustc_invocations": {total_measured_rustc_invocations},
  "correctness": "PASS",
  "evo_prime_samples_ms": [{}],
  "evo_edit_samples_ms": [{}],
  "stable_prime_samples_ms": [{}],
  "stable_edit_samples_ms": [{}],
  "incremental_prime_samples_ms": [{}],
  "incremental_edit_samples_ms": [{}],
  "evo_edit_median_ms": {:.3},
  "stable_edit_median_ms": {:.3},
  "incremental_edit_median_ms": {:.3},
  "stable_to_incremental_speedup": {:.6},
  "evo_to_incremental_speedup": {:.6},
  "accepted_76_to_incremental_speedup": {:.6},
  "incremental_state_after_prime_files": [{}],
  "incremental_state_after_prime_bytes": [{}],
  "incremental_state_after_edit_files": [{}],
  "incremental_state_after_edit_bytes": [{}]
}}
"#,
        env::consts::OS,
        env::consts::ARCH,
        samples_json(input.evo_prime),
        samples_json(input.evo_edit),
        samples_json(input.stable_prime),
        samples_json(input.stable_edit),
        samples_json(input.incremental_prime),
        samples_json(input.incremental_edit),
        evo_edit.median_ms,
        stable_edit.median_ms,
        incremental_edit.median_ms,
        stable_to_incremental,
        evo_to_incremental,
        accepted_to_incremental,
        state_values_json(input.state_after_prime, |state| state.files),
        state_values_json(input.state_after_prime, |state| state.bytes),
        state_values_json(input.state_after_edit, |state| state.files),
        state_values_json(input.state_after_edit, |state| state.bytes)
    );
    fs::write(out_dir.join("report.json"), json).expect("JSON report should be written");

    let mut csv = String::from(
        "arm,phase,index,elapsed_ms,rustc_compile_delta,state_files,state_bytes,notes\n",
    );
    push_research_csv_rows(
        &mut csv,
        "evo-fresh-workdir",
        "prime",
        input.evo_prime,
        None,
        "production-no-cache-path",
    );
    push_research_csv_rows(
        &mut csv,
        "evo-fresh-workdir",
        "edit",
        input.evo_edit,
        None,
        "generated-rust-changing-edit",
    );
    push_research_csv_rows(
        &mut csv,
        "stable-direct-no-incremental",
        "prime",
        input.stable_prime,
        None,
        "stable-main-rs",
    );
    push_research_csv_rows(
        &mut csv,
        "stable-direct-no-incremental",
        "edit",
        input.stable_edit,
        None,
        "stable-main-rs",
    );
    push_research_csv_rows(
        &mut csv,
        "stable-direct-incremental",
        "prime",
        input.incremental_prime,
        Some(input.state_after_prime),
        "fresh-session-stable-main-rs",
    );
    push_research_csv_rows(
        &mut csv,
        "stable-direct-incremental",
        "edit",
        input.incremental_edit,
        Some(input.state_after_edit),
        "reused-session-stable-main-rs",
    );
    fs::write(out_dir.join("raw-samples.csv"), csv).expect("CSV report should be written");

    fs::write(
        out_dir.join("baseline-generated.rs"),
        input.baseline_generated,
    )
    .expect("baseline generated Rust should be written");
    fs::write(out_dir.join("edited-generated.rs"), input.edited_generated)
        .expect("edited generated Rust should be written");
    fs::write(out_dir.join("baseline.evo"), input.baseline_source)
        .expect("baseline Evolution source should be written");
    fs::write(out_dir.join("edited.evo"), input.edited_source)
        .expect("edited Evolution source should be written");
    fs::write(out_dir.join("stdin.bin"), input.fixture_stdin)
        .expect("fixture stdin evidence should be written");
    fs::write(out_dir.join("expected.stdout"), input.expected_stdout)
        .expect("fixture expected output evidence should be written");
    fs::write(out_dir.join("rustc-vV.txt"), input.rustc_version)
        .expect("rustc version evidence should be written");
    fs::write(out_dir.join("rustc-C-help.txt"), input.rustc_c_help)
        .expect("rustc -C help evidence should be written");
}

#[test]
fn incremental_directory_stats_count_regular_files_recursively() {
    let dir = temp_dir("incremental-dir-stats");
    fs::create_dir_all(dir.join("nested")).expect("nested directory should be created");
    fs::write(dir.join("one"), b"abc").expect("first file should write");
    fs::write(dir.join("nested/two"), b"12345").expect("second file should write");
    assert_eq!(
        incremental_dir_stats(&dir),
        IncrementalDirStats { files: 2, bytes: 8 }
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
#[ignore = "run explicitly on the controlled Ubuntu CI evidence runner"]
fn incremental_build_research_isolates_stable_path_and_persistent_rustc_state() {
    let dir = temp_dir("incremental-evidence");
    fs::create_dir_all(&dir).expect("test directory should be created");

    let fixture = fixture_dir();
    let fixture_source =
        fs::read(fixture.join("evolution.evo")).expect("fixture source should read");
    let fixture_stdin = fs::read(fixture.join("stdin.bin")).expect("fixture stdin should read");
    let expected_stdout =
        fs::read(fixture.join("expected.stdout")).expect("fixture expected stdout should read");

    let baseline_source =
        String::from_utf8(fixture_source.clone()).expect("fixture should be UTF-8");
    assert!(baseline_source.matches(EDIT_FROM).count() >= 2);
    let edited_source = baseline_source.replacen(EDIT_FROM, EDIT_TO, 1);
    assert_ne!(baseline_source, edited_source);

    let emission_source = dir.join("emission.evo");
    fs::write(&emission_source, &baseline_source).expect("baseline source should write");
    let baseline_generated = evo_command("emit-rust", &emission_source)
        .output()
        .expect("baseline emit-rust should execute");
    assert_success(&baseline_generated, "baseline emit-rust");

    fs::write(&emission_source, &edited_source).expect("edited source should write");
    let edited_generated = evo_command("emit-rust", &emission_source)
        .output()
        .expect("edited emit-rust should execute");
    assert_success(&edited_generated, "edited emit-rust");
    assert_ne!(baseline_generated.stdout, edited_generated.stdout);

    let counter = dir.join("rustc-count.txt");
    fs::write(&counter, "0").expect("counter should be initialized");
    let rustc = real_rustc();
    let wrapper = compile_rustc_wrapper(&dir, &rustc);
    let rustc_version = rustc_version(&rustc);
    let rustc_c_help = rustc_codegen_help(&rustc);
    assert!(
        rustc_c_help
            .lines()
            .any(|line| line.contains("incremental")),
        "pinned rustc must advertise the incremental codegen option"
    );

    let mut evo_prime = Vec::with_capacity(RESEARCH_SAMPLES);
    let mut evo_edit = Vec::with_capacity(RESEARCH_SAMPLES);
    for index in 0..RESEARCH_SAMPLES {
        let sample_dir = dir.join(format!("evo-{index}"));
        fs::create_dir_all(&sample_dir).expect("evo sample directory should be created");
        let source = sample_dir.join("program.evo");
        let output = sample_dir.join(format!("program{}", env::consts::EXE_SUFFIX));
        fs::write(&source, &baseline_source).expect("evo baseline source should write");

        let before = compile_count(&counter);
        let (prime, elapsed) = timed_output(&mut build_command(
            &source, &output, &wrapper, &rustc, &counter,
        ));
        assert_success(&prime, "evo prime build");
        assert_eq!(compile_count(&counter) - before, 1);
        run_binary(&output, &fixture_stdin, &expected_stdout);
        evo_prime.push(elapsed);

        fs::write(&source, &edited_source).expect("evo edited source should write");
        let before = compile_count(&counter);
        let (edit, elapsed) = timed_output(&mut build_command(
            &source, &output, &wrapper, &rustc, &counter,
        ));
        assert_success(&edit, "evo edited build");
        assert_eq!(compile_count(&counter) - before, 1);
        run_binary(&output, &fixture_stdin, &expected_stdout);
        evo_edit.push(elapsed);
    }

    let mut stable_prime = Vec::with_capacity(RESEARCH_SAMPLES);
    let mut stable_edit = Vec::with_capacity(RESEARCH_SAMPLES);
    for index in 0..RESEARCH_SAMPLES {
        let sample_dir = dir.join(format!("stable-{index}"));
        fs::create_dir_all(&sample_dir).expect("stable sample directory should be created");
        let generated = sample_dir.join("main.rs");
        let output = sample_dir.join(format!("program{}", env::consts::EXE_SUFFIX));
        fs::write(&generated, &baseline_generated.stdout)
            .expect("stable baseline generated Rust should write");

        let before = compile_count(&counter);
        let (prime, elapsed) = timed_output(&mut direct_rustc_command(
            &wrapper, &rustc, &counter, &generated, &output,
        ));
        assert_success(&prime, "stable direct prime");
        assert_eq!(compile_count(&counter) - before, 1);
        run_binary(&output, &fixture_stdin, &expected_stdout);
        stable_prime.push(elapsed);

        fs::write(&generated, &edited_generated.stdout)
            .expect("stable edited generated Rust should write");
        let before = compile_count(&counter);
        let (edit, elapsed) = timed_output(&mut direct_rustc_command(
            &wrapper, &rustc, &counter, &generated, &output,
        ));
        assert_success(&edit, "stable direct edit");
        assert_eq!(compile_count(&counter) - before, 1);
        run_binary(&output, &fixture_stdin, &expected_stdout);
        stable_edit.push(elapsed);
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
        let (prime, elapsed) = timed_output(&mut incremental_rustc_command(
            &wrapper, &rustc, &counter, &generated, &output, &session,
        ));
        assert_success(&prime, "incremental direct prime");
        assert_eq!(compile_count(&counter) - before, 1);
        run_binary(&output, &fixture_stdin, &expected_stdout);
        let state = incremental_dir_stats(&session);
        assert!(
            state.files > 0,
            "incremental prime must persist state files"
        );
        incremental_prime.push(elapsed);
        state_after_prime.push(state);

        fs::write(&generated, &edited_generated.stdout)
            .expect("incremental edited generated Rust should write");
        let before = compile_count(&counter);
        let (edit, elapsed) = timed_output(&mut incremental_rustc_command(
            &wrapper, &rustc, &counter, &generated, &output, &session,
        ));
        assert_success(&edit, "incremental direct edit");
        assert_eq!(compile_count(&counter) - before, 1);
        run_binary(&output, &fixture_stdin, &expected_stdout);
        incremental_edit.push(elapsed);
        state_after_edit.push(incremental_dir_stats(&session));
    }

    let out_dir = env::var_os("EVO_INCREMENTAL_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| dir.join("incremental-build-research-report"));
    write_research_report(
        &out_dir,
        &ResearchReportInput {
            evo_prime: &evo_prime,
            evo_edit: &evo_edit,
            stable_prime: &stable_prime,
            stable_edit: &stable_edit,
            incremental_prime: &incremental_prime,
            incremental_edit: &incremental_edit,
            state_after_prime: &state_after_prime,
            state_after_edit: &state_after_edit,
            baseline_generated: &baseline_generated.stdout,
            edited_generated: &edited_generated.stdout,
            baseline_source: baseline_source.as_bytes(),
            edited_source: edited_source.as_bytes(),
            fixture_stdin: &fixture_stdin,
            expected_stdout: &expected_stdout,
            rustc_version: &rustc_version,
            rustc_c_help: &rustc_c_help,
        },
    );

    let evo_edit_stats = stats(&evo_edit);
    let stable_edit_stats = stats(&stable_edit);
    let incremental_edit_stats = stats(&incremental_edit);
    println!("evo_edit_ms={:.3}", evo_edit_stats.median_ms);
    println!("stable_edit_ms={:.3}", stable_edit_stats.median_ms);
    println!(
        "incremental_edit_ms={:.3}",
        incremental_edit_stats.median_ms
    );
    println!(
        "stable_to_incremental_speedup={:.3}",
        stable_edit_stats.median_ms / incremental_edit_stats.median_ms
    );
    println!(
        "accepted_76_to_incremental_speedup={:.3}",
        ACCEPTED_76_EDIT_BASELINE_MS / incremental_edit_stats.median_ms
    );

    let _ = fs::remove_dir_all(dir);
}
