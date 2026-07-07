#!/usr/bin/env python3
"""Compare criterion benchmark output against benches/baseline.json.

Fails with exit code 1 when any benchmark mean exceeds baseline * max_regression_ratio.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BASELINE_PATH = ROOT / "benches" / "baseline.json"
MAX_REGRESSION_DEFAULT = 1.20

NAME_RE = re.compile(r"^(\S+/\d+)\s*$")
# Criterion prints: time:   [14.871 µs 14.888 µs 14.914 µs]  (unit after each value)
TIME_RE = re.compile(
    r"time:\s+\[([\d.]+)\s+(?:µs|us|ms|ns|s)\s+([\d.]+)\s+(?:µs|us|ms|ns|s)\s+([\d.]+)\s+(?:µs|us|ms|ns|s)\]"
)


def unit_to_ns_multiplier(line: str) -> float:
    if " ms " in line or line.rstrip().endswith(" ms]"):
        return 1_000_000.0
    if " ns " in line or line.rstrip().endswith(" ns]"):
        return 1.0
    if " s " in line or line.rstrip().endswith(" s]"):
        return 1_000_000_000.0
    # µs / us (default for these benches)
    return 1_000.0


def parse_benchmarks(output: str) -> dict[str, float]:
    results: dict[str, float] = {}
    current_name: str | None = None

    for line in output.splitlines():
        stripped = line.strip()
        name_match = NAME_RE.match(stripped)
        if name_match and "time:" not in stripped:
            current_name = name_match.group(1)
            continue

        inline = re.match(r"^(\S+/\d+)\s+time:", stripped)
        if inline:
            current_name = inline.group(1)

        time_match = TIME_RE.search(line)
        if time_match and current_name:
            mean = float(time_match.group(2))
            results[current_name] = mean * unit_to_ns_multiplier(line)
            current_name = None

    return results


def run_benchmarks() -> dict[str, float]:
    cmd = [
        "cargo",
        "bench",
        "--bench",
        "rendering",
        "--",
        "--noplot",
        "--sample-size",
        "20",
    ]
    print("Running:", " ".join(cmd))
    proc = subprocess.run(
        cmd,
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    output = proc.stdout + proc.stderr
    if proc.returncode != 0:
        print(output, file=sys.stderr)
        raise SystemExit(f"cargo bench failed with code {proc.returncode}")
    return parse_benchmarks(output)


def load_baseline() -> dict:
    with BASELINE_PATH.open() as f:
        return json.load(f)


def main() -> int:
    if not BASELINE_PATH.exists():
        print(f"Missing baseline: {BASELINE_PATH}", file=sys.stderr)
        return 1

    baseline = load_baseline()
    max_ratio = float(baseline.get("max_regression_ratio", MAX_REGRESSION_DEFAULT))
    expected: dict[str, float] = baseline["benchmarks"]
    actual = run_benchmarks()

    if not actual:
        print("No benchmark results parsed from criterion output.", file=sys.stderr)
        return 1

    failures: list[str] = []
    missing: list[str] = []

    for name, base_ns in expected.items():
        if name not in actual:
            missing.append(name)
            continue
        ratio = actual[name] / base_ns
        if ratio > max_ratio:
            failures.append(
                f"{name}: {actual[name]/1e6:.3f} ms vs baseline {base_ns/1e6:.3f} ms "
                f"({ratio:.2%} > {max_ratio:.0%} limit)"
            )
        else:
            print(f"OK  {name}: {actual[name]/1e6:.3f} ms (baseline {base_ns/1e6:.3f} ms)")

    if missing:
        print("\nMissing benchmarks (not in criterion output):", file=sys.stderr)
        for name in missing:
            print(f"  - {name}", file=sys.stderr)

    if failures:
        print("\nRegression failures:", file=sys.stderr)
        for msg in failures:
            print(f"  - {msg}", file=sys.stderr)
        return 1

    print(f"\nAll {len(expected)} benchmarks within {max_ratio:.0%} of baseline.")
    return 0 if not missing else 1


if __name__ == "__main__":
    raise SystemExit(main())
