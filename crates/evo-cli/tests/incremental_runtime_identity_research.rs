include!("incremental_runtime_quality_research.rs");

const IDENTITY_WARMUPS: usize = 3;
const IDENTITY_SAMPLES: usize = 21;

#[derive(Debug, Clone, Copy)]
struct BinaryDiffSummary {
    differing_bytes: u64,
    first_difference: Option<usize>,
}

struct IdentityReportInput<'a> {
    baseline_control: &'a [Duration],
    edited_control: &'a [Duration],
    incremental_prime: &'a [Duration],
    incremental_edit: &'a [Duration],
    baseline_control_bytes: &'a [u8],
    edited_control_bytes: &'a [u8],
    incremental_prime_bytes: &'a [u8],
    incremental_edit_bytes: &'a [u8],
    state_after_prime: IncrementalDirStats,
    state_after_edit: IncrementalDirStats,
    rustc_version: &'a str,
}

fn binary_diff_summary(left: &[u8], right: &[u8]) -> BinaryDiffSummary {
    let common_len = left.len().min(right.len());
    let mut differing_bytes = 0_u64;
    let mut first_difference = None;

    for index in 0..common_len {
        if left[index] != right[index] {
            differing_bytes += 1;
            first_difference.get_or_insert(index);
        }
    }

    if left.len() != right.len() {
        differing_bytes += left.len().abs_diff(right.len()) as u64;
        first_difference.get_or_insert(common_len);
    }

    BinaryDiffSummary {
        differing_bytes,
        first_difference,
    }
}

fn optional_usize_json(value: Option<usize>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| value.to_string())
}

