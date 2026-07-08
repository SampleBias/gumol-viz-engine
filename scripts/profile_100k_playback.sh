#!/usr/bin/env bash
# Run interactive 100K GPU profiling validation (playback @ 30+ FPS target).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

"${ROOT}/scripts/generate_100k_fixture.sh"

OUTPUT="${GUMOL_PROFILE_OUTPUT:-target/profile_100k_playback.json}"
WARMUP="${GUMOL_PROFILE_WARMUP:-120}"
FRAMES="${GUMOL_PROFILE_FRAMES:-300}"

echo "Running 100K playback profiling (warmup=${WARMUP}, samples=${FRAMES})..."
cargo run --release -- \
  --profile \
  --profile-playback \
  --profile-exit \
  --profile-warmup="${WARMUP}" \
  --profile-frames="${FRAMES}" \
  --profile-output="${OUTPUT}" \
  --generate-100k

echo "Report written to ${OUTPUT}"
