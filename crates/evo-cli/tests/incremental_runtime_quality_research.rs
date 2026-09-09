include!("incremental_build_research.rs");

const RUNTIME_WARMUPS: usize = 3;
const RUNTIME_SAMPLES: usize = 21;
const RUNTIME_MAX_RELATIVE_MAD: f64 = 0.15;
const RUNTIME_CANDIDATE_CGU: &str = "256";

#[derive(Debug, Clone, Copy)]
struct RuntimeQualityStats {
    median_ms: f64,
    min_ms: f64,
    max_ms: f64,
    p95_ms: f64,
    relative_mad: f64,
}

struct RuntimeQualityReportInput<'a> {
    reference: &'a [Duration],
    current: &'a [Duration],
    cgu256_control: &'a [Duration],
    candidate: &'a [Duration],
    reference_bytes: u64,
    current_bytes: u64,
    cgu256_control_bytes: u64,
    candidate_bytes: u64,
    current_equals_reference: bool,
    cgu256_control_equals_reference: bool,
    candidate_equals_reference: bool,
    candidate_equals_current: bool,
    candidate_equals_cgu256_control: bool,
    state_after_prime: IncrementalDirStats,
    state_after_edit: IncrementalDirStats,
    rustc_version: &'a str,
}

struct RuntimeSampleSinks<'a> {
    reference: &'a mut Vec<Duration>,
    current: &'a mut Vec<Duration>,
    cgu256_control: &'a mut Vec<Duration>,
    candidate: &'a mut Vec<Duration>,
}

fn runtime_compile_command(
    rustc: &OsStr,
    source: &Path,
    output: &Path,
    codegen_units: &str,
    incremental: Option<&Path>,
) -> Command {
    let mut command = Command::new(rustc);
    command
        .arg(source)
        .arg(format!("--edition={RUST_EDITION}"))
        .arg("--error-format=short")
        .arg("-C")
        .arg(format!("opt-level={RUST_OPT_LEVEL}"))
        .arg("-C")
        .arg(format!("codegen-units={codegen_units}"));
    if let Some(session) = incremental {
        command
            .arg("-C")
            .arg(format!("incremental={}", session.display()));
    }
    command.arg("-o").arg(output);
    command
}

fn compile_runtime_binary(
    rustc: &OsStr,
    source: &Path,
    output: &Path,
    codegen_units: &str,
    incremental: Option<&Path>,
    label: &str,
) {
    let result = runtime_compile_command(rustc, source, output, codegen_units, incremental)
        .output()
        .expect("runtime-quality rustc should execute");
    assert_success(&result, label);
}

fn timed_runtime(binary: &Path, stdin: &[u8]) -> Duration {
    let start = Instant::now();
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("runtime-quality binary should execute");
    child
        .stdin
        .take()
        .expect("runtime-quality stdin pipe should exist")
        .write_all(stdin)
        .expect("runtime-quality stdin should write");
    let status = child.wait().expect("runtime-quality binary should finish");
    assert!(status.success(), "runtime-quality binary must succeed");
    start.elapsed()
}

fn median_f64(values: &mut [f64]) -> f64 {
    values.sort_by(|left, right| left.total_cmp(right));
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    }
}

fn runtime_quality_stats(samples: &[Duration]) -> RuntimeQualityStats {
    let summary = stats(samples);
    let mut deviations = samples
        .iter()
        .map(|sample| ((sample.as_secs_f64() * 1_000.0) - summary.median_ms).abs())
        .collect::<Vec<_>>();
    let mad_ms = median_f64(&mut deviations);
    let relative_mad = if summary.median_ms == 0.0 {
        0.0
    } else {
        mad_ms / summary.median_ms
    };

    RuntimeQualityStats {
        median_ms: summary.median_ms,
        min_ms: summary.min_ms,
        max_ms: summary.max_ms,
        p95_ms: summary.p95_ms,
        relative_mad,
    }
}

