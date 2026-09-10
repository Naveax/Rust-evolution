include!("build_latency_baseline.rs");

use std::fmt::Write as _;

const OPT_RESEARCH_WARMUPS: usize = 2;
const OPT_RESEARCH_SAMPLES: usize = 9;
const OPT_RESEARCH_CASES: [&str; 2] = ["enums-v0", "logical-operators-v0"];
const OPT_CANDIDATE_LEVEL: &str = "2";
const OPT_BUILD_MIN_IMPROVEMENT: f64 = 0.05;
const OPT_BUILD_MIN_SAVED_MS: f64 = 5.0;
const OPT_BUILD_MAX_RELATIVE_MAD: f64 = 0.10;

#[derive(Clone, Copy)]
enum OptimizationArm {
    Current,
    Candidate,
}

impl OptimizationArm {
    fn label(self) -> &'static str {
        match self {
            Self::Current => "opt3-current",
            Self::Candidate => "opt2-candidate",
        }
    }

    fn level(self) -> &'static str {
        match self {
            Self::Current => RUST_OPT_LEVEL,
            Self::Candidate => OPT_CANDIDATE_LEVEL,
        }
    }
}

struct OptimizationArmResult {
    label: &'static str,
    samples: Vec<Duration>,
    binary_bytes: u64,
}

struct OptimizationCaseResult {
    name: String,
    generated_rust: Vec<u8>,
    current: OptimizationArmResult,
    candidate: OptimizationArmResult,
}

fn optimization_order(index: usize) -> [OptimizationArm; 2] {
    if index.is_multiple_of(2) {
        [OptimizationArm::Current, OptimizationArm::Candidate]
    } else {
        [OptimizationArm::Candidate, OptimizationArm::Current]
    }
}

fn optimization_rustc_command(
    rustc: &OsStr,
    source: &Path,
    output: &Path,
    opt_level: &str,
) -> Command {
    let mut command = Command::new(rustc);
    command
        .arg(source)
        .arg(format!("--edition={RUST_EDITION}"))
        .arg("--error-format=short")
        .arg("-C")
        .arg(format!("opt-level={opt_level}"))
        .arg("-C")
        .arg(format!("codegen-units={RUST_CODEGEN_UNITS}"))
        .arg("-o")
        .arg(output);
    command
}

fn optimization_sample(
    rustc: &OsStr,
    source: &Path,
    output: &Path,
    arm: OptimizationArm,
    stdin: &[u8],
    expected_stdout: &[u8],
) -> Duration {
    let mut command = optimization_rustc_command(rustc, source, output, arm.level());
    let (result, elapsed) = timed_output(&mut command);
    assert_success(&result, arm.label());
    run_binary(output, stdin, expected_stdout);
    elapsed
}

fn optimization_relative_mad(samples: &[Duration]) -> f64 {
    let summary = stats(samples);
    if summary.median_ms <= f64::EPSILON {
        return 0.0;
    }

    let mut deviations = samples
        .iter()
        .map(|sample| ((sample.as_secs_f64() * 1_000.0) - summary.median_ms).abs())
        .collect::<Vec<_>>();
    deviations.sort_by(|left, right| left.total_cmp(right));
    let middle = deviations.len() / 2;
    let mad_ms = if deviations.len().is_multiple_of(2) {
        (deviations[middle - 1] + deviations[middle]) / 2.0
    } else {
        deviations[middle]
    };
    mad_ms / summary.median_ms
}

fn optimization_case_verdict(result: &OptimizationCaseResult) -> &'static str {
    let current = stats(&result.current.samples);
    let candidate = stats(&result.candidate.samples);
    let max_relative_mad = optimization_relative_mad(&result.current.samples)
        .max(optimization_relative_mad(&result.candidate.samples));

    if max_relative_mad > OPT_BUILD_MAX_RELATIVE_MAD {
        return "INCONCLUSIVE-NOISY";
    }

    let saved_ms = current.median_ms - candidate.median_ms;
    let improvement = saved_ms / current.median_ms;
    if saved_ms >= OPT_BUILD_MIN_SAVED_MS && improvement >= OPT_BUILD_MIN_IMPROVEMENT {
        "BUILD-GATE-PASS"
    } else {
        "BUILD-GATE-FAIL"
    }
}

