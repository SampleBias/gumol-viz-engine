//! LRU frame cache and background prefetch for streaming trajectories.
//!
//! Large DCD trajectories use on-demand frame loading (plan 04). This module
//! keeps the last N parsed frames in memory and prefetches upcoming frames
//! during playback so timeline scrubbing stays responsive.

use crate::core::trajectory::{FrameData, TimelineState};
use crate::io::streaming::FrameProvider;
use crate::systems::loading::{FileLoadedEvent, SimulationData};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

/// Default number of parsed frames kept in the LRU cache.
pub const FRAME_CACHE_CAPACITY: usize = 30;

/// How many frames ahead to prefetch during playback.
pub const PREFETCH_AHEAD: usize = 3;

/// Resolved current/next frames for timeline position updates.
#[derive(Resource, Default, Debug, Clone)]
pub struct TimelineFrames {
    pub current_index: usize,
    pub next_index: usize,
    pub current: Option<FrameData>,
    pub next: Option<FrameData>,
    /// True when a requested frame is not yet available (cache miss + disk read).
    pub loading: bool,
    /// Whether the trajectory uses on-demand frame loading.
    pub streaming: bool,
    /// Number of frames currently held in the LRU cache.
    pub cached_count: usize,
}

/// LRU cache of parsed trajectory frames for streaming providers.
#[derive(Resource, Debug)]
pub struct FrameCache {
    capacity: usize,
    lru: VecDeque<usize>,
    frames: HashMap<usize, FrameData>,
    prefetch_pending: HashSet<usize>,
    prefetch_rx: Option<crossbeam_channel::Receiver<(usize, Result<FrameData, String>)>>,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

impl Default for FrameCache {
    fn default() -> Self {
        Self::new(FRAME_CACHE_CAPACITY)
    }
}

impl FrameCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            lru: VecDeque::new(),
            frames: HashMap::new(),
            prefetch_pending: HashSet::new(),
            prefetch_rx: None,
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    pub fn clear(&mut self) {
        self.lru.clear();
        self.frames.clear();
        self.prefetch_pending.clear();
        self.prefetch_rx = None;
    }

