include!("link_time_research.rs");

const LINKER_CANDIDATE_SAMPLES: usize = 9;
const LINKER_CANDIDATE_WARMUPS: usize = 2;
const LINKER_CANDIDATE_CASES: [&str; 2] = ["enums-v0", "logical-operators-v0"];
const EXPECTED_MOLD_PACKAGE_VERSION: &str = "2.30.0+dfsg-1build1";
const MATERIAL_TOTAL_IMPROVEMENT: f64 = 0.05;
const MATERIAL_TOTAL_MS: f64 = 5.0;
const MAX_RELATIVE_MAD: f64 = 0.10;

#[derive(Clone, Copy)]
enum CandidateMode {
    BaselineLld,
    SystemLd,
    Mold,
}

impl CandidateMode {
    fn label(self) -> &'static str {
        match self {
            Self::BaselineLld => "baseline-lld",
            Self::SystemLd => "system-ld-control",
            Self::Mold => "mold-candidate",
        }
    }

    fn configure(self, command: &mut Command) {
        match self {
            Self::BaselineLld => {}
            Self::SystemLd => {
                command
                    .arg("-C")
                    .arg("linker-features=-lld")
                    .arg("-C")
                    .arg("link-self-contained=-linker");
            }
            Self::Mold => {
                command
                    .arg("-C")
                    .arg("linker-features=-lld")
                    .arg("-C")
                    .arg("link-self-contained=-linker")
                    .arg("-C")
                    .arg("link-arg=-fuse-ld=mold");
            }
        }
    }
}

struct SampleOutcome {
    binary: PathBuf,
    full: Duration,
    link: Duration,
    linker_invocations: u64,
    wrapper_args: String,
}

struct ArmSamples {
    full: Vec<Duration>,
    link: Vec<Duration>,
    linker_invocations: Vec<u64>,
    wrapper_args: String,
    final_binary: Option<PathBuf>,
}

impl ArmSamples {
    fn new() -> Self {
        Self {
            full: Vec::with_capacity(LINKER_CANDIDATE_SAMPLES),
            link: Vec::with_capacity(LINKER_CANDIDATE_SAMPLES),
            linker_invocations: Vec::with_capacity(LINKER_CANDIDATE_SAMPLES),
            wrapper_args: String::new(),
            final_binary: None,
        }
    }

    fn push(&mut self, outcome: SampleOutcome) {
        let sample_number = self.full.len() + 1;
        writeln!(
            self.wrapper_args,
            "# sample {sample_number}\n{}",
            outcome.wrapper_args
        )
        .expect("wrapper args aggregate should write");
        self.full.push(outcome.full);
        self.link.push(outcome.link);
        self.linker_invocations.push(outcome.linker_invocations);
        self.final_binary = Some(outcome.binary);
    }
}

struct ArmResult {
    label: &'static str,
    full: Vec<Duration>,
    link: Vec<Duration>,
    linker_invocations: Vec<u64>,
    binary_bytes: u64,
    needed_libraries: Vec<String>,
    print_link_args: Vec<u8>,
    wrapper_args: String,
}

struct CandidateCaseResult {
    name: String,
    generated_rust: Vec<u8>,
    baseline: ArmResult,
    system: ArmResult,
    mold: ArmResult,
}

impl CandidateCaseResult {
    fn arms(&self) -> [&ArmResult; 3] {
        [&self.baseline, &self.system, &self.mold]
    }
}

fn candidate_rustc_command(rustc: &OsStr, generated: &Path, mode: CandidateMode) -> Command {
    let mut command = production_rustc_command(rustc, generated);
    mode.configure(&mut command);
    command
}

fn rotated_modes(index: usize) -> [CandidateMode; 3] {
    match index % 3 {
        0 => [
            CandidateMode::BaselineLld,
            CandidateMode::SystemLd,
            CandidateMode::Mold,
        ],
        1 => [
            CandidateMode::SystemLd,
            CandidateMode::Mold,
            CandidateMode::BaselineLld,
        ],
        _ => [
            CandidateMode::Mold,
            CandidateMode::BaselineLld,
            CandidateMode::SystemLd,
        ],
    }
}