fn optimization_aggregate_verdict(results: &[OptimizationCaseResult]) -> &'static str {
    if results
        .iter()
        .any(|result| optimization_case_verdict(result) == "BUILD-GATE-FAIL")
    {
        "REJECT-DEFER"
    } else if results
        .iter()
        .any(|result| optimization_case_verdict(result) == "INCONCLUSIVE-NOISY")
    {
        "INCONCLUSIVE"
    } else {
        "BUILD-GATE-PASS-RUNTIME-REQUIRED"
    }
}

fn write_optimization_report(
    out_dir: &Path,
    results: &[OptimizationCaseResult],
    rustc_version: &str,
) {
    fs::create_dir_all(out_dir).expect("optimization research output directory should be created");
    let git_sha = env::var("EVO_GIT_SHA")
        .or_else(|_| env::var("GITHUB_SHA"))
        .unwrap_or_else(|_| "local".to_owned());
    let host = rustc_host(rustc_version);
    let aggregate = optimization_aggregate_verdict(results);

    let mut markdown = format!(
        "# Release optimization cost research v0\n\n\
- git_sha: `{git_sha}`\n\
- platform: `{}-{}`\n\
- rustc_host: `{host}`\n\
- cases: `{}`\n\
- warmups_per_arm: `{OPT_RESEARCH_WARMUPS}`\n\
- samples_per_arm: `{OPT_RESEARCH_SAMPLES}`\n\
- current_flags: `--edition={RUST_EDITION} --error-format=short -C opt-level={RUST_OPT_LEVEL} -C codegen-units={RUST_CODEGEN_UNITS}`\n\
- candidate_flags: `--edition={RUST_EDITION} --error-format=short -C opt-level={OPT_CANDIDATE_LEVEL} -C codegen-units={RUST_CODEGEN_UNITS}`\n\
- material_build_gate: `>= {:.0}% AND >= {:.1} ms median total compile+link reduction on every initial case`\n\
- max_relative_mad: `{OPT_BUILD_MAX_RELATIVE_MAD:.3}`\n\
- aggregate_verdict: `{aggregate}`\n\
- correctness: `PASS`\n\n\
| Case | Arm | Median ms | Min | Max | P95 | Rel MAD | Binary bytes | Saved vs opt3 ms | Improvement vs opt3 | Case verdict |\n\
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |\n",
        env::consts::OS,
        env::consts::ARCH,
        results.len(),
        OPT_BUILD_MIN_IMPROVEMENT * 100.0,
        OPT_BUILD_MIN_SAVED_MS,
    );

    for result in results {
        let current = stats(&result.current.samples);
        let candidate = stats(&result.candidate.samples);
        let saved_ms = current.median_ms - candidate.median_ms;
        let improvement = saved_ms / current.median_ms;

        for arm in [&result.current, &result.candidate] {
            let summary = stats(&arm.samples);
            let (arm_saved_ms, arm_improvement) = if arm.label == result.candidate.label {
                (saved_ms, improvement)
            } else {
                (0.0, 0.0)
            };
            writeln!(
                markdown,
                "| {} | {} | {:.3} | {:.3} | {:.3} | {:.3} | {:.4} | {} | {:.3} | {:.2}% | {} |",
                result.name,
                arm.label,
                summary.median_ms,
                summary.min_ms,
                summary.max_ms,
                summary.p95_ms,
                optimization_relative_mad(&arm.samples),
                arm.binary_bytes,
                arm_saved_ms,
                arm_improvement * 100.0,
                optimization_case_verdict(result),
            )
            .expect("optimization Markdown table should write");
        }
    }

    markdown.push_str(
        "\nThe decision uses total production-equivalent rustc compile+link wall time. \
The only intended rustc flag difference is `opt-level=3` versus `opt-level=2`; edition, \
codegen units, generated Rust, source path policy, linker defaults and no-incremental behavior \
remain fixed. Every measured binary must emit the exact committed stdout after compilation. \
If either initial case misses the pre-registered material build gate, the candidate stops here \
without spending the seven-case runtime corpus. If both pass, runtime parity against equivalent \
reference Rust and against current opt3 Evolution becomes the next required gate. No production \
optimization setting is changed by this research.\n",
    );
    fs::write(out_dir.join("report.md"), markdown)
        .expect("optimization Markdown report should be written");

    let mut json = format!(
        "{{\n  \"git_sha\": \"{git_sha}\",\n  \"platform\": \"{}-{}\",\n  \"rustc_host\": \"{host}\",\n  \"warmups_per_arm\": {OPT_RESEARCH_WARMUPS},\n  \"samples_per_arm\": {OPT_RESEARCH_SAMPLES},\n  \"current_opt_level\": \"{RUST_OPT_LEVEL}\",\n  \"candidate_opt_level\": \"{OPT_CANDIDATE_LEVEL}\",\n  \"material_improvement_ratio\": {OPT_BUILD_MIN_IMPROVEMENT:.6},\n  \"material_saved_ms\": {OPT_BUILD_MIN_SAVED_MS:.3},\n  \"max_relative_mad\": {OPT_BUILD_MAX_RELATIVE_MAD:.6},\n  \"aggregate_verdict\": \"{aggregate}\",\n  \"correctness\": \"PASS\",\n  \"cases\": [\n",
        env::consts::OS,
        env::consts::ARCH,
    );

    for (index, result) in results.iter().enumerate() {
        let current = stats(&result.current.samples);
        let candidate = stats(&result.candidate.samples);
        let saved_ms = current.median_ms - candidate.median_ms;
        let improvement = saved_ms / current.median_ms;
        let comma = if index + 1 == results.len() { "" } else { "," };

        writeln!(
            json,
            "    {{\n      \"name\": \"{}\",\n      \"verdict\": \"{}\",\n      \"current_samples_ms\": [{}],\n      \"candidate_samples_ms\": [{}],\n      \"current_median_ms\": {:.3},\n      \"candidate_median_ms\": {:.3},\n      \"saved_ms\": {:.3},\n      \"improvement_ratio\": {:.9},\n      \"current_relative_mad\": {:.9},\n      \"candidate_relative_mad\": {:.9},\n      \"current_binary_bytes\": {},\n      \"candidate_binary_bytes\": {}\n    }}{comma}",
            result.name,
            optimization_case_verdict(result),
            samples_json(&result.current.samples),
            samples_json(&result.candidate.samples),
            current.median_ms,
            candidate.median_ms,
            saved_ms,
            improvement,
            optimization_relative_mad(&result.current.samples),
            optimization_relative_mad(&result.candidate.samples),
            result.current.binary_bytes,
            result.candidate.binary_bytes,
        )
        .expect("optimization JSON case should write");
    }
    json.push_str("  ]\n}\n");
    fs::write(out_dir.join("report.json"), json)
        .expect("optimization JSON report should be written");

    let mut csv = String::from("case,arm,index,elapsed_ms\n");
    for result in results {
        for arm in [&result.current, &result.candidate] {
            for (index, sample) in arm.samples.iter().enumerate() {
                writeln!(
                    csv,
                    "{},{},{},{:.3}",
                    result.name,
                    arm.label,
                    index + 1,
                    sample.as_secs_f64() * 1_000.0
                )
                .expect("optimization CSV row should write");
            }
        }
    }
    fs::write(out_dir.join("raw-samples.csv"), csv)
        .expect("optimization CSV report should be written");
    fs::write(out_dir.join("rustc-vV.txt"), rustc_version)
        .expect("optimization rustc version evidence should be written");

    for result in results {
        let case_dir = out_dir.join(&result.name);
        fs::create_dir_all(&case_dir).expect("optimization case output directory should create");
        fs::write(case_dir.join("generated.rs"), &result.generated_rust)
            .expect("optimization generated Rust should be written");
    }
}

