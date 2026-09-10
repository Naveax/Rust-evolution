#!/usr/bin/env python3
from __future__ import annotations

import csv
import json
import os
import pathlib
import statistics
import subprocess
import sys
import tempfile

SAMPLES = 5
WARMUPS = 1
CASES = ("enums-v0", "logical-operators-v0")
RUSTC_FLAGS = (
    "--edition=2024",
    "--error-format=short",
    "-C",
    "opt-level=3",
    "-C",
    "codegen-units=1",
)
TIME = "/usr/bin/time"


def run_checked(cmd: list[str], *, stdin: bytes | None = None) -> subprocess.CompletedProcess[bytes]:
    proc = subprocess.run(cmd, input=stdin, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    if proc.returncode != 0:
        raise RuntimeError(
            f"command failed ({proc.returncode}): {' '.join(cmd)}\n"
            f"stdout={proc.stdout.decode(errors='replace')}\n"
            f"stderr={proc.stderr.decode(errors='replace')}"
        )
    return proc


def measure(cmd: list[str], raw_path: pathlib.Path, *, stdin: bytes | None = None) -> tuple[int, float, bytes]:
    raw_path.parent.mkdir(parents=True, exist_ok=True)
    wrapped = [TIME, "-f", "%M\t%e", "-o", str(raw_path), "--", *cmd]
    proc = run_checked(wrapped, stdin=stdin)
    line = raw_path.read_text(encoding="utf-8").strip().splitlines()[-1]
    rss_text, elapsed_text = line.split("\t", 1)
    return int(rss_text), float(elapsed_text), proc.stdout


def median(values: list[int | float]) -> float:
    return float(statistics.median(values))


def arm_summary(rows: list[dict[str, object]]) -> dict[str, float | int]:
    rss = [int(row["peak_rss_kib"]) for row in rows]
    elapsed = [float(row["elapsed_s"]) for row in rows]
    return {
        "samples": len(rows),
        "peak_rss_median_kib": median(rss),
        "peak_rss_min_kib": min(rss),
        "peak_rss_max_kib": max(rss),
        "elapsed_median_s": median(elapsed),
    }


def main() -> int:
    root = pathlib.Path(__file__).resolve().parents[1]
    out = pathlib.Path(os.environ.get("EVO_COMPILE_MEMORY_OUT", root / "target/evo-compile-memory-research"))
    evo = pathlib.Path(os.environ.get("EVO_BIN", root / "target/debug/evo"))
    rustc = os.environ.get("RUSTC", "rustc")
    git_sha = os.environ.get("EVO_GIT_SHA", os.environ.get("GITHUB_SHA", "local"))

    if sys.platform != "linux":
        raise RuntimeError("compile-memory research is intentionally Linux-only")
    if not pathlib.Path(TIME).is_file():
        raise RuntimeError("/usr/bin/time is required")
    if not evo.is_file():
        raise RuntimeError(f"evo binary not found: {evo}")

    out.mkdir(parents=True, exist_ok=True)
    (out / "rustc-vV.txt").write_bytes(run_checked([rustc, "-vV"]).stdout)
    time_version = run_checked([TIME, "--version"]).stdout
    (out / "time-version.txt").write_bytes(time_version)
    (out / "methodology.txt").write_text(
        "Accepted primary metric: GNU /usr/bin/time maximum resident set size (%M), in KiB, "
        "for the directly measured command process. This is process peak RSS, not concurrent "
        "whole-process-tree peak RSS. Full evo build whole-tree peak is intentionally unavailable "
        "in this slice until a separate process-tree/cgroup method is proven.\n",
        encoding="utf-8",
    )

    all_rows: list[dict[str, object]] = []
    summaries: dict[str, object] = {}

    with tempfile.TemporaryDirectory(prefix="evo-compile-memory-") as tmp_text:
        tmp = pathlib.Path(tmp_text)
        for case in CASES:
            case_dir = root / "benchmarks/cases" / case
            source = case_dir / "evolution.evo"
            stdin = (case_dir / "stdin.bin").read_bytes()
            expected = (case_dir / "expected.stdout").read_bytes()

            emitted = run_checked([str(evo), "emit-rust", str(source)]).stdout
            generated_path = out / f"generated-{case}.rs"
            generated_path.write_bytes(emitted)

            arms: dict[str, list[dict[str, object]]] = {"check": [], "emit-rust": [], "direct-rustc": []}

            for warmup in range(WARMUPS):
                run_checked([str(evo), "check", str(source)])
                run_checked([str(evo), "emit-rust", str(source)])
                warm_binary = tmp / f"{case}-warm-{warmup}"
                run_checked([rustc, str(generated_path), *RUSTC_FLAGS, "-o", str(warm_binary)])
                run_output = run_checked([str(warm_binary)], stdin=stdin).stdout
                if run_output != expected:
                    raise RuntimeError(f"warmup correctness failed for {case}")

            for index in range(1, SAMPLES + 1):
                rss, elapsed, stdout = measure(
                    [str(evo), "check", str(source)],
                    out / "raw" / f"{case}-check-{index}.txt",
                )
                if stdout != b"ok\n":
                    raise RuntimeError(f"unexpected check stdout for {case}: {stdout!r}")
                row = {"case": case, "arm": "check", "sample": index, "peak_rss_kib": rss, "elapsed_s": elapsed}
                arms["check"].append(row)
                all_rows.append(row)

                rss, elapsed, stdout = measure(
                    [str(evo), "emit-rust", str(source)],
                    out / "raw" / f"{case}-emit-rust-{index}.txt",
                )
                if stdout != emitted:
                    raise RuntimeError(f"emit-rust bytes changed during measurement for {case}")
                row = {"case": case, "arm": "emit-rust", "sample": index, "peak_rss_kib": rss, "elapsed_s": elapsed}
                arms["emit-rust"].append(row)
                all_rows.append(row)

                binary = tmp / f"{case}-direct-{index}"
                rss, elapsed, _ = measure(
                    [rustc, str(generated_path), *RUSTC_FLAGS, "-o", str(binary)],
                    out / "raw" / f"{case}-direct-rustc-{index}.txt",
                )
                run_output = run_checked([str(binary)], stdin=stdin).stdout
                if run_output != expected:
                    raise RuntimeError(f"direct-rustc correctness failed for {case}")
                row = {"case": case, "arm": "direct-rustc", "sample": index, "peak_rss_kib": rss, "elapsed_s": elapsed}
                arms["direct-rustc"].append(row)
                all_rows.append(row)

            arm_stats = {name: arm_summary(rows) for name, rows in arms.items()}
            direct_median = float(arm_stats["direct-rustc"]["peak_rss_median_kib"])
            for name in ("check", "emit-rust"):
                phase_median = float(arm_stats[name]["peak_rss_median_kib"])
                arm_stats[name]["share_of_direct_rustc"] = phase_median / direct_median if direct_median else None
            summaries[case] = arm_stats

    with (out / "raw-samples.csv").open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=["case", "arm", "sample", "peak_rss_kib", "elapsed_s"])
        writer.writeheader()
        writer.writerows(all_rows)

    report = {
        "git_sha": git_sha,
        "platform": sys.platform,
        "measurement": {
            "tool": "GNU /usr/bin/time",
            "metric": "%M maximum resident set size",
            "unit": "KiB",
            "scope": "directly measured process",
            "whole_build_tree_peak": "unavailable-not-measured",
            "warmups": WARMUPS,
            "samples": SAMPLES,
        },
        "rustc_flags": list(RUSTC_FLAGS),
        "correctness": "PASS",
        "cases": summaries,
    }
    (out / "report.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    lines = [
        "# Compile memory baseline v0",
        "",
        f"- git_sha: `{git_sha}`",
        "- metric: GNU `/usr/bin/time` `%M` maximum resident set size, KiB",
        "- scope: directly measured process only; **not** concurrent whole-process-tree peak RSS",
        "- full `evo build --no-cache` tree peak: `UNAVAILABLE / NOT MEASURED`",
        f"- sampling: {WARMUPS} warmup + {SAMPLES} measured samples per arm/case",
        "- correctness: **PASS**",
        "",
        "| Case | Arm | Median KiB | Min KiB | Max KiB | Median time s | Share of direct rustc |",
        "| --- | --- | ---: | ---: | ---: | ---: | ---: |",
    ]
    for case in CASES:
        case_stats = summaries[case]
        for arm in ("check", "emit-rust", "direct-rustc"):
            stats = case_stats[arm]
            share = stats.get("share_of_direct_rustc")
            share_text = "1.000" if arm == "direct-rustc" else f"{float(share):.3f}"
            lines.append(
                f"| `{case}` | `{arm}` | {float(stats['peak_rss_median_kib']):.0f} | "
                f"{int(stats['peak_rss_min_kib'])} | {int(stats['peak_rss_max_kib'])} | "
                f"{float(stats['elapsed_median_s']):.3f} | {share_text} |"
            )
    lines.extend([
        "",
        "Interpretation must use the pre-registered #91 guide. This harness deliberately does not "
        "pretend that GNU time's process RSS is a simultaneous process-tree peak.",
        "",
    ])
    (out / "report.md").write_text("\n".join(lines), encoding="utf-8")
    print((out / "report.md").read_text(encoding="utf-8"))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
