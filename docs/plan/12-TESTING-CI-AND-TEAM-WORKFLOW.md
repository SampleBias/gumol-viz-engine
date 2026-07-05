# 12 — Testing, CI & Team Workflow

**Priority:** P1 (continuous)  
**Estimated effort:** Ongoing — 1 developer owns this throughout  
**Dependencies:** 01 (start immediately)  
**Runs in parallel with all other plans**

---

## Goal

Keep main branch green, prevent regressions, and enable 4–6 developers to work in parallel without stepping on each other.

---

## Testing Strategy

### Test Pyramid

```
        ┌─────────┐
        │  Manual │  Smoke test checklist (every release)
        │  QA     │
        ├─────────┤
        │ Integr. │  tests/*.rs — full load pipeline
        │  Tests  │
        ├─────────┤
        │  Unit   │  #[cfg(test)] in each module
        │  Tests  │
        ├─────────┤
        │  Bench  │  benches/*.rs — performance regression
        └─────────┘
```

---

## Tasks

### Task 12.1 — Unit test coverage per module
**Effort:** Ongoing  
**Target:** Every parser and core function has ≥1 test

| Module | Required tests |
|--------|----------------|
| `io/xyz.rs` | ✅ exists — extend with multi-frame |
| `io/pdb.rs` | Parse ATOM, HETATM, CONECT, CRYST1 |
| `io/gro.rs` | ✅ `tests/test_gro_load.rs` — add multi-frame |
| `io/dcd.rs` | Header parse, single frame (synthetic bytes) |
| `io/mmcif.rs` | water.cif load, atom_site loop |
| `core/atom.rs` | Element from_symbol, cpk_color, vdw_radius |
| `core/bond.rs` | Bond detection thresholds |
| `core/trajectory.rs` | Frame interpolation |
| `interaction/measurement.rs` | Distance, angle, dihedral math |
| `rendering/instanced.rs` | AtomInstanceData size, index map |

---

### Task 12.2 — Integration tests
**Effort:** 2 days initial  
**Files:** `tests/`

- [x] `tests/test_plugin_registration.rs` — GumolVizPlugin builds App
- [x] `tests/test_load_pipeline.rs` — LoadFileEvent → SimulationData populated
- [x] `tests/test_xyz_load.rs` — end-to-end XYZ
- [x] `tests/test_pdb_load.rs` — mini.pdb atom count + CONECT
- [x] `tests/test_gro_load.rs` — ✅ exists
- [x] `tests/test_mmcif_load.rs` — water.cif
- [x] `tests/test_format_detection.rs` — all extensions

Use small fixture files in `tests/fixtures/` (copy from `examples/`).

---

### Task 12.3 — CI pipeline (GitHub Actions)
**Effort:** 1 day  
**Files:** `.github/workflows/ci.yml`

```yaml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo fmt -- --check
      - run: cargo clippy -- -D warnings
      - run: cargo test
      - run: cargo build --release
```

- [x] Add CI workflow (`.github/workflows/ci.yml`)
- [x] Cache cargo registry (`Swatinem/rust-cache`)
- [x] Fail on fmt/clippy/test errors
- [x] Optional: bench regression on main only (`bench-smoke` job)

---

### Task 12.4 — Pre-commit hooks
**Effort:** 2 hours  
**Files:** `.pre-commit-config.yaml` or `cargo-husky`

- [x] `cargo fmt` (`.pre-commit-config.yaml`)
- [x] `cargo clippy -- -D warnings`
- [x] `cargo test` (fast tests only — lib + integration)

---

### Task 12.5 — Smoke test checklist (manual, every release)

```
[ ] cargo run — window opens, no panic
[ ] Open water.xyz — 3 atoms visible
[ ] Open 1CRN.pdb — protein visible, correct colors
[ ] Open alanine.gro — loads without error
[ ] Play trajectory (multi-frame XYZ) — animates
[ ] Click atom — selected and highlighted
[ ] 2 atoms selected — distance shown
[ ] Switch ball-and-stick — bonds appear
[ ] Screenshot — PNG saved
[ ] Export OBJ — file opens in Blender
[ ] Load new file — previous scene cleared
[ ] cargo test — all pass
```

---

## Team Parallel Work Streams

After **01** is merged, assign developers to minimize file conflicts:

| Developer | Primary plan | Primary files | Weeks |
|-----------|-------------|---------------|-------|
| **Dev A** (rendering lead) | 02, 10 | `rendering/instanced.rs`, `rendering/materials.rs` | 1–4 |
| **Dev B** (systems) | 03, 05 | `systems/bonds.rs`, `systems/timeline.rs` | 2–4 |
| **Dev C** (I/O) | 04 | `io/*.rs`, `systems/loading.rs` | 1–3 |
| **Dev D** (interaction) | 06, 07 | `interaction/`, `ui/inspector.rs` | 3–5 |
| **Dev E** (UI/export) | 07, 09 | `ui/`, `export/` | 3–5 |
| **Dev F** (QA/infra) | 12 | `tests/`, `.github/`, `benches/` | 1–ongoing |

### Merge Conflict Hotspots

These files will be touched by multiple devs — coordinate carefully:

| File | Devs | Rule |
|------|------|------|
| `systems/mod.rs` | A, B | Dev A owns system ordering |
| `rendering/instanced.rs` | A, B, D | Dev A owns; others submit PRs to A |
| `ui/mod.rs` | D, E | Split into modules first (07.7) |
| `Cargo.toml` | All | Dev F approves dependency additions |

---

## Branch Strategy

```
main          ← always green, protected
  └── feat/02-instanced-index     (Dev A)
  └── feat/03-bonds-instanced     (Dev B, branched from feat/02)
  └── feat/04-dcd-topology        (Dev C)
  └── feat/06-pick-proxies        (Dev D, branched from feat/02)
  └── feat/12-ci                  (Dev F)
```

- PRs target `main`; rebase weekly
- Feature branches live ≤1 week
- No direct pushes to `main`

---

## PR Checklist (require in every PR)

```markdown
## Checklist
- [ ] cargo fmt
- [ ] cargo clippy -- -D warnings
- [ ] cargo test
- [ ] Updated docs/plan if task completed
- [ ] No new `TODO` without issue link
- [ ] Tested manually (describe what you tested)
```

---

## Documentation Updates

When completing any plan task:
1. Mark task `[x]` in the relevant plan file
2. Update status table in `00-INDEX.md`
3. Update `OPTIMIZATION_PROGRESS.md` if performance-related
4. Update `README.md` if user-facing feature added

---

## Profiling Workflow (from plan 10)

Use these commands when investigating performance regressions:

```bash
# 1. Baseline benchmark
cargo bench --bench rendering -- --save-baseline main

# 2. After changes, compare against baseline
cargo bench --bench rendering -- --baseline main

# 3. Chrome trace (optional `trace` feature)
cargo run --release --features trace

# 4. Bond detection / spatial index
cargo bench --bench bonds

# 5. Load path / memory estimates
cargo bench --bench loading
```

Install local hooks before committing:

```bash
pip install pre-commit   # or your distro package
pre-commit install
pre-commit run --all-files
```

---

## Definition of Done

- [x] CI green on every PR (GitHub Actions workflow added)
- [x] Integration test per file format
- [ ] 6-developer assignment doc shared with team
- [ ] Smoke test checklist passed before v1 tag