fn runtime_order(index: usize) -> [u8; 4] {
    const ORDERS: [[u8; 4]; 8] = [
        [0, 1, 2, 3],
        [3, 2, 1, 0],
        [1, 3, 0, 2],
        [2, 0, 3, 1],
        [0, 2, 1, 3],
        [3, 1, 2, 0],
        [2, 3, 0, 1],
        [1, 0, 3, 2],
    ];
    ORDERS[index % ORDERS.len()]
}

fn run_runtime_round(
    index: usize,
    stdin: &[u8],
    reference_binary: &Path,
    current_binary: &Path,
    cgu256_control_binary: &Path,
    candidate_binary: &Path,
    mut samples: Option<RuntimeSampleSinks<'_>>,
) {
    for arm in runtime_order(index) {
        let binary = match arm {
            0 => reference_binary,
            1 => current_binary,
            2 => cgu256_control_binary,
            3 => candidate_binary,
            _ => unreachable!("runtime arm must be known"),
        };
        let elapsed = timed_runtime(binary, stdin);
        if let Some(values) = samples.as_mut() {
            match arm {
                0 => values.reference.push(elapsed),
                1 => values.current.push(elapsed),
                2 => values.cgu256_control.push(elapsed),
                3 => values.candidate.push(elapsed),
                _ => unreachable!("runtime arm must be known"),
            }
        }
    }
}

fn runtime_verdict(
    stable: bool,
    candidate_equals_reference: bool,
    candidate_to_reference: f64,
    candidate_equals_current: bool,
    candidate_to_current: f64,
) -> &'static str {
    if candidate_equals_reference && candidate_equals_current {
        "PASS-BINARY-PARITY"
    } else if !stable {
        "INCONCLUSIVE"
    } else if (candidate_equals_reference || candidate_to_reference <= 1.0)
        && (candidate_equals_current || candidate_to_current <= 1.0)
    {
        "PASS"
    } else {
        "FAIL"
    }
}

fn push_runtime_csv_rows(csv: &mut String, arm: &str, samples: &[Duration]) {
    for (index, sample) in samples.iter().enumerate() {
        writeln!(
            csv,
            "{arm},{},{:.3}",
            index + 1,
            sample.as_secs_f64() * 1_000.0
        )
        .expect("writing runtime-quality CSV to String should not fail");
    }
}