fn relative_mad(samples: &[Duration]) -> f64 {
    let median = stats(samples).median_ms;
    if median <= f64::EPSILON {
        return 0.0;
    }
    let mut deviations = samples
        .iter()
        .map(|sample| ((sample.as_secs_f64() * 1_000.0) - median).abs())
        .collect::<Vec<_>>();
    deviations.sort_by(|left, right| left.total_cmp(right));
    let middle = deviations.len() / 2;
    let mad = if deviations.len().is_multiple_of(2) {
        (deviations[middle - 1] + deviations[middle]) / 2.0
    } else {
        deviations[middle]
    };
    mad / median
}

fn tool_version(program: &OsStr) -> Vec<u8> {
    let output = Command::new(program)
        .arg("--version")
        .output()
        .expect("tool version probe should execute");
    assert_success(&output, "tool version probe");
    let mut bytes = output.stdout;
    bytes.extend_from_slice(&output.stderr);
    bytes
}

fn mold_package_version() -> String {
    let output = Command::new("dpkg-query")
        .arg("-W")
        .arg("-f=${Version}")
        .arg("mold")
        .output()
        .expect("dpkg-query mold should execute");
    assert_success(&output, "dpkg-query mold");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn needed_libraries(binary: &Path) -> Vec<String> {
    let output = Command::new("readelf")
        .arg("-d")
        .arg(binary)
        .output()
        .expect("readelf should execute");
    assert_success(&output, "readelf dynamic section");
    let text = String::from_utf8_lossy(&output.stdout);
    let mut libraries = text
        .lines()
        .filter(|line| line.contains("(NEEDED)"))
        .filter_map(|line| {
            let start = line.find('[')? + 1;
            let end = line[start..].find(']')? + start;
            Some(line[start..end].to_owned())
        })
        .collect::<Vec<_>>();
    libraries.sort();
    libraries.dedup();
    libraries
}

fn probe_link_args(
    rustc: &OsStr,
    generated: &Path,
    case_dir: &Path,
    mode: CandidateMode,
    stdin: &[u8],
    expected_stdout: &[u8],
) -> Vec<u8> {
    let binary = case_dir.join(format!("probe-{}{}", mode.label(), env::consts::EXE_SUFFIX));
    let mut command = candidate_rustc_command(rustc, generated, mode);
    let output = command
        .arg("--print")
        .arg("link-args")
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("rustc link-args probe should execute");
    assert_success(&output, "rustc --print link-args candidate probe");
    run_binary(&binary, stdin, expected_stdout);
    let mut bytes = output.stdout;
    bytes.extend_from_slice(&output.stderr);
    bytes
}

fn assert_probe_contract(mode: CandidateMode, probe: &[u8]) {
    let text = String::from_utf8_lossy(probe);
    match mode {
        CandidateMode::BaselineLld => assert!(
            text.contains("-fuse-ld=lld"),
            "baseline should retain Rust 1.98 default lld request"
        ),
        CandidateMode::SystemLd => {
            assert!(
                !text.contains("-fuse-ld=lld"),
                "system control must opt out of the default lld request"
            );
            assert!(
                !text.contains("-fuse-ld=mold"),
                "system control must not accidentally select mold"
            );
        }
        CandidateMode::Mold => {
            assert!(
                !text.contains("-fuse-ld=lld"),
                "mold candidate must disable the default lld request"
            );
            assert!(
                text.contains("-fuse-ld=mold"),
                "mold candidate must actually request mold"
            );
        }
    }
}

struct InstrumentedContext<'a> {
    rustc: &'a OsStr,
    generated: &'a Path,
    case_dir: &'a Path,
    instrumented_path: &'a OsStr,
    real_cc: &'a Path,
    stdin: &'a [u8],
    expected_stdout: &'a [u8],
}

fn run_instrumented_sample(
    context: &InstrumentedContext<'_>,
    mode: CandidateMode,
    tag: &str,
) -> SampleOutcome {
    let binary =
        context
            .case_dir
            .join(format!("{}-{tag}{}", mode.label(), env::consts::EXE_SUFFIX));
    let timing_file = context
        .case_dir
        .join(format!("{}-{tag}-link-timing.txt", mode.label()));
    let args_file = context
        .case_dir
        .join(format!("{}-{tag}-link-args.txt", mode.label()));

    let mut command = candidate_rustc_command(context.rustc, context.generated, mode);
    command
        .arg("-o")
        .arg(&binary)
        .env("PATH", context.instrumented_path)
        .env("EVO_TEST_REAL_LINKER", context.real_cc)
        .env("EVO_TEST_LINK_TIMINGS", &timing_file)
        .env("EVO_TEST_LINK_ARGS", &args_file);
    let (output, full) = timed_output(&mut command);
    assert_success(&output, "instrumented linker candidate compile");
    run_binary(&binary, context.stdin, context.expected_stdout);
    let (link, linker_invocations) = read_link_child_sample(&timing_file);
    assert_eq!(
        linker_invocations, 1,
        "single-file candidate compile should invoke the linker driver exactly once"
    );
    let wrapper_args =
        fs::read_to_string(args_file).expect("candidate wrapper linker args should read");

    SampleOutcome {
        binary,
        full,
        link,
        linker_invocations,
        wrapper_args,
    }
}

