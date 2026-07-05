# 05 — Timeline & Animation

**Priority:** P1  
**Estimated effort:** 4–5 developer-days  
**Dependencies:** 02 (instanced position updates)  
**Can parallelize with:** 04, 06

---

## Goal

Smooth, performant trajectory playback for 10,000+ frames with optional GPU-side interpolation.

---

## Current State

**Done (`systems/timeline.rs`):**
- `TimelineState` resource (current frame, playing, speed, loop)
- `update_timeline` advances frame index based on delta time
- `update_instanced_positions_from_timeline` in instanced module
- Keyboard controls (space = play/pause, arrows = step)
- UI timeline slider and frame counter

**Partial:**
- Frame interpolation logic may exist but needs verification with multi-frame files
- `TimelineState` stores integer frame index — no sub-frame alpha

**Not done:**
- GPU compute interpolation (OPTIMIZATION Phase 3)
- Frame preloading / caching
- Timeline markers and annotations

---

## Tasks

### Task 5.1 — Sub-frame interpolation
**Effort:** 1 day  
**Files:** `systems/timeline.rs`, `rendering/instanced.rs`

- [ ] Add to `TimelineState`:
  ```rust
  pub frame_alpha: f32,  // 0.0–1.0 fractional position between frames
  pub interpolated: bool,
  ```
- [ ] `update_timeline` computes fractional frame: `current + alpha`
- [ ] Position update lerps between frame N and N+1:
  ```rust
  pos = lerp(frame[n].pos[atom_id], frame[n+1].pos[atom_id], alpha)
  ```
- [ ] Toggle in UI: "Smooth playback"

**Acceptance:** Slow-motion playback shows smooth motion, not frame stepping.

---

### Task 5.2 — Interpolation for all atom IDs
**Effort:** 4 hours  
**Files:** `core/trajectory.rs`, `systems/timeline.rs`

- [ ] Verify `FrameData.positions` is `HashMap<u32, Vec3>` with all atom IDs in every frame
- [ ] Handle missing atoms (pad with previous position + warning)
- [ ] Unit test: 2-frame trajectory, verify lerp at alpha=0.5

---

### Task 5.3 — Playback speed and looping
**Effort:** 2 hours  
**Files:** `systems/timeline.rs`, `ui/mod.rs`

- [ ] Playback speed: 0.25×, 0.5×, 1×, 2×, 4× (UI dropdown exists — verify wired)
- [ ] Loop: jump to frame 0 when reaching last frame
- [ ] Ping-pong loop (optional, v2)

---

### Task 5.4 — Frame caching for streaming (depends on 4.7)
**Effort:** 1 day  
**Files:** new `src/systems/frame_cache.rs`

- [ ] LRU cache: hold last 30 parsed frames in memory
- [ ] Prefetch next frame on background thread during playback
- [ ] Timeline scrub: show "Loading frame…" if not cached

---

### Task 5.5 — GPU compute interpolation (optional, high impact)
**Effort:** 2 days  
**Files:** `assets/shaders/atom_interpolate.wgsl`, `rendering/instanced.rs`

From OPTIMIZATION_PROGRESS Phase 3.

- [ ] Upload frame N and N+1 positions to GPU buffers
- [ ] Compute shader: `out_pos = mix(pos_a, pos_b, alpha)`
- [ ] Skip CPU lerp entirely during playback
- [ ] Fallback to CPU path if compute not available

**Efficiency:** Do this after 5.1 CPU path works. Benchmark before/after on 100K atoms.

---

### Task 5.6 — Timeline UI enhancements
**Effort:** 4 hours  
**Files:** `ui/mod.rs`

- [ ] Show time in ps/ns if metadata available (GRO/DCD delta)
- [ ] Frame number input (jump to frame)
- [ ] Playback range (start/end frame for loop region) — optional v2

---

## Performance Targets

| Atoms | Frames | Target FPS during playback |
|-------|--------|---------------------------|
| 1K | 100 | 60 |
| 10K | 1,000 | 60 |
| 100K | 10,000 | 30+ (with GPU interpolation) |

---

## Definition of Done

- [ ] Multi-frame XYZ and GRO trajectories animate smoothly
- [ ] Interpolation toggle works
- [ ] No frame stutter on 1K atom, 100 frame system
- [ ] Timeline UI shows correct frame/time