fn write_runtime_quality_report(out_dir: &Path, input: &RuntimeQualityReportInput<'_>) {
    fs::create_dir_all(out_dir).expect("runtime-quality output directory should be created");

    let reference = runtime_quality_stats(input.reference);
    let current = runtime_quality_stats(input.current);
    let cgu256_control = runtime_quality_stats(input.cgu256_control);
    let candidate = runtime_quality_stats(input.candidate);
    let max_relative_mad = reference
        .relative_mad
        .max(current.relative_mad)
        .max(cgu256_control.relative_mad)
        .max(candidate.relative_mad);
    let stable = max_relative_mad <= RUNTIME_MAX_RELATIVE_MAD;
    let current_to_reference = current.median_ms / reference.median_ms;
    let cgu256_control_to_reference = cgu256_control.median_ms / reference.median_ms;
    let candidate_to_reference = candidate.median_ms / reference.median_ms;
    let candidate_to_current = candidate.median_ms / current.median_ms;
    let candidate_to_cgu256_control = candidate.median_ms / cgu256_control.median_ms;
    let verdict = runtime_verdict(
        stable,
        input.candidate_equals_reference,
        candidate_to_reference,
        input.candidate_equals_current,
        candidate_to_current,
    );
    let git_sha = env::var("EVO_GIT_SHA")
        .or_else(|_| env::var("GITHUB_SHA"))
        .unwrap_or_else(|_| "local".to_owned());
    let host = rustc_host(input.rustc_version);

    let markdown = format!(
        r#"# Incremental candidate runtime-quality research v0

- git_sha: `{git_sha}`
- platform: `{}-{}`
- rustc_host: `{host}`
- fixture: `benchmarks/cases/enums-v0/evolution.evo`
- warmups_per_arm: `{RUNTIME_WARMUPS}`
- samples_per_arm: `{RUNTIME_SAMPLES}`
- max_relative_mad: `{RUNTIME_MAX_RELATIVE_MAD:.3}`
- current_flags: `--edition={RUST_EDITION} --error-format=short -C opt-level={RUST_OPT_LEVEL} -C codegen-units={RUST_CODEGEN_UNITS}`
- candidate_flags: `--edition={RUST_EDITION} --error-format=short -C opt-level={RUST_OPT_LEVEL} -C codegen-units={RUNTIME_CANDIDATE_CGU} -C incremental=<session>`
- correctness: `PASS`
- stable_measurement: `{stable}`
- verdict: `{verdict}`

| Runtime arm | Median/min/max/p95 ms | Relative MAD | Binary bytes |
| --- | ---: | ---: | ---: |
| reference Rust, CGU 1 | {:.3}/{:.3}/{:.3}/{:.3} | {:.6} | {} |
| edited Evolution generated Rust, current CGU 1 | {:.3}/{:.3}/{:.3}/{:.3} | {:.6} | {} |
| edited Evolution generated Rust, CGU 256, no incremental | {:.3}/{:.3}/{:.3}/{:.3} | {:.6} | {} |
| edited Evolution generated Rust, CGU 256, reused incremental state | {:.3}/{:.3}/{:.3}/{:.3} | {:.6} | {} |

- current Evolution / reference ratio: `{current_to_reference:.6}`
- CGU-256 control / reference ratio: `{cgu256_control_to_reference:.6}`
- candidate / reference ratio: `{candidate_to_reference:.6}`
- candidate / current-CGU-1 ratio: `{candidate_to_current:.6}`
- candidate / matching-CGU-256-control ratio: `{candidate_to_cgu256_control:.6}`

Binary equality:

- current CGU 1 == reference: `{}`
- CGU 256 control == reference: `{}`
- incremental candidate == reference: `{}`
- incremental candidate == current CGU 1: `{}`
- incremental candidate == matching CGU 256 control: `{}`

Incremental state for the candidate compile:

- after prime: `{}` files / `{}` bytes
- after edit: `{}` files / `{}` bytes

The reference arm uses the committed accepted Rust fixture. The current and candidate Evolution arms use the exact edited generated Rust from the same deterministic result-preserving source edit used by #76/#82. The candidate binary is produced by a real baseline prime followed by an edited compile that reuses the same rustc incremental session. Timed samples redirect stdout/stderr symmetrically to the platform null device after exact-output correctness checks. This is a research gate only; no production compiler flags or cache behavior are changed.
"#,
        env::consts::OS,
        env::consts::ARCH,
        reference.median_ms,
        reference.min_ms,
        reference.max_ms,
        reference.p95_ms,
        reference.relative_mad,
        input.reference_bytes,
        current.median_ms,
        current.min_ms,
        current.max_ms,
        current.p95_ms,
        current.relative_mad,
        input.current_bytes,
        cgu256_control.median_ms,
        cgu256_control.min_ms,
        cgu256_control.max_ms,
        cgu256_control.p95_ms,
        cgu256_control.relative_mad,
        input.cgu256_control_bytes,
        candidate.median_ms,
        candidate.min_ms,
        candidate.max_ms,
        candidate.p95_ms,
        candidate.relative_mad,
        input.candidate_bytes,
        input.current_equals_reference,
        input.cgu256_control_equals_reference,
        input.candidate_equals_reference,
        input.candidate_equals_current,
        input.candidate_equals_cgu256_control,
        input.state_after_prime.files,
        input.state_after_prime.bytes,
        input.state_after_edit.files,
        input.state_after_edit.bytes
    );
    fs::write(out_dir.join("report.md"), markdown)
        .expect("runtime-quality Markdown report should be written");

    let json = format!(
        r#"{{
  "git_sha": "{git_sha}",
  "platform": "{}-{}",
  "rustc_host": "{host}",
  "warmups_per_arm": {RUNTIME_WARMUPS},
  "samples_per_arm": {RUNTIME_SAMPLES},
  "max_relative_mad": {RUNTIME_MAX_RELATIVE_MAD:.6},
  "stable_measurement": {stable},
  "verdict": "{verdict}",
  "correctness": "PASS",
  "reference_samples_ms": [{}],
  "current_samples_ms": [{}],
  "cgu256_control_samples_ms": [{}],
  "candidate_samples_ms": [{}],
  "reference_median_ms": {:.3},
  "current_median_ms": {:.3},
  "cgu256_control_median_ms": {:.3},
  "candidate_median_ms": {:.3},
  "reference_relative_mad": {:.9},
  "current_relative_mad": {:.9},
  "cgu256_control_relative_mad": {:.9},
  "candidate_relative_mad": {:.9},
  "current_to_reference_ratio": {:.9},
  "cgu256_control_to_reference_ratio": {:.9},
  "candidate_to_reference_ratio": {:.9},
  "candidate_to_current_ratio": {:.9},
  "candidate_to_cgu256_control_ratio": {:.9},
  "reference_binary_bytes": {},
  "current_binary_bytes": {},
  "cgu256_control_binary_bytes": {},
  "candidate_binary_bytes": {},
  "current_equals_reference": {},
  "cgu256_control_equals_reference": {},
  "candidate_equals_reference": {},
  "candidate_equals_current": {},
  "candidate_equals_cgu256_control": {},
  "candidate_state_after_prime_files": {},
  "candidate_state_after_prime_bytes": {},
  "candidate_state_after_edit_files": {},
  "candidate_state_after_edit_bytes": {}
}}
"#,
        env::consts::OS,
        env::consts::ARCH,
        samples_json(input.reference),
        samples_json(input.current),
        samples_json(input.cgu256_control),
        samples_json(input.candidate),
        reference.median_ms,
        current.median_ms,
        cgu256_control.median_ms,
        candidate.median_ms,
        reference.relative_mad,
        current.relative_mad,
        cgu256_control.relative_mad,
        candidate.relative_mad,
        current_to_reference,
        cgu256_control_to_reference,
        candidate_to_reference,
        candidate_to_current,
        candidate_to_cgu256_control,
        input.reference_bytes,
        input.current_bytes,
        input.cgu256_control_bytes,
        input.candidate_bytes,
        input.current_equals_reference,
        input.cgu256_control_equals_reference,
        input.candidate_equals_reference,
        input.candidate_equals_current,
        input.candidate_equals_cgu256_control,
        input.state_after_prime.files,
        input.state_after_prime.bytes,
        input.state_after_edit.files,
        input.state_after_edit.bytes
    );
    fs::write(out_dir.join("report.json"), json)
        .expect("runtime-quality JSON report should be written");

    let mut csv = String::from("arm,index,elapsed_ms\n");
    push_runtime_csv_rows(&mut csv, "reference-cgu1", input.reference);
    push_runtime_csv_rows(&mut csv, "evolution-current-cgu1", input.current);
    push_runtime_csv_rows(
        &mut csv,
        "evolution-cgu256-no-incremental",
        input.cgu256_control,
    );
    push_runtime_csv_rows(&mut csv, "evolution-cgu256-incremental", input.candidate);
    fs::write(out_dir.join("raw-samples.csv"), csv)
        .expect("runtime-quality CSV report should be written");
    fs::write(out_dir.join("rustc-vV.txt"), input.rustc_version)
        .expect("runtime-quality rustc version evidence should be written");
}