fn finish_arm(mode: CandidateMode, samples: ArmSamples, print_link_args: Vec<u8>) -> ArmResult {
    let final_binary = samples
        .final_binary
        .expect("measured arm should retain a final binary");
    ArmResult {
        label: mode.label(),
        full: samples.full,
        link: samples.link,
        linker_invocations: samples.linker_invocations,
        binary_bytes: fs::metadata(&final_binary)
            .expect("candidate binary metadata should read")
            .len(),
        needed_libraries: needed_libraries(&final_binary),
        print_link_args,
        wrapper_args: samples.wrapper_args,
    }
}

fn arm_is_stable(arm: &ArmResult) -> bool {
    relative_mad(&arm.full) <= MAX_RELATIVE_MAD && relative_mad(&arm.link) <= MAX_RELATIVE_MAD
}

fn case_verdict(result: &CandidateCaseResult) -> &'static str {
    if result.baseline.needed_libraries != result.mold.needed_libraries {
        return "MOLD-DEPLOYMENT-MISMATCH";
    }
    if !arm_is_stable(&result.baseline) || !arm_is_stable(&result.mold) {
        return "INCONCLUSIVE-NOISY";
    }

    let baseline = stats(&result.baseline.full).median_ms;
    let mold = stats(&result.mold.full).median_ms;
    let saved_ms = baseline - mold;
    let improvement = saved_ms / baseline;
    if saved_ms >= MATERIAL_TOTAL_MS && improvement >= MATERIAL_TOTAL_IMPROVEMENT {
        "MOLD-MATERIAL-WIN"
    } else if mold < baseline {
        "MOLD-NONMATERIAL-WIN"
    } else {
        "MOLD-NO-WIN"
    }
}

fn aggregate_verdict(results: &[CandidateCaseResult]) -> &'static str {
    if results
        .iter()
        .any(|result| case_verdict(result) == "MOLD-DEPLOYMENT-MISMATCH")
    {
        "MOLD-REJECT"
    } else if results
        .iter()
        .any(|result| case_verdict(result) == "INCONCLUSIVE-NOISY")
    {
        "INCONCLUSIVE"
    } else if results
        .iter()
        .all(|result| case_verdict(result) == "MOLD-MATERIAL-WIN")
    {
        "MOLD-FOLLOW-UP"
    } else if results
        .iter()
        .any(|result| case_verdict(result) == "MOLD-NO-WIN")
    {
        "MOLD-REJECT-OR-DEFER"
    } else {
        "MOLD-NONMATERIAL-DEFER"
    }
}

