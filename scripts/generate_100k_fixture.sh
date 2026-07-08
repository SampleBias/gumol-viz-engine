#!/usr/bin/env bash
# Generate standard 100K profiling fixtures (static + 10-frame playback).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo test --lib utils::synthetic::tests::write_and_parse_synthetic_xyz_roundtrip -- --nocapture

python3 - <<'PY'
from pathlib import Path

root = Path(".")
static_path = root / "tests/fixtures/synthetic_100k.xyz"
playback_path = root / "tests/fixtures/synthetic_100k_10f.xyz"
static_path.parent.mkdir(parents=True, exist_ok=True)

def element(i: int) -> str:
    return ("C", "H", "O")[i % 3]

def pos(i: int, frame: int) -> tuple[float, float, float]:
    import math
    f = float(frame)
    x = math.sin(i * 1.5) * 10.0 + f * 0.01
    y = math.cos(i * 0.7) * 10.0 + f * 0.005
    z = math.sin(i * 0.3) * 10.0
    return x, y, z

def write_xyz(path: Path, atoms: int, frames: int) -> None:
    with path.open("w", encoding="utf-8") as fh:
        for frame in range(frames):
            fh.write(f"{atoms}\n")
            fh.write(f"synthetic {atoms} atoms frame {frame}\n")
            for i in range(atoms):
                x, y, z = pos(i, frame)
                fh.write(f"{element(i)} {x:.6f} {y:.6f} {z:.6f}\n")

if not static_path.exists():
    print(f"Writing {static_path}")
    write_xyz(static_path, 100_000, 1)
else:
    print(f"Exists: {static_path}")

if not playback_path.exists():
    print(f"Writing {playback_path}")
    write_xyz(playback_path, 100_000, 10)
else:
    print(f"Exists: {playback_path}")

print("100K fixtures ready")
PY