#[test]
#[ignore = "run explicitly on controlled Ubuntu for release optimization research"]
fn release_optimization_cost_compares_opt3_and_opt2_builds() {
    assert_eq!(
        env::consts::OS,
        "linux",
        "release optimization research is Ubuntu-only"
    );

    let root = repo_root().join("benchmarks/cases");
    let dir = temp_dir("release-optimization-cost");
    fs::create_dir_all(&dir).expect("optimization research directory should be created");
    let rustc = real_rustc();
    let version = rustc_version(&rustc);
    assert_eq!(
        rustc_host(&version),
        "x86_64-unknown-linux-gnu",
        "optimization research expects the controlled GNU Linux host"
    );

    let mut results = Vec::with_capacity(OPT_RESEARCH_CASES.len());

    for case_name in OPT_RESEARCH_CASES {
        let fixture = root.join(case_name);
        let source_bytes = fs::read(fixture.join("evolution.evo"))
            .expect("optimization fixture source should read");
        let fixture_stdin =
            fs::read(fixture.join("stdin.bin")).expect("optimization fixture stdin should read");
        let expected_stdout = fs::read(fixture.join("expected.stdout"))
            .expect("optimization fixture expected stdout should read");

        let case_dir = dir.join(case_name);
        fs::create_dir_all(&case_dir).expect("optimization case directory should create");
        let evo_path = case_dir.join("program.evo");
        fs::write(&evo_path, source_bytes).expect("optimization Evolution source should stage");
        let emitted = evo_command("emit-rust", &evo_path)
            .output()
            .expect("optimization emit-rust should execute");
        assert_success(&emitted, "optimization emit-rust");
        assert!(
            !emitted.stdout.is_empty(),
            "optimization generated Rust should not be empty"
        );

        let generated = case_dir.join("main.rs");
        fs::write(&generated, &emitted.stdout).expect("optimization generated Rust should stage");
        let current_binary = case_dir.join(format!("opt3-current{}", env::consts::EXE_SUFFIX));
        let candidate_binary = case_dir.join(format!("opt2-candidate{}", env::consts::EXE_SUFFIX));

        for warmup in 0..OPT_RESEARCH_WARMUPS {
            for arm in optimization_order(warmup) {
                let output = match arm {
                    OptimizationArm::Current => &current_binary,
                    OptimizationArm::Candidate => &candidate_binary,
                };
                let _ = optimization_sample(
                    &rustc,
                    &generated,
                    output,
                    arm,
                    &fixture_stdin,
                    &expected_stdout,
                );
            }
        }

        let mut current_samples = Vec::with_capacity(OPT_RESEARCH_SAMPLES);
        let mut candidate_samples = Vec::with_capacity(OPT_RESEARCH_SAMPLES);
        for sample in 0..OPT_RESEARCH_SAMPLES {
            for arm in optimization_order(sample + OPT_RESEARCH_WARMUPS) {
                let output = match arm {
                    OptimizationArm::Current => &current_binary,
                    OptimizationArm::Candidate => &candidate_binary,
                };
                let elapsed = optimization_sample(
                    &rustc,
                    &generated,
                    output,
                    arm,
                    &fixture_stdin,
                    &expected_stdout,
                );
                match arm {
                    OptimizationArm::Current => current_samples.push(elapsed),
                    OptimizationArm::Candidate => candidate_samples.push(elapsed),
                }
            }
        }

        results.push(OptimizationCaseResult {
            name: case_name.to_owned(),
            generated_rust: emitted.stdout,
            current: OptimizationArmResult {
                label: OptimizationArm::Current.label(),
                samples: current_samples,
                binary_bytes: fs::metadata(&current_binary)
                    .expect("current optimization binary metadata should read")
                    .len(),
            },
            candidate: OptimizationArmResult {
                label: OptimizationArm::Candidate.label(),
                samples: candidate_samples,
                binary_bytes: fs::metadata(&candidate_binary)
                    .expect("candidate optimization binary metadata should read")
                    .len(),
            },
        });
    }

    let out_dir = env::var_os("EVO_RELEASE_OPT_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| dir.join("report"));
    write_optimization_report(&out_dir, &results, &version);

    println!(
        "release_optimization_research_verdict={}",
        optimization_aggregate_verdict(&results)
    );
    for result in &results {
        let current = stats(&result.current.samples);
        let candidate = stats(&result.candidate.samples);
        println!(
            "release_optimization_case={} verdict={} opt3_ms={:.3} opt2_ms={:.3} opt2_vs_opt3={:.6} saved_ms={:.3}",
            result.name,
            optimization_case_verdict(result),
            current.median_ms,
            candidate.median_ms,
            candidate.median_ms / current.median_ms,
            current.median_ms - candidate.median_ms,
        );
    }

    let _ = fs::remove_dir_all(dir);
}