fn json_string_list(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("\"{value}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

struct ToolEvidence<'a> {
    rustc_version: &'a str,
    cc_version: &'a [u8],
    ld_version: &'a [u8],
    mold_version: &'a [u8],
    mold_package_version: &'a str,
}

fn write_candidate_report(
    out_dir: &Path,
    results: &[CandidateCaseResult],
    tools: &ToolEvidence<'_>,
) {
    fs::create_dir_all(out_dir).expect("linker candidate output directory should be created");
    let git_sha = env::var("EVO_GIT_SHA")
        .or_else(|_| env::var("GITHUB_SHA"))
        .unwrap_or_else(|_| "local".to_owned());
    let aggregate = aggregate_verdict(results);
    let host = rustc_host(tools.rustc_version);

    let mut markdown = format!(
        "# Linker candidate research v0\n\n\
- git_sha: `{git_sha}`\n\
- platform: `{}-{}`\n\
- rustc_host: `{host}`\n\
- samples_per_arm: `{LINKER_CANDIDATE_SAMPLES}`\n\
- warmups_per_arm: `{LINKER_CANDIDATE_WARMUPS}`\n\
- cases: `{}`\n\
- baseline: `Rust 1.98 default cc -> lld`\n\
- system_control: `-C linker-features=-lld -C link-self-contained=-linker`\n\
- mold_candidate: `system control + -C link-arg=-fuse-ld=mold`\n\
- mold_package: `{}`\n\
- material_total_gate: `>= {:.0}% AND >= {:.1} ms median full-build reduction on every initial case`\n\
- max_relative_mad: `{MAX_RELATIVE_MAD:.3}`\n\
- aggregate_verdict: `{aggregate}`\n\
- correctness: `PASS`\n\n\
| Case | Arm | Full median ms | Min | Max | P95 | Full rel MAD | Link median ms | Link/full | Link rel MAD | Invocations | Binary bytes | NEEDED | Case verdict |\n\
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | ---: | --- | --- |\n",
        env::consts::OS,
        env::consts::ARCH,
        results.len(),
        tools.mold_package_version,
        MATERIAL_TOTAL_IMPROVEMENT * 100.0,
        MATERIAL_TOTAL_MS,
    );

    for result in results {
        for arm in result.arms() {
            let full = stats(&arm.full);
            let link = stats(&arm.link);
            let invocation_range = format!(
                "{}-{}",
                arm.linker_invocations.iter().copied().min().unwrap_or(0),
                arm.linker_invocations.iter().copied().max().unwrap_or(0)
            );
            writeln!(
                markdown,
                "| {} | {} | {:.3} | {:.3} | {:.3} | {:.3} | {:.4} | {:.3} | {:.2}% | {:.4} | {} | {} | {} | {} |",
                result.name,
                arm.label,
                full.median_ms,
                full.min_ms,
                full.max_ms,
                full.p95_ms,
                relative_mad(&arm.full),
                link.median_ms,
                link.median_ms / full.median_ms * 100.0,
                relative_mad(&arm.link),
                invocation_range,
                arm.binary_bytes,
                arm.needed_libraries.join(";"),
                case_verdict(result),
            )
            .expect("candidate Markdown table should write");
        }
    }

    markdown.push_str(
        "\nThe decision uses total production-equivalent rustc wall time; linker-child time is supporting attribution. The system-ld arm is a control, not a production proposal. `mold` provisioning occurs before measurement and is not counted as build-time improvement. Every measured binary must pass committed stdin/stdout correctness. `DT_NEEDED` names are compared between the current lld baseline and mold; a mismatch rejects the candidate before runtime follow-up. No production linker setting is changed.\n",
    );
    fs::write(out_dir.join("report.md"), markdown).expect("candidate Markdown report should write");

    let mut json = format!(
        "{{\n  \"git_sha\": \"{git_sha}\",\n  \"platform\": \"{}-{}\",\n  \"rustc_host\": \"{host}\",\n  \"mold_package_version\": \"{}\",\n  \"samples_per_arm\": {LINKER_CANDIDATE_SAMPLES},\n  \"warmups_per_arm\": {LINKER_CANDIDATE_WARMUPS},\n  \"aggregate_verdict\": \"{aggregate}\",\n  \"correctness\": \"PASS\",\n  \"cases\": [\n",
        env::consts::OS,
        env::consts::ARCH,
        tools.mold_package_version,
    );
    for (case_index, result) in results.iter().enumerate() {
        let case_comma = if case_index + 1 == results.len() {
            ""
        } else {
            ","
        };
        writeln!(
            json,
            "    {{\n      \"name\": \"{}\",\n      \"verdict\": \"{}\",\n      \"arms\": [",
            result.name,
            case_verdict(result)
        )
        .expect("candidate JSON case should write");
        let arms = result.arms();
        for (arm_index, arm) in arms.iter().enumerate() {
            let full = stats(&arm.full);
            let link = stats(&arm.link);
            let arm_comma = if arm_index + 1 == arms.len() { "" } else { "," };
            writeln!(
                json,
                "        {{\"name\": \"{}\", \"full_samples_ms\": [{}], \"link_samples_ms\": [{}], \"full_median_ms\": {:.3}, \"link_median_ms\": {:.3}, \"full_relative_mad\": {:.9}, \"link_relative_mad\": {:.9}, \"binary_bytes\": {}, \"needed_libraries\": [{}], \"linker_invocations\": [{}]}}{arm_comma}",
                arm.label,
                samples_json(&arm.full),
                samples_json(&arm.link),
                full.median_ms,
                link.median_ms,
                relative_mad(&arm.full),
                relative_mad(&arm.link),
                arm.binary_bytes,
                json_string_list(&arm.needed_libraries),
                arm.linker_invocations
                    .iter()
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join(", "),
            )
            .expect("candidate JSON arm should write");
        }
        writeln!(json, "      ]\n    }}{case_comma}")
            .expect("candidate JSON case close should write");
    }
    json.push_str("  ]\n}\n");
    fs::write(out_dir.join("report.json"), json).expect("candidate JSON report should write");

    let mut csv = String::from("case,arm,index,full_ms,link_ms,linker_invocations\n");
    for result in results {
        for arm in result.arms() {
            for (index, (full, link)) in arm.full.iter().zip(&arm.link).enumerate() {
                writeln!(
                    csv,
                    "{},{},{},{:.3},{:.3},{}",
                    result.name,
                    arm.label,
                    index + 1,
                    full.as_secs_f64() * 1_000.0,
                    link.as_secs_f64() * 1_000.0,
                    arm.linker_invocations[index]
                )
                .expect("candidate CSV row should write");
            }
        }
    }
    fs::write(out_dir.join("raw-samples.csv"), csv).expect("candidate CSV should write");
    fs::write(out_dir.join("rustc-vV.txt"), tools.rustc_version)
        .expect("rustc evidence should write");
    fs::write(out_dir.join("cc-version.txt"), tools.cc_version).expect("cc evidence should write");
    fs::write(out_dir.join("ld-version.txt"), tools.ld_version).expect("ld evidence should write");
    fs::write(out_dir.join("mold-version.txt"), tools.mold_version)
        .expect("mold evidence should write");
    fs::write(
        out_dir.join("mold-package-version.txt"),
        tools.mold_package_version,
    )
    .expect("mold package evidence should write");

    for result in results {
        let case_dir = out_dir.join(&result.name);
        fs::create_dir_all(&case_dir).expect("candidate case evidence directory should create");
        fs::write(case_dir.join("generated.rs"), &result.generated_rust)
            .expect("candidate generated Rust evidence should write");
        for arm in result.arms() {
            fs::write(
                case_dir.join(format!("{}-rustc-print-link-args.txt", arm.label)),
                &arm.print_link_args,
            )
            .expect("candidate print link args should write");
            fs::write(
                case_dir.join(format!("{}-wrapper-link-args.txt", arm.label)),
                &arm.wrapper_args,
            )
            .expect("candidate wrapper args should write");
            fs::write(
                case_dir.join(format!("{}-needed-libraries.txt", arm.label)),
                arm.needed_libraries.join("\n"),
            )
            .expect("candidate needed libraries should write");
        }
    }
}

#[test]
#[ignore = "run explicitly on the controlled Ubuntu 24.04 linker-candidate runner"]
fn linker_candidate_research_compares_current_lld_system_ld_and_mold() {
    assert_eq!(
        env::consts::OS,
        "linux",
        "linker candidate experiment is Ubuntu-only"
    );

    let dir = temp_dir("linker-candidate-research");
    fs::create_dir_all(&dir).expect("candidate research directory should create");
    let rustc = real_rustc();
    let rustc_version = rustc_version(&rustc);
    assert_eq!(
        rustc_host(&rustc_version),
        "x86_64-unknown-linux-gnu",
        "candidate experiment expects the controlled GNU Linux host"
    );

    let real_cc = resolve_from_path("cc");
    let wrapper = compile_linker_wrapper(&dir, &rustc);
    let wrapper_dir = wrapper
        .parent()
        .expect("linker wrapper should have a parent")
        .to_path_buf();
    let instrumented_path = prefixed_path(&wrapper_dir);

    let mold_package_version = mold_package_version();
    assert_eq!(
        mold_package_version, EXPECTED_MOLD_PACKAGE_VERSION,
        "controlled candidate runner must use the pinned Noble mold package"
    );
    let cc_version = tool_version(real_cc.as_os_str());
    let ld_version = tool_version(OsStr::new("ld"));
    let mold_version = tool_version(OsStr::new("mold"));

    let mut results = Vec::with_capacity(LINKER_CANDIDATE_CASES.len());
    for case_name in LINKER_CANDIDATE_CASES {
        let fixture = repo_root().join("benchmarks/cases").join(case_name);
        let source_bytes =
            fs::read(fixture.join("evolution.evo")).expect("candidate fixture source should read");
        let fixture_stdin =
            fs::read(fixture.join("stdin.bin")).expect("candidate stdin should read");
        let expected_stdout = fs::read(fixture.join("expected.stdout"))
            .expect("candidate expected stdout should read");
        let case_dir = dir.join(case_name);
        fs::create_dir_all(&case_dir).expect("candidate case directory should create");
        let evo_path = case_dir.join("program.evo");
        fs::write(&evo_path, source_bytes).expect("candidate Evolution source should stage");
        let emitted = evo_command("emit-rust", &evo_path)
            .output()
            .expect("candidate emit-rust should execute");
        assert_success(&emitted, "candidate emit-rust");
        assert!(
            !emitted.stdout.is_empty(),
            "generated Rust should not be empty"
        );
        let generated = case_dir.join("main.rs");
        fs::write(&generated, &emitted.stdout).expect("candidate generated Rust should stage");

        let baseline_probe = probe_link_args(
            &rustc,
            &generated,
            &case_dir,
            CandidateMode::BaselineLld,
            &fixture_stdin,
            &expected_stdout,
        );
        let system_probe = probe_link_args(
            &rustc,
            &generated,
            &case_dir,
            CandidateMode::SystemLd,
            &fixture_stdin,
            &expected_stdout,
        );
        let mold_probe = probe_link_args(
            &rustc,
            &generated,
            &case_dir,
            CandidateMode::Mold,
            &fixture_stdin,
            &expected_stdout,
        );
        assert_probe_contract(CandidateMode::BaselineLld, &baseline_probe);
        assert_probe_contract(CandidateMode::SystemLd, &system_probe);
        assert_probe_contract(CandidateMode::Mold, &mold_probe);

        let context = InstrumentedContext {
            rustc: &rustc,
            generated: &generated,
            case_dir: &case_dir,
            instrumented_path: instrumented_path.as_os_str(),
            real_cc: &real_cc,
            stdin: &fixture_stdin,
            expected_stdout: &expected_stdout,
        };

        for warmup in 0..LINKER_CANDIDATE_WARMUPS {
            for mode in rotated_modes(warmup) {
                let tag = format!("warmup-{}", warmup + 1);
                let _ = run_instrumented_sample(&context, mode, &tag);
            }
        }

        let mut baseline = ArmSamples::new();
        let mut system = ArmSamples::new();
        let mut mold = ArmSamples::new();
        for sample in 0..LINKER_CANDIDATE_SAMPLES {
            for mode in rotated_modes(sample) {
                let tag = format!("sample-{}", sample + 1);
                let outcome = run_instrumented_sample(&context, mode, &tag);
                match mode {
                    CandidateMode::BaselineLld => baseline.push(outcome),
                    CandidateMode::SystemLd => system.push(outcome),
                    CandidateMode::Mold => mold.push(outcome),
                }
            }
        }

        results.push(CandidateCaseResult {
            name: case_name.to_owned(),
            generated_rust: emitted.stdout,
            baseline: finish_arm(CandidateMode::BaselineLld, baseline, baseline_probe),
            system: finish_arm(CandidateMode::SystemLd, system, system_probe),
            mold: finish_arm(CandidateMode::Mold, mold, mold_probe),
        });
    }

    let out_dir = env::var_os("EVO_LINKER_CANDIDATE_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| dir.join("report"));
    let tools = ToolEvidence {
        rustc_version: &rustc_version,
        cc_version: &cc_version,
        ld_version: &ld_version,
        mold_version: &mold_version,
        mold_package_version: &mold_package_version,
    };
    write_candidate_report(&out_dir, &results, &tools);

    println!(
        "linker_candidate_research_verdict={}",
        aggregate_verdict(&results)
    );
    for result in &results {
        let baseline = stats(&result.baseline.full);
        let system = stats(&result.system.full);
        let mold = stats(&result.mold.full);
        println!(
            "linker_candidate_case={} verdict={} baseline_ms={:.3} system_ld_ms={:.3} mold_ms={:.3} mold_vs_baseline={:.6}",
            result.name,
            case_verdict(result),
            baseline.median_ms,
            system.median_ms,
            mold.median_ms,
            mold.median_ms / baseline.median_ms,
        );
    }

    let _ = fs::remove_dir_all(dir);
}