fn write_identity_report(out_dir: &Path, input: &IdentityReportInput<'_>) {
    fs::create_dir_all(out_dir).expect("runtime-identity output directory should be created");

    let baseline_control = runtime_quality_stats(input.baseline_control);
    let edited_control = runtime_quality_stats(input.edited_control);
    let incremental_prime = runtime_quality_stats(input.incremental_prime);
    let incremental_edit = runtime_quality_stats(input.incremental_edit);
    let max_relative_mad = baseline_control
        .relative_mad
        .max(edited_control.relative_mad)
        .max(incremental_prime.relative_mad)
        .max(incremental_edit.relative_mad);
    let stable = max_relative_mad <= RUNTIME_MAX_RELATIVE_MAD;

    let prime_flag_ratio = incremental_prime.median_ms / baseline_control.median_ms;
    let edit_flag_ratio = incremental_edit.median_ms / edited_control.median_ms;
    let control_edit_ratio = edited_control.median_ms / baseline_control.median_ms;
    let incremental_reuse_ratio = incremental_edit.median_ms / incremental_prime.median_ms;

    let baseline_flag_diff =
        binary_diff_summary(input.baseline_control_bytes, input.incremental_prime_bytes);
    let edited_flag_diff =
        binary_diff_summary(input.edited_control_bytes, input.incremental_edit_bytes);
    let control_edit_diff =
        binary_diff_summary(input.baseline_control_bytes, input.edited_control_bytes);
    let incremental_edit_diff =
        binary_diff_summary(input.incremental_prime_bytes, input.incremental_edit_bytes);

    let git_sha = env::var("EVO_GIT_SHA")
        .or_else(|_| env::var("GITHUB_SHA"))
        .unwrap_or_else(|_| "local".to_owned());
    let host = rustc_host(input.rustc_version);

    let markdown = format!(
        r#"# Incremental runtime/codegen identity research v0

- git_sha: `{git_sha}`
- platform: `{}-{}`
- rustc_host: `{host}`
- fixture: `benchmarks/cases/enums-v0/evolution.evo`
- warmups_per_arm: `{IDENTITY_WARMUPS}`
- samples_per_arm: `{IDENTITY_SAMPLES}`
- rustc_base_flags: `--edition={RUST_EDITION} --error-format=short -C opt-level={RUST_OPT_LEVEL} -C codegen-units={RUNTIME_CANDIDATE_CGU}`
- incremental_delta: `-C incremental=<single-session>`
- correctness: `PASS`
- stable_measurement: `{stable}`

| Arm | Median/min/max/p95 ms | Relative MAD | Binary bytes |
| --- | ---: | ---: | ---: |
| baseline generated Rust, CGU256, no incremental | {:.3}/{:.3}/{:.3}/{:.3} | {:.6} | {} |
| edited generated Rust, CGU256, no incremental | {:.3}/{:.3}/{:.3}/{:.3} | {:.6} | {} |
| baseline generated Rust, CGU256, incremental prime | {:.3}/{:.3}/{:.3}/{:.3} | {:.6} | {} |
| edited generated Rust, CGU256, incremental reuse | {:.3}/{:.3}/{:.3}/{:.3} | {:.6} | {} |

Runtime ratios:

- incremental-prime / baseline-no-incremental: `{prime_flag_ratio:.6}`
- incremental-edit / edited-no-incremental: `{edit_flag_ratio:.6}`
- edited-no-incremental / baseline-no-incremental: `{control_edit_ratio:.6}`
- incremental-edit / incremental-prime: `{incremental_reuse_ratio:.6}`

Binary identity:

- baseline no-incremental == incremental prime: `{}`
  - differing bytes: `{}`
  - first differing offset: `{}`
- edited no-incremental == incremental edit: `{}`
  - differing bytes: `{}`
  - first differing offset: `{}`
- baseline no-incremental == edited no-incremental: `{}`
  - differing bytes: `{}`
  - first differing offset: `{}`
- incremental prime == incremental edit: `{}`
  - differing bytes: `{}`
  - first differing offset: `{}`

Incremental state:

- after prime: `{}` files / `{}` bytes
- after edit: `{}` files / `{}` bytes

All four binaries use the same stable generated-source path. The non-incremental baseline and edit are compiled independently with otherwise identical CGU256 flags. The incremental pair uses one fresh session, first compiling baseline generated Rust and then overwriting the same `main.rs` with the deterministic result-preserving edit before recompiling. Every binary is correctness-checked with committed stdin/stdout before timing. This slice exists to distinguish codegen/runtime effects caused by enabling incremental mode from effects caused by reuse across the edit; it does not change production behavior.
"#,
        env::consts::OS,
        env::consts::ARCH,
        baseline_control.median_ms,
        baseline_control.min_ms,
        baseline_control.max_ms,
        baseline_control.p95_ms,
        baseline_control.relative_mad,
        input.baseline_control_bytes.len(),
        edited_control.median_ms,
        edited_control.min_ms,
        edited_control.max_ms,
        edited_control.p95_ms,
        edited_control.relative_mad,
        input.edited_control_bytes.len(),
        incremental_prime.median_ms,
        incremental_prime.min_ms,
        incremental_prime.max_ms,
        incremental_prime.p95_ms,
        incremental_prime.relative_mad,
        input.incremental_prime_bytes.len(),
        incremental_edit.median_ms,
        incremental_edit.min_ms,
        incremental_edit.max_ms,
        incremental_edit.p95_ms,
        incremental_edit.relative_mad,
        input.incremental_edit_bytes.len(),
        input.baseline_control_bytes == input.incremental_prime_bytes,
        baseline_flag_diff.differing_bytes,
        optional_usize_json(baseline_flag_diff.first_difference),
        input.edited_control_bytes == input.incremental_edit_bytes,
        edited_flag_diff.differing_bytes,
        optional_usize_json(edited_flag_diff.first_difference),
        input.baseline_control_bytes == input.edited_control_bytes,
        control_edit_diff.differing_bytes,
        optional_usize_json(control_edit_diff.first_difference),
        input.incremental_prime_bytes == input.incremental_edit_bytes,
        incremental_edit_diff.differing_bytes,
        optional_usize_json(incremental_edit_diff.first_difference),
        input.state_after_prime.files,
        input.state_after_prime.bytes,
        input.state_after_edit.files,
        input.state_after_edit.bytes
    );
    fs::write(out_dir.join("report.md"), markdown)
        .expect("runtime-identity Markdown report should be written");

    let json = format!(
        r#"{{
  "git_sha": "{git_sha}",
  "platform": "{}-{}",
  "rustc_host": "{host}",
  "warmups_per_arm": {IDENTITY_WARMUPS},
  "samples_per_arm": {IDENTITY_SAMPLES},
  "stable_measurement": {stable},
  "correctness": "PASS",
  "baseline_control_samples_ms": [{}],
  "edited_control_samples_ms": [{}],
  "incremental_prime_samples_ms": [{}],
  "incremental_edit_samples_ms": [{}],
  "baseline_control_median_ms": {:.3},
  "edited_control_median_ms": {:.3},
  "incremental_prime_median_ms": {:.3},
  "incremental_edit_median_ms": {:.3},
  "incremental_prime_to_baseline_control_ratio": {:.9},
  "incremental_edit_to_edited_control_ratio": {:.9},
  "edited_control_to_baseline_control_ratio": {:.9},
  "incremental_edit_to_incremental_prime_ratio": {:.9},
  "baseline_control_binary_bytes": {},
  "edited_control_binary_bytes": {},
  "incremental_prime_binary_bytes": {},
  "incremental_edit_binary_bytes": {},
  "baseline_control_equals_incremental_prime": {},
  "baseline_flag_differing_bytes": {},
  "baseline_flag_first_difference": {},
  "edited_control_equals_incremental_edit": {},
  "edited_flag_differing_bytes": {},
  "edited_flag_first_difference": {},
  "baseline_control_equals_edited_control": {},
  "control_edit_differing_bytes": {},
  "control_edit_first_difference": {},
  "incremental_prime_equals_incremental_edit": {},
  "incremental_edit_differing_bytes": {},
  "incremental_edit_first_difference": {},
  "state_after_prime_files": {},
  "state_after_prime_bytes": {},
  "state_after_edit_files": {},
  "state_after_edit_bytes": {}
}}
"#,
        env::consts::OS,
        env::consts::ARCH,
        samples_json(input.baseline_control),
        samples_json(input.edited_control),
        samples_json(input.incremental_prime),
        samples_json(input.incremental_edit),
        baseline_control.median_ms,
        edited_control.median_ms,
        incremental_prime.median_ms,
        incremental_edit.median_ms,
        prime_flag_ratio,
        edit_flag_ratio,
        control_edit_ratio,
        incremental_reuse_ratio,
        input.baseline_control_bytes.len(),
        input.edited_control_bytes.len(),
        input.incremental_prime_bytes.len(),
        input.incremental_edit_bytes.len(),
        input.baseline_control_bytes == input.incremental_prime_bytes,
        baseline_flag_diff.differing_bytes,
        optional_usize_json(baseline_flag_diff.first_difference),
        input.edited_control_bytes == input.incremental_edit_bytes,
        edited_flag_diff.differing_bytes,
        optional_usize_json(edited_flag_diff.first_difference),
        input.baseline_control_bytes == input.edited_control_bytes,
        control_edit_diff.differing_bytes,
        optional_usize_json(control_edit_diff.first_difference),
        input.incremental_prime_bytes == input.incremental_edit_bytes,
        incremental_edit_diff.differing_bytes,
        optional_usize_json(incremental_edit_diff.first_difference),
        input.state_after_prime.files,
        input.state_after_prime.bytes,
        input.state_after_edit.files,
        input.state_after_edit.bytes
    );
    fs::write(out_dir.join("report.json"), json)
        .expect("runtime-identity JSON report should be written");

    let mut csv = String::from("arm,index,elapsed_ms\n");
    push_runtime_csv_rows(
        &mut csv,
        "baseline-cgu256-no-incremental",
        input.baseline_control,
    );
    push_runtime_csv_rows(
        &mut csv,
        "edited-cgu256-no-incremental",
        input.edited_control,
    );
    push_runtime_csv_rows(
        &mut csv,
        "baseline-cgu256-incremental-prime",
        input.incremental_prime,
    );
    push_runtime_csv_rows(
        &mut csv,
        "edited-cgu256-incremental-reuse",
        input.incremental_edit,
    );
    fs::write(out_dir.join("raw-samples.csv"), csv)
        .expect("runtime-identity CSV report should be written");
    fs::write(out_dir.join("rustc-vV.txt"), input.rustc_version)
        .expect("runtime-identity rustc version evidence should be written");
}