    pub fn contains(&self, index: usize) -> bool {
        self.frames.contains_key(&index)
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Fetch a frame, loading from the provider on cache miss.
    pub fn get_or_load(
        &mut self,
        provider: &dyn FrameProvider,
        index: usize,
    ) -> Result<FrameData, String> {
        if let Some(frame) = self.frames.get(&index) {
            let frame = frame.clone();
            self.touch(index);
            self.cache_hits += 1;
            return Ok(frame);
        }

        self.cache_misses += 1;
        let frame = provider.get_frame(index).map_err(|e| e.to_string())?;
        self.insert(index, frame.clone());
        Ok(frame)
    }

    fn insert(&mut self, index: usize, frame: FrameData) {
        if self.frames.contains_key(&index) {
            self.touch(index);
            self.frames.insert(index, frame);
            return;
        }

        while self.frames.len() >= self.capacity {
            if let Some(evict) = self.lru.pop_front() {
                self.frames.remove(&evict);
                self.prefetch_pending.remove(&evict);
            } else {
                break;
            }
        }

        self.lru.push_back(index);
        self.frames.insert(index, frame);
    }

    fn touch(&mut self, index: usize) {
        if let Some(pos) = self.lru.iter().position(|&i| i == index) {
            self.lru.remove(pos);
        }
        self.lru.push_back(index);
    }

    /// Queue background loads for frames not yet cached.
    pub fn prefetch(
        &mut self,
        provider: Arc<dyn FrameProvider>,
        indices: impl IntoIterator<Item = usize>,
    ) {
        if self.prefetch_rx.is_some() {
            return;
        }

        let mut to_load = Vec::new();
        for index in indices {
            if self.frames.contains_key(&index) || self.prefetch_pending.contains(&index) {
                continue;
            }
            to_load.push(index);
            self.prefetch_pending.insert(index);
        }

        if to_load.is_empty() {
            return;
        }

        let (tx, rx) = crossbeam_channel::unbounded();
        self.prefetch_rx = Some(rx);

        std::thread::spawn(move || {
            for index in to_load {
                let result = provider.get_frame(index).map_err(|e| e.to_string());
                let _ = tx.send((index, result));
            }
        });
    }

    /// Apply completed prefetch results to the cache.
    pub fn poll_prefetch(&mut self) {
        let Some(rx) = self.prefetch_rx.as_ref() else {
            return;
        };

        let messages: Vec<(usize, Result<FrameData, String>)> = rx.try_iter().collect();
        if messages.is_empty() {
            return;
        }

        for (index, result) in messages {
            self.prefetch_pending.remove(&index);
            match result {
                Ok(frame) => self.insert(index, frame),
                Err(err) => warn!("Prefetch frame {index} failed: {err}"),
            }
        }

        if self.prefetch_pending.is_empty() {
            self.prefetch_rx = None;
        }
    }
}

/// Resolve current and next frames for the timeline (uses cache when streaming).
pub fn resolve_timeline_frames(
    sim_data: Res<SimulationData>,
    timeline: Res<TimelineState>,
    mut cache: ResMut<FrameCache>,
    mut frames: ResMut<TimelineFrames>,
) {
    if !sim_data.loaded || sim_data.num_frames() == 0 {
        *frames = TimelineFrames::default();
        return;
    }

    cache.poll_prefetch();

    frames.streaming = sim_data.is_streaming();
    frames.cached_count = cache.len();

    let current_idx = timeline
        .current_frame
        .min(sim_data.num_frames().saturating_sub(1));
    let next_idx = (current_idx + 1).min(sim_data.num_frames().saturating_sub(1));

    if frames.current_index == current_idx
        && frames.next_index == next_idx
        && frames.current.is_some()
        && (!timeline.interpolate || timeline.interpolation_factor <= 0.0 || frames.next.is_some())
        && !frames.loading
    {
        return;
    }

    frames.current_index = current_idx;
    frames.next_index = next_idx;
    frames.loading = false;

    if sim_data.is_streaming() {
        let Some(provider) = sim_data.frame_provider() else {
            frames.current = None;
            frames.next = None;
            return;
        };

        match cache.get_or_load(provider.as_ref(), current_idx) {
            Ok(frame) => frames.current = Some(frame),
            Err(err) => {
                error!("Failed to load frame {current_idx}: {err}");
                frames.current = None;
                frames.loading = true;
            }
        }

        if timeline.interpolate && current_idx != next_idx {
            match cache.get_or_load(provider.as_ref(), next_idx) {
                Ok(frame) => frames.next = Some(frame),
                Err(err) => {
                    warn!("Failed to load next frame {next_idx}: {err}");
                    frames.next = None;
                }
            }
        } else {
            frames.next = None;
        }
    } else {
        frames.current = sim_data.get_frame(current_idx);
        frames.next = if timeline.interpolate && current_idx != next_idx {
            sim_data.get_frame(next_idx)
        } else {
            None
        };
    }
}

/// Prefetch upcoming frames during playback on streaming trajectories.
pub fn prefetch_during_playback(
    sim_data: Res<SimulationData>,
    timeline: Res<TimelineState>,
    mut cache: ResMut<FrameCache>,
) {
    if !sim_data.is_streaming() || !timeline.is_playing || sim_data.num_frames() <= 1 {
        return;
    }

    let Some(provider) = sim_data.frame_provider() else {
        return;
    };

    let start = timeline.current_frame + 1;
    let end = (start + PREFETCH_AHEAD).min(sim_data.num_frames());
    let indices: Vec<usize> = (start..end).collect();
    cache.prefetch(provider, indices);
}

/// Clear cache when a new trajectory is loaded.
pub fn clear_frame_cache_on_load(
    mut cache: ResMut<FrameCache>,
    mut frames: ResMut<TimelineFrames>,
    file_loaded_events: EventReader<FileLoadedEvent>,
    topology_events: EventReader<crate::systems::loading::TopologyAppliedEvent>,
) {
    if file_loaded_events.is_empty() && topology_events.is_empty() {
        return;
    }

    cache.clear();
    *frames = TimelineFrames::default();
    info!("Frame cache cleared for new trajectory");
}

pub fn register(app: &mut App) {
    app.init_resource::<FrameCache>()
        .init_resource::<TimelineFrames>();
    info!("Frame cache registered (capacity {})", FRAME_CACHE_CAPACITY);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::trajectory::TrajectoryMetadata;
    use bevy::prelude::Vec3;
    use std::path::Path;

    struct TestProvider {
        num_frames: usize,
        num_atoms: usize,
        metadata: TrajectoryMetadata,
    }

    impl TestProvider {
        fn new(num_frames: usize, num_atoms: usize) -> Self {
            Self {
                num_frames,
                num_atoms,
                metadata: TrajectoryMetadata::default(),
            }
        }
    }

    impl FrameProvider for TestProvider {
        fn num_frames(&self) -> usize {
            self.num_frames
        }

        fn num_atoms(&self) -> usize {
            self.num_atoms
        }

        fn time_step(&self) -> f32 {
            1.0
        }

        fn file_path(&self) -> &Path {
            Path::new("test.dcd")
        }

        fn metadata(&self) -> &TrajectoryMetadata {
            &self.metadata
        }

        fn get_frame(&self, index: usize) -> crate::io::IOResult<FrameData> {
            let mut frame = FrameData::new(index, index as f32);
            frame.set_position(0, Vec3::new(index as f32, 0.0, 0.0));
            Ok(frame)
        }
    }

    #[test]
    fn test_lru_eviction() {
        let provider = TestProvider::new(100, 1);
        let mut cache = FrameCache::new(3);

        for i in 0..5 {
            cache.get_or_load(&provider, i).unwrap();
        }

        assert_eq!(cache.len(), 3);
        assert!(!cache.contains(0));
        assert!(!cache.contains(1));
        assert!(cache.contains(4));
    }

    #[test]
    fn test_cache_hit_tracking() {
        let provider = TestProvider::new(10, 1);
        let mut cache = FrameCache::new(30);

        cache.get_or_load(&provider, 0).unwrap();
        cache.get_or_load(&provider, 0).unwrap();

        assert_eq!(cache.cache_hits, 1);
        assert_eq!(cache.cache_misses, 1);
    }
}