#[test]
#[ignore = "run explicitly on Ubuntu after build-latency feasibility slices"]
fn incremental_candidate_runtime_quality_is_measured_against_current_and_reference() {
    let dir = temp_dir("incremental-runtime-quality");
    fs::create_dir_all(&dir).expect("runtime-quality directory should be created");

    let fixture = fixture_dir();
    let fixture_source =
        fs::read(fixture.join("evolution.evo")).expect("fixture source should read");
    let fixture_stdin = fs::read(fixture.join("stdin.bin")).expect("fixture stdin should read");
    let expected_stdout =
        fs::read(fixture.join("expected.stdout")).expect("fixture expected stdout should read");
    let reference_rust =
        fs::read(fixture.join("reference.rs")).expect("reference Rust should read");

    let baseline_source =
        String::from_utf8(fixture_source).expect("fixture source should be UTF-8");
    let edited_source = baseline_source.replacen(EDIT_FROM, EDIT_TO, 1);
    assert_ne!(baseline_source, edited_source);

    let evolution_source = dir.join("program.evo");
    fs::write(&evolution_source, &baseline_source).expect("baseline Evolution source should write");
    let baseline_generated = evo_command("emit-rust", &evolution_source)
        .output()
        .expect("baseline emit-rust should execute");
    assert_success(&baseline_generated, "baseline emit-rust");
    assert_eq!(
        baseline_generated.stdout, reference_rust,
        "accepted baseline generated Rust must match committed reference Rust"
    );

    fs::write(&evolution_source, &edited_source).expect("edited Evolution source should write");
    let edited_generated = evo_command("emit-rust", &evolution_source)
        .output()
        .expect("edited emit-rust should execute");
    assert_success(&edited_generated, "edited emit-rust");
    assert_ne!(baseline_generated.stdout, edited_generated.stdout);

    let rustc = real_rustc();
    let version = rustc_version(&rustc);
    let source = dir.join("main.rs");
    let reference_binary = dir.join(format!("reference{}", env::consts::EXE_SUFFIX));
    let current_binary = dir.join(format!("current{}", env::consts::EXE_SUFFIX));
    let cgu256_control_binary = dir.join(format!("cgu256-control{}", env::consts::EXE_SUFFIX));
    let candidate_prime_binary = dir.join(format!("candidate-prime{}", env::consts::EXE_SUFFIX));
    let candidate_binary = dir.join(format!("candidate{}", env::consts::EXE_SUFFIX));
    let session = dir.join("candidate-session");

    fs::write(&source, &reference_rust).expect("reference Rust should stage");
    compile_runtime_binary(
        &rustc,
        &source,
        &reference_binary,
        RUST_CODEGEN_UNITS,
        None,
        "reference CGU 1 compile",
    );
    run_binary(&reference_binary, &fixture_stdin, &expected_stdout);

    fs::write(&source, &edited_generated.stdout).expect("edited generated Rust should stage");
    compile_runtime_binary(
        &rustc,
        &source,
        &current_binary,
        RUST_CODEGEN_UNITS,
        None,
        "current CGU 1 compile",
    );
    run_binary(&current_binary, &fixture_stdin, &expected_stdout);

    compile_runtime_binary(
        &rustc,
        &source,
        &cgu256_control_binary,
        RUNTIME_CANDIDATE_CGU,
        None,
        "CGU 256 control compile",
    );
    run_binary(&cgu256_control_binary, &fixture_stdin, &expected_stdout);

    fs::write(&source, &baseline_generated.stdout).expect("candidate baseline Rust should stage");
    compile_runtime_binary(
        &rustc,
        &source,
        &candidate_prime_binary,
        RUNTIME_CANDIDATE_CGU,
        Some(&session),
        "candidate incremental prime compile",
    );
    run_binary(&candidate_prime_binary, &fixture_stdin, &expected_stdout);
    let state_after_prime = incremental_dir_stats(&session);
    assert!(
        state_after_prime.files > 0,
        "candidate prime must persist state"
    );

    fs::write(&source, &edited_generated.stdout).expect("candidate edited Rust should stage");
    compile_runtime_binary(
        &rustc,
        &source,
        &candidate_binary,
        RUNTIME_CANDIDATE_CGU,
        Some(&session),
        "candidate incremental edited compile",
    );
    run_binary(&candidate_binary, &fixture_stdin, &expected_stdout);
    let state_after_edit = incremental_dir_stats(&session);

    for index in 0..RUNTIME_WARMUPS {
        run_runtime_round(
            index,
            &fixture_stdin,
            &reference_binary,
            &current_binary,
            &cgu256_control_binary,
            &candidate_binary,
            None,
        );
    }

    let mut reference_samples = Vec::with_capacity(RUNTIME_SAMPLES);
    let mut current_samples = Vec::with_capacity(RUNTIME_SAMPLES);
    let mut cgu256_control_samples = Vec::with_capacity(RUNTIME_SAMPLES);
    let mut candidate_samples = Vec::with_capacity(RUNTIME_SAMPLES);
    for index in 0..RUNTIME_SAMPLES {
        run_runtime_round(
            index + RUNTIME_WARMUPS,
            &fixture_stdin,
            &reference_binary,
            &current_binary,
            &cgu256_control_binary,
            &candidate_binary,
            Some(RuntimeSampleSinks {
                reference: &mut reference_samples,
                current: &mut current_samples,
                cgu256_control: &mut cgu256_control_samples,
                candidate: &mut candidate_samples,
            }),
        );
    }

    let reference_bytes = fs::read(&reference_binary).expect("reference binary should read");
    let current_bytes = fs::read(&current_binary).expect("current binary should read");
    let cgu256_control_bytes =
        fs::read(&cgu256_control_binary).expect("CGU 256 control binary should read");
    let candidate_bytes = fs::read(&candidate_binary).expect("candidate binary should read");

    let out_root = env::var_os("EVO_INCREMENTAL_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| dir.join("incremental-build-research-report"));
    let out_dir = out_root.join("runtime-quality");
    write_runtime_quality_report(
        &out_dir,
        &RuntimeQualityReportInput {
            reference: &reference_samples,
            current: &current_samples,
            cgu256_control: &cgu256_control_samples,
            candidate: &candidate_samples,
            reference_bytes: reference_bytes.len() as u64,
            current_bytes: current_bytes.len() as u64,
            cgu256_control_bytes: cgu256_control_bytes.len() as u64,
            candidate_bytes: candidate_bytes.len() as u64,
            current_equals_reference: current_bytes == reference_bytes,
            cgu256_control_equals_reference: cgu256_control_bytes == reference_bytes,
            candidate_equals_reference: candidate_bytes == reference_bytes,
            candidate_equals_current: candidate_bytes == current_bytes,
            candidate_equals_cgu256_control: candidate_bytes == cgu256_control_bytes,
            state_after_prime,
            state_after_edit,
            rustc_version: &version,
        },
    );

    let reference = runtime_quality_stats(&reference_samples);
    let current = runtime_quality_stats(&current_samples);
    let cgu256_control = runtime_quality_stats(&cgu256_control_samples);
    let candidate = runtime_quality_stats(&candidate_samples);
    println!("runtime_reference_cgu1_ms={:.3}", reference.median_ms);
    println!("runtime_current_cgu1_ms={:.3}", current.median_ms);
    println!("runtime_cgu256_control_ms={:.3}", cgu256_control.median_ms);
    println!("runtime_candidate_ms={:.3}", candidate.median_ms);
    println!(
        "runtime_candidate_to_reference={:.6}",
        candidate.median_ms / reference.median_ms
    );
    println!(
        "runtime_candidate_to_current={:.6}",
        candidate.median_ms / current.median_ms
    );

    let _ = fs::remove_dir_all(dir);
}