#[test]
#[ignore = "run explicitly after the runtime-quality slice on Ubuntu"]
fn incremental_runtime_identity_isolates_flag_from_edit_reuse() {
    let dir = temp_dir("incremental-runtime-identity");
    fs::create_dir_all(&dir).expect("runtime-identity directory should be created");

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

    let evolution_source = dir.join("program.evo");
    fs::write(&evolution_source, &baseline_source).expect("baseline Evolution source should write");
    let baseline_generated = evo_command("emit-rust", &evolution_source)
        .output()
        .expect("baseline emit-rust should execute");
    assert_success(&baseline_generated, "baseline emit-rust");

    fs::write(&evolution_source, &edited_source).expect("edited Evolution source should write");
    let edited_generated = evo_command("emit-rust", &evolution_source)
        .output()
        .expect("edited emit-rust should execute");
    assert_success(&edited_generated, "edited emit-rust");
    assert_ne!(baseline_generated.stdout, edited_generated.stdout);

    let rustc = real_rustc();
    let version = rustc_version(&rustc);
    let generated = dir.join("main.rs");
    let baseline_control_binary = dir.join(format!("baseline-control{}", env::consts::EXE_SUFFIX));
    let edited_control_binary = dir.join(format!("edited-control{}", env::consts::EXE_SUFFIX));
    let incremental_prime_binary =
        dir.join(format!("incremental-prime{}", env::consts::EXE_SUFFIX));
    let incremental_edit_binary = dir.join(format!("incremental-edit{}", env::consts::EXE_SUFFIX));
    let session = dir.join("session");

    fs::write(&generated, &baseline_generated.stdout)
        .expect("baseline generated Rust should stage");
    compile_runtime_binary(
        &rustc,
        &generated,
        &baseline_control_binary,
        RUNTIME_CANDIDATE_CGU,
        None,
        "baseline CGU256 no-incremental compile",
    );
    run_binary(&baseline_control_binary, &fixture_stdin, &expected_stdout);

    fs::write(&generated, &edited_generated.stdout).expect("edited generated Rust should stage");
    compile_runtime_binary(
        &rustc,
        &generated,
        &edited_control_binary,
        RUNTIME_CANDIDATE_CGU,
        None,
        "edited CGU256 no-incremental compile",
    );
    run_binary(&edited_control_binary, &fixture_stdin, &expected_stdout);

    fs::write(&generated, &baseline_generated.stdout)
        .expect("incremental prime generated Rust should stage");
    compile_runtime_binary(
        &rustc,
        &generated,
        &incremental_prime_binary,
        RUNTIME_CANDIDATE_CGU,
        Some(&session),
        "baseline CGU256 incremental prime compile",
    );
    run_binary(&incremental_prime_binary, &fixture_stdin, &expected_stdout);
    let state_after_prime = incremental_dir_stats(&session);
    assert!(
        state_after_prime.files > 0,
        "incremental prime must persist state"
    );

    fs::write(&generated, &edited_generated.stdout)
        .expect("incremental edited generated Rust should stage");
    compile_runtime_binary(
        &rustc,
        &generated,
        &incremental_edit_binary,
        RUNTIME_CANDIDATE_CGU,
        Some(&session),
        "edited CGU256 incremental reuse compile",
    );
    run_binary(&incremental_edit_binary, &fixture_stdin, &expected_stdout);
    let state_after_edit = incremental_dir_stats(&session);

    for index in 0..IDENTITY_WARMUPS {
        run_runtime_round(
            index,
            &fixture_stdin,
            &baseline_control_binary,
            &edited_control_binary,
            &incremental_prime_binary,
            &incremental_edit_binary,
            None,
        );
    }

    let mut baseline_control_samples = Vec::with_capacity(IDENTITY_SAMPLES);
    let mut edited_control_samples = Vec::with_capacity(IDENTITY_SAMPLES);
    let mut incremental_prime_samples = Vec::with_capacity(IDENTITY_SAMPLES);
    let mut incremental_edit_samples = Vec::with_capacity(IDENTITY_SAMPLES);
    for index in 0..IDENTITY_SAMPLES {
        run_runtime_round(
            index + IDENTITY_WARMUPS,
            &fixture_stdin,
            &baseline_control_binary,
            &edited_control_binary,
            &incremental_prime_binary,
            &incremental_edit_binary,
            Some(RuntimeSampleSinks {
                reference: &mut baseline_control_samples,
                current: &mut edited_control_samples,
                cgu256_control: &mut incremental_prime_samples,
                candidate: &mut incremental_edit_samples,
            }),
        );
    }

    let baseline_control_bytes =
        fs::read(&baseline_control_binary).expect("baseline control binary should read");
    let edited_control_bytes =
        fs::read(&edited_control_binary).expect("edited control binary should read");
    let incremental_prime_bytes =
        fs::read(&incremental_prime_binary).expect("incremental prime binary should read");
    let incremental_edit_bytes =
        fs::read(&incremental_edit_binary).expect("incremental edit binary should read");

    let out_root = env::var_os("EVO_INCREMENTAL_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| dir.join("incremental-build-research-report"));
    let out_dir = out_root.join("runtime-identity");
    write_identity_report(
        &out_dir,
        &IdentityReportInput {
            baseline_control: &baseline_control_samples,
            edited_control: &edited_control_samples,
            incremental_prime: &incremental_prime_samples,
            incremental_edit: &incremental_edit_samples,
            baseline_control_bytes: &baseline_control_bytes,
            edited_control_bytes: &edited_control_bytes,
            incremental_prime_bytes: &incremental_prime_bytes,
            incremental_edit_bytes: &incremental_edit_bytes,
            state_after_prime,
            state_after_edit,
            rustc_version: &version,
        },
    );

    let baseline_control = runtime_quality_stats(&baseline_control_samples);
    let edited_control = runtime_quality_stats(&edited_control_samples);
    let incremental_prime = runtime_quality_stats(&incremental_prime_samples);
    let incremental_edit = runtime_quality_stats(&incremental_edit_samples);
    println!(
        "identity_baseline_control_ms={:.3}",
        baseline_control.median_ms
    );
    println!("identity_edited_control_ms={:.3}", edited_control.median_ms);
    println!(
        "identity_incremental_prime_ms={:.3}",
        incremental_prime.median_ms
    );
    println!(
        "identity_incremental_edit_ms={:.3}",
        incremental_edit.median_ms
    );
    println!(
        "identity_prime_flag_ratio={:.6}",
        incremental_prime.median_ms / baseline_control.median_ms
    );
    println!(
        "identity_edit_flag_ratio={:.6}",
        incremental_edit.median_ms / edited_control.median_ms
    );

    let _ = fs::remove_dir_all(dir);
}
