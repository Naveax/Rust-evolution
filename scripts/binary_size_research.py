#!/usr/bin/env python3
from __future__ import annotations

import csv
import hashlib
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

CASES = (
    "runtime-repeat-v0",
    "control-flow-branch-v0",
    "logical-operators-v0",
    "function-call-v0",
    "block-locals-v0",
    "records-v0",
    "enums-v0",
)
RUSTC_FLAGS = (
    "--crate-name",
    "evo_binary_size_case",
    "--edition=2024",
    "--error-format=short",
    "-C",
    "opt-level=3",
    "-C",
    "codegen-units=1",
)
CANONICAL_SOURCE = "benchmark.rs"
SECTION_RE = re.compile(r"^\s*(\S+)\s+(\d+)\s+([0-9a-fA-Fx]+)\s*$")
NEEDED_RE = re.compile(r"Shared library: \[(.+?)\]")


def run(cmd: list[str], *, stdin: bytes | None = None, cwd: pathlib.Path | None = None) -> subprocess.CompletedProcess[bytes]:
    proc = subprocess.run(cmd, input=stdin, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    if proc.returncode != 0:
        raise RuntimeError(
            f"command failed ({proc.returncode}): {' '.join(cmd)}\n"
            f"stdout={proc.stdout.decode(errors='replace')}\n"
            f"stderr={proc.stderr.decode(errors='replace')}"
        )
    return proc


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def compile_canonical(rustc: str, source: pathlib.Path, output: pathlib.Path, work_root: pathlib.Path, label: str) -> None:
    work = work_root / label
    work.mkdir(parents=True, exist_ok=True)
    staged = work / CANONICAL_SOURCE
    shutil.copyfile(source, staged)
    run([rustc, CANONICAL_SOURCE, *RUSTC_FLAGS, "-o", str(output.resolve())], cwd=work)


def parse_sections(text: str) -> dict[str, int]:
    sections: dict[str, int] = {}
    for line in text.splitlines():
        match = SECTION_RE.match(line)
        if not match:
            continue
        name, size, _address = match.groups()
        if name in {"section", "Total"}:
            continue
        sections[name] = int(size)
    return sections


def needed_names(text: str) -> list[str]:
    return sorted(match.group(1) for match in NEEDED_RE.finditer(text))


def main() -> int:
    if sys.platform != "linux":
        raise RuntimeError("binary-size research is intentionally Linux-only")

    root = pathlib.Path(__file__).resolve().parents[1]
    out = pathlib.Path(os.environ.get("EVO_BINARY_SIZE_OUT", root / "target/evo-binary-size-research"))
    evo = pathlib.Path(os.environ.get("EVO_BIN", root / "target/debug/evo"))
    rustc = os.environ.get("RUSTC", "rustc")
    size_tool = shutil.which("size")
    readelf = shutil.which("readelf")
    git_sha = os.environ.get("EVO_GIT_SHA", os.environ.get("GITHUB_SHA", "local"))

    if not evo.is_file():
        raise RuntimeError(f"evo binary not found: {evo}")
    if not size_tool or not readelf:
        raise RuntimeError("GNU binutils size/readelf are required for the controlled Linux baseline")

    out.mkdir(parents=True, exist_ok=True)
    (out / "rustc-vV.txt").write_bytes(run([rustc, "-vV"]).stdout)
    (out / "size-version.txt").write_bytes(run([size_tool, "--version"]).stdout)
    (out / "readelf-version.txt").write_bytes(run([readelf, "--version"]).stdout)
    (out / "methodology.txt").write_text(
        "Reference and Evolution-generated Rust are each staged as benchmark.rs in isolated work directories, "
        "compiled with identical crate name and production-equivalent Rust 1.98 flags. This avoids source-path/crate-name "
        "identity noise. The baseline deliberately does not enable ThinLTO, stripping, panic=abort, or a different linker.\n",
        encoding="utf-8",
    )

    rows: list[dict[str, object]] = []
    report_cases: dict[str, object] = {}

    with tempfile.TemporaryDirectory(prefix="evo-binary-size-") as tmp_text:
        tmp = pathlib.Path(tmp_text)
        for case in CASES:
            case_dir = root / "benchmarks" / "cases" / case
            evo_source = case_dir / "evolution.evo"
            reference_source = case_dir / "reference.rs"
            stdin = (case_dir / "stdin.bin").read_bytes()
            expected = (case_dir / "expected.stdout").read_bytes()

            generated = run([str(evo), "emit-rust", str(evo_source)]).stdout
            retained = out / case
            retained.mkdir(parents=True, exist_ok=True)
            generated_source = retained / "generated.rs"
            generated_source.write_bytes(generated)
            shutil.copyfile(reference_source, retained / "reference.rs")

            reference_binary = tmp / f"{case}-reference"
            evolution_binary = tmp / f"{case}-evolution"
            compile_canonical(rustc, reference_source, reference_binary, tmp, f"{case}-reference-work")
            compile_canonical(rustc, generated_source, evolution_binary, tmp, f"{case}-evolution-work")

            reference_output = run([str(reference_binary)], stdin=stdin).stdout
            evolution_output = run([str(evolution_binary)], stdin=stdin).stdout
            if reference_output != expected:
                raise RuntimeError(f"reference correctness failed for {case}")
            if evolution_output != expected:
                raise RuntimeError(f"Evolution correctness failed for {case}")
            if reference_output != evolution_output:
                raise RuntimeError(f"differential correctness failed for {case}")

            ref_size = reference_binary.stat().st_size
            evo_size = evolution_binary.stat().st_size
            delta = evo_size - ref_size
            overhead = (delta / ref_size) if ref_size else 0.0
            ref_sha = sha256_file(reference_binary)
            evo_sha = sha256_file(evolution_binary)
            byte_equal = ref_sha == evo_sha

            ref_size_raw = run([size_tool, "-A", str(reference_binary)]).stdout.decode("utf-8", errors="strict")
            evo_size_raw = run([size_tool, "-A", str(evolution_binary)]).stdout.decode("utf-8", errors="strict")
            (retained / "reference-size-A.txt").write_text(ref_size_raw, encoding="utf-8")
            (retained / "evolution-size-A.txt").write_text(evo_size_raw, encoding="utf-8")
            ref_sections = parse_sections(ref_size_raw)
            evo_sections = parse_sections(evo_size_raw)

            ref_readelf = run([readelf, "-d", str(reference_binary)]).stdout.decode("utf-8", errors="strict")
            evo_readelf = run([readelf, "-d", str(evolution_binary)]).stdout.decode("utf-8", errors="strict")
            (retained / "reference-readelf-d.txt").write_text(ref_readelf, encoding="utf-8")
            (retained / "evolution-readelf-d.txt").write_text(evo_readelf, encoding="utf-8")
            ref_needed = needed_names(ref_readelf)
            evo_needed = needed_names(evo_readelf)

            section_names = sorted(set(ref_sections) | set(evo_sections))
            section_deltas = {
                name: evo_sections.get(name, 0) - ref_sections.get(name, 0)
                for name in section_names
                if evo_sections.get(name, 0) != ref_sections.get(name, 0)
            }

            row = {
                "case": case,
                "reference_bytes": ref_size,
                "evolution_bytes": evo_size,
                "delta_bytes": delta,
                "overhead_ratio": overhead,
                "byte_equal": byte_equal,
                "needed_equal": ref_needed == evo_needed,
                "source_equal": reference_source.read_bytes() == generated,
            }
            rows.append(row)
            report_cases[case] = {
                **row,
                "reference_sha256": ref_sha,
                "evolution_sha256": evo_sha,
                "reference_needed": ref_needed,
                "evolution_needed": evo_needed,
                "section_deltas_bytes": section_deltas,
            }

    with (out / "raw-sizes.csv").open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0].keys()))
        writer.writeheader()
        writer.writerows(rows)

    followup_cases = [
        row["case"]
        for row in rows
        if int(row["delta_bytes"]) > 16 * 1024 and float(row["overhead_ratio"]) > 0.01
    ]
    threshold_edge_cases = [
        row["case"]
        for row in rows
        if int(row["delta_bytes"]) > 16 * 1024 or float(row["overhead_ratio"]) > 0.01
    ]
    hidden_footprint_cases = [
        row["case"]
        for row in rows
        if not bool(row["needed_equal"])
    ]
    if len(followup_cases) >= 2 or hidden_footprint_cases:
        verdict = "FOLLOW-UP-CANDIDATE"
    elif threshold_edge_cases:
        verdict = "EXPAND-INVESTIGATE"
    else:
        verdict = "DEFER-NO-ACTION"

    report = {
        "git_sha": git_sha,
        "platform": sys.platform,
        "rustc_flags": list(RUSTC_FLAGS),
        "canonical_source_name": CANONICAL_SOURCE,
        "correctness": "PASS",
        "pre_registered_thresholds": {"overhead_ratio": 0.01, "overhead_bytes": 16384, "representative_cases": 2},
        "followup_cases": followup_cases,
        "threshold_edge_cases": threshold_edge_cases,
        "hidden_dependency_cases": hidden_footprint_cases,
        "verdict": verdict,
        "cases": report_cases,
    }
    (out / "report.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    lines = [
        "# Binary size baseline v0",
        "",
        f"- git_sha: `{git_sha}`",
        "- correctness: **PASS**",
        f"- aggregate verdict: **{verdict}**",
        "- compile identity: canonical `benchmark.rs`, crate `evo_binary_size_case`, edition 2024, opt3, cgu1",
        "- no ThinLTO, stripping, panic strategy change, incremental state, or linker override",
        "",
        "| Case | Reference bytes | Evolution bytes | Delta | Overhead | Byte equal | DT_NEEDED equal | Source equal |",
        "| --- | ---: | ---: | ---: | ---: | --- | --- | --- |",
    ]
    for row in rows:
        lines.append(
            f"| `{row['case']}` | {row['reference_bytes']} | {row['evolution_bytes']} | {row['delta_bytes']} | "
            f"{float(row['overhead_ratio']) * 100:.4f}% | {row['byte_equal']} | {row['needed_equal']} | {row['source_equal']} |"
        )
    lines.extend([
        "",
        "The decision follows the pre-registered #93 Evolution-specific overhead guide. Absolute ordinary Rust/toolchain footprint is baseline data, not automatically an Evolution defect.",
        "",
    ])
    (out / "report.md").write_text("\n".join(lines), encoding="utf-8")
    print((out / "report.md").read_text(encoding="utf-8"))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
