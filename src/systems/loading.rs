//! File loading system
//!
//! This system handles loading molecular trajectory files and storing
//! the parsed data in Bevy resources.

use crate::core::atom::AtomData;
use crate::core::atom::Element;
use crate::core::trajectory::{FrameData, Trajectory};
use crate::io::gro::GroParser;
use crate::io::mmcif::MmcifParser;
use crate::io::pdb::PDBParser;
use crate::io::streaming::{self, FrameProvider};
use crate::io::xyz::XYZParser;
use crate::io::{load_topology, FileFormat, IOResult};
use bevy::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Arc;

type ParsedLoadResult = (
    Trajectory,
    Vec<AtomData>,
    Vec<crate::core::bond::BondData>,
    Option<Arc<dyn FrameProvider>>,
    bool,
);

/// Resource containing the loaded simulation data
#[derive(Resource, Clone)]
pub struct SimulationData {
    /// Trajectory metadata and in-memory frames (empty when streaming)
    pub trajectory: Trajectory,
    /// On-demand frame access for large trajectories
    frame_provider: Option<Arc<dyn FrameProvider>>,
    /// Atom metadata (static data that doesn't change between frames)
    pub atom_data: Vec<AtomData>,
    /// Bond topology from file (e.g. PDB CONECT) or empty for distance detection
    pub bond_data: Vec<crate::core::bond::BondData>,
    /// Whether data is loaded
    pub loaded: bool,
    /// DCD loaded without topology — atom metadata is placeholder until topology applied
    pub needs_topology: bool,
}

impl Default for SimulationData {
    fn default() -> Self {
        Self {
            trajectory: Trajectory::new(PathBuf::new(), 0, 1.0),
            frame_provider: None,
            atom_data: Vec::new(),
            bond_data: Vec::new(),
            loaded: false,
            needs_topology: false,
        }
    }
}

impl SimulationData {
    /// Create simulation data from a trajectory and atom data
    pub fn new(trajectory: Trajectory, atom_data: Vec<AtomData>) -> Self {
        Self {
            trajectory,
            frame_provider: None,
            atom_data,
            bond_data: Vec::new(),
            loaded: true,
            needs_topology: false,
        }
    }

    /// Create simulation data with explicit bond topology
    pub fn with_bonds(
        trajectory: Trajectory,
        atom_data: Vec<AtomData>,
        bond_data: Vec<crate::core::bond::BondData>,
    ) -> Self {
        Self {
            trajectory,
            frame_provider: None,
            atom_data,
            bond_data,
            loaded: true,
            needs_topology: false,
        }
    }

    /// Attach a streaming frame provider (trajectory.frames may be empty).
    pub fn with_frame_provider(mut self, provider: Arc<dyn FrameProvider>) -> Self {
        self.frame_provider = Some(provider);
        self
    }

    /// Get a trajectory frame (from memory or streaming provider).
    pub fn get_frame(&self, index: usize) -> Option<FrameData> {
        if let Some(provider) = &self.frame_provider {
            provider.get_frame(index).ok()
        } else {
            self.trajectory.get_frame(index).cloned()
        }
    }

    /// Apply topology metadata to a DCD trajectory already loaded with placeholders.
    pub fn apply_topology(
        &mut self,
        atom_data: Vec<AtomData>,
        bond_data: Vec<crate::core::bond::BondData>,
    ) -> Result<(), String> {
        crate::io::topology::validate_atom_count(atom_data.len(), self.num_atoms())?;
        self.atom_data = atom_data;
        self.bond_data = bond_data;
        self.needs_topology = false;
        Ok(())
    }

    /// Get the number of atoms in the simulation
    pub fn num_atoms(&self) -> usize {
        if self.trajectory.num_atoms > 0 {
            self.trajectory.num_atoms
        } else {
            self.atom_data.len()
        }
    }

    /// Get the number of frames in the trajectory
    pub fn num_frames(&self) -> usize {
        if let Some(provider) = &self.frame_provider {
            provider.num_frames()
        } else {
            self.trajectory.num_frames()
        }
    }

    /// Whether frames are loaded on demand from disk (large DCD).
    pub fn is_streaming(&self) -> bool {
        self.frame_provider.is_some()
    }

    /// Access the streaming frame provider when present.
    pub fn frame_provider(&self) -> Option<Arc<dyn FrameProvider>> {
        self.frame_provider.clone()
    }

    /// Get the total simulation time
    pub fn total_time(&self) -> f32 {
        self.trajectory.total_time
    }

    /// Contiguous positions for all `atom_data` entries in frame order (cache-friendly SoA).
    pub fn frame_positions_dense(&self, frame_idx: usize) -> Option<Vec<Vec3>> {
        let frame = self.get_frame(frame_idx)?;
        Some(
            self.atom_data
                .iter()
                .map(|a| frame.get_position(a.id).unwrap_or(Vec3::ZERO))
                .collect(),
        )
    }
}

/// Resource tracking the currently loaded file
#[derive(Resource, Clone, Debug)]
pub struct FileHandle {
    /// Path to the loaded file
    pub path: PathBuf,
    /// File format
    pub format: FileFormat,
    /// File name
    pub name: String,
}

impl FileHandle {
    /// Create a new file handle
    pub fn new(path: PathBuf, format: FileFormat) -> Self {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        Self { path, format, name }
    }
}

/// Resource holding CLI file path from startup args (if any)
#[derive(Resource, Default, Debug)]
pub struct CliFileArg(pub Option<PathBuf>);

/// Resource holding CLI topology path from startup args (pairs with DCD)
#[derive(Resource, Default, Debug)]
pub struct CliTopologyArg(pub Option<PathBuf>);

/// Tracks topology file state for DCD trajectories
#[derive(Resource, Default, Debug)]
pub struct TopologyState {
    pub path: Option<PathBuf>,
    pub pending_dcd: Option<PathBuf>,
}

/// Event sent when a topology file should be loaded and applied
#[derive(Event, Debug)]
pub struct LoadTopologyEvent {
    pub path: PathBuf,
}

/// Event sent after topology is applied to a DCD trajectory
#[derive(Event, Debug)]
pub struct TopologyAppliedEvent {
    pub topology_path: PathBuf,
    pub num_atoms: usize,
}

/// Event sent when a file is requested to be loaded
#[derive(Event, Debug)]
pub struct LoadFileEvent {
    /// Path to the file to load
    pub path: PathBuf,
}

/// Event sent when a file is successfully loaded
#[derive(Event, Debug)]
pub struct FileLoadedEvent {
    /// Path to the loaded file
    pub path: PathBuf,
    /// Number of atoms
    pub num_atoms: usize,
    /// Number of frames
    pub num_frames: usize,
}

/// Event sent when file loading fails
#[derive(Event, Debug)]
pub struct FileLoadErrorEvent {
    /// Path to the file that failed
    pub path: PathBuf,
    /// Error message
    pub error: String,
}

/// Load a file based on its format
fn load_file(path: &Path, topology_path: Option<&Path>) -> IOResult<ParsedLoadResult> {
    let format = FileFormat::from_path(path);

    info!("Loading file: {:?} (format: {:?})", path, format);

    match format {
        FileFormat::XYZ => {
            let trajectory = XYZParser::parse_file(path)?;
            let atom_data = create_atom_data_from_xyz(&trajectory)?;
            Ok((trajectory, atom_data, Vec::new(), None, false))
        }
        FileFormat::PDB => {
            let (trajectory, atom_data, bond_data) = PDBParser::parse_file_with_atoms(path)?;
            Ok((trajectory, atom_data, bond_data, None, false))
        }
        FileFormat::GRO => {
            let trajectory = GroParser::parse_file(path)?;
            let atom_data = create_atom_data_from_gro(&trajectory)?;
            Ok((trajectory, atom_data, Vec::new(), None, false))
        }
        FileFormat::MmCIF => {
            let trajectory = MmcifParser::parse_file(path)?;
            let atom_data = create_atom_data_from_mmcif(&trajectory)?;
            Ok((trajectory, atom_data, Vec::new(), None, false))
        }
        FileFormat::DCD => {
            let (trajectory, frame_provider) = streaming::open_dcd(path)?;

            if let Some(topo_path) = topology_path {
                let (atom_data, bond_data) = load_topology(topo_path)?;
                crate::io::topology::validate_atom_count(atom_data.len(), trajectory.num_atoms)
                    .map_err(crate::io::IOError::InvalidFormat)?;
                Ok((trajectory, atom_data, bond_data, frame_provider, false))
            } else {
                let atom_data = create_placeholder_atom_data(&trajectory)?;
                Ok((trajectory, atom_data, Vec::new(), frame_provider, true))
            }
        }
        _ => Err(crate::io::IOError::UnsupportedFormat(format!(
            "{:?}",
            format
        ))),
    }
}

/// Create atom data from XYZ trajectory
fn create_atom_data_from_xyz(trajectory: &Trajectory) -> IOResult<Vec<AtomData>> {
    let mut atom_data = Vec::new();

    if let Some(first_frame) = trajectory.get_frame(0) {
        // Parse XYZ file to get element information
        if trajectory.file_path.exists() {
            let file = File::open(&trajectory.file_path)?;
            let reader = BufReader::new(file);
            let mut lines = reader.lines();
            let mut atom_index = 0;

            // Skip first line (number of atoms)
            let _ = lines.next();
            // Skip comment line
            let _ = lines.next();

            // Parse atom lines
            while atom_index < trajectory.num_atoms {
                if let Some(Ok(line)) = lines.next() {
                    let parts: Vec<&str> = line.split_whitespace().collect();

                    if parts.len() >= 4 {
                        // Parse element
                        let element = Element::from_symbol(parts[0]).unwrap_or_else(|_| {
                            warn!("Unknown element: {}, using Unknown", parts[0]);
                            Element::Unknown
                        });

                        // Create atom data
                        atom_data.push(AtomData::new(
                            atom_index as u32,
                            element,
                            0,                    // residue ID
                            "UNK".to_string(),    // residue name
                            "A".to_string(),      // chain ID
                            parts[0].to_string(), // atom name (use element symbol)
                        ));

                        atom_index += 1;
                    }
                } else {
                    break;
                }
            }
        } else {
            // Fallback: create atom data without element info
            for atom_id in first_frame.atom_ids() {
                atom_data.push(AtomData::new(
                    *atom_id,
                    Element::Unknown,
                    0,
                    "UNK".to_string(),
                    "A".to_string(),
                    format!("ATOM{}", atom_id),
                ));
            }
        }
    }

    Ok(atom_data)
}

/// Create atom data from GRO trajectory
fn create_atom_data_from_gro(trajectory: &Trajectory) -> IOResult<Vec<AtomData>> {
    let mut atom_data = Vec::new();

    // Parse GRO file to get element and residue information
    if trajectory.file_path.exists() {
        let file = File::open(&trajectory.file_path)?;
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;

        if lines.len() >= 3 {
            // Skip title line (0) and atom count line (1)
            // Start from line 2 (first atom line)
            for (i, line) in lines.iter().skip(2).take(trajectory.num_atoms).enumerate() {
                let parsed = crate::io::gro::GroParser::parse_atom_line(line, i + 3, i)
                    .unwrap_or_else(|_| {
                        // Fallback to default values
                        crate::io::gro::ParsedAtom {
                            residue_id: 1,
                            residue_name: "UNK".to_string(),
                            atom_name: "X".to_string(),
                            element: Element::Unknown,
                            position: Vec3::ZERO,
                            velocity: None,
                        }
                    });

                atom_data.push(AtomData::new(
                    i as u32,
                    parsed.element,
                    parsed.residue_id as u32,
                    parsed.residue_name,
                    "A".to_string(), // GRO doesn't have chain ID
                    parsed.atom_name,
                ));
            }
        }
    } else {
        // Fallback: create placeholder atom data
        return create_placeholder_atom_data(trajectory);
    }

    Ok(atom_data)
}

/// Create atom data from mmCIF trajectory
fn create_atom_data_from_mmcif(trajectory: &Trajectory) -> IOResult<Vec<AtomData>> {
    // Re-parse mmCIF file to extract atom metadata (element, residue, chain, etc.)
    if trajectory.file_path.exists() {
        if let Ok(atom_data) = MmcifParser::parse_atom_data_from_file(&trajectory.file_path) {
            if atom_data.len() == trajectory.num_atoms {
                return Ok(atom_data);
            }
        }
    }
    // Fallback: create placeholder atom data
    create_placeholder_atom_data(trajectory)
}

/// Create placeholder atom data (for formats without atom metadata)
fn create_placeholder_atom_data(trajectory: &Trajectory) -> IOResult<Vec<AtomData>> {
    let mut atom_data = Vec::new();
    let count = trajectory.num_atoms;

    for atom_id in 0..count {
        atom_data.push(AtomData::new(
            atom_id as u32,
            Element::Unknown,
            0,
            "UNK".to_string(),
            "A".to_string(),
            format!("ATOM{atom_id}"),
        ));
    }

    Ok(atom_data)
}

fn apply_load_result(
    sim_data: &mut SimulationData,
    trajectory: Trajectory,
    atom_data: Vec<AtomData>,
    bond_data: Vec<crate::core::bond::BondData>,
    frame_provider: Option<Arc<dyn FrameProvider>>,
    needs_topology: bool,
) {
    sim_data.trajectory = trajectory;
    sim_data.frame_provider = frame_provider;
    sim_data.atom_data = atom_data;
    sim_data.bond_data = bond_data;
    sim_data.loaded = true;
    sim_data.needs_topology = needs_topology;
}

/// Result of a background file load.
type LoadResult = (
    Trajectory,
    Vec<AtomData>,
    Vec<crate::core::bond::BondData>,
    Option<Arc<dyn FrameProvider>>,
    bool,
);

/// Tracks an in-flight async file load.
#[derive(Resource, Default)]
pub struct AsyncLoadState {
    pub in_progress: bool,
    receiver: Option<crossbeam_channel::Receiver<Result<LoadResult, String>>>,
    pending_path: Option<PathBuf>,
}

/// Queue async file loads on a background thread.
pub fn handle_load_file_events(
    mut load_events: EventReader<LoadFileEvent>,
    mut async_state: ResMut<AsyncLoadState>,
    cli_topology: Res<CliTopologyArg>,
) {
    if load_events.is_empty() || async_state.in_progress {
        return;
    }

    if let Some(event) = load_events.read().next() {
        info!("Queueing async load: {:?}", event.path);

        let path = event.path.clone();
        let topology = cli_topology.0.clone();
        let (tx, rx) = crossbeam_channel::unbounded();
        async_state.receiver = Some(rx);
        async_state.pending_path = Some(path.clone());
        async_state.in_progress = true;

        std::thread::spawn(move || {
            let result = load_file(&path, topology.as_deref()).map_err(|e| e.to_string());
            let _ = tx.send(result);
        });
    }
}

/// Apply completed async loads on the main thread.
pub fn poll_async_load(
    mut commands: Commands,
    mut async_state: ResMut<AsyncLoadState>,
    mut sim_data: ResMut<SimulationData>,
    mut load_success: EventWriter<FileLoadedEvent>,
    mut load_error: EventWriter<FileLoadErrorEvent>,
    mut diagnostics: ResMut<crate::performance::PerformanceDiagnostics>,
) {
    let Some(receiver) = async_state.receiver.take() else {
        return;
    };

    match receiver.try_recv() {
        Ok(Ok((trajectory, atom_data, bond_data, frame_provider, needs_topology))) => {
            let path = async_state.pending_path.take().unwrap_or_default();
            info!(
                "Async load complete: {} atoms, {} frames{}",
                atom_data.len(),
                if frame_provider.is_some() {
                    frame_provider.as_ref().map(|p| p.num_frames()).unwrap_or(0)
                } else {
                    trajectory.num_frames()
                },
                if needs_topology {
                    " (needs topology)"
                } else {
                    ""
                }
            );

            apply_load_result(
                &mut sim_data,
                trajectory.clone(),
                atom_data.clone(),
                bond_data,
                frame_provider,
                needs_topology,
            );

            let format = FileFormat::from_path(&path);
            commands.insert_resource(FileHandle::new(path.clone(), format));

            diagnostics.estimated_bytes =
                crate::performance::memory::estimate_simulation_bytes(&sim_data);
            diagnostics.memory_warning = crate::performance::memory::memory_warning(&sim_data);

            load_success.send(FileLoadedEvent {
                path,
                num_atoms: atom_data.len(),
                num_frames: sim_data.num_frames(),
            });

            async_state.in_progress = false;
        }
        Ok(Err(err)) => {
            let path = async_state.pending_path.take().unwrap_or_default();
            error!("Async load failed {:?}: {}", path, err);
            load_error.send(FileLoadErrorEvent { path, error: err });
            async_state.in_progress = false;
        }
        Err(crossbeam_channel::TryRecvError::Empty) => {
            async_state.receiver = Some(receiver);
        }
        Err(crossbeam_channel::TryRecvError::Disconnected) => {
            async_state.in_progress = false;
        }
    }
}

/// Legacy sync loader kept for tests and benchmarks.
pub fn handle_load_file_events_sync(
    mut commands: Commands,
    mut load_events: EventReader<LoadFileEvent>,
    mut load_success: EventWriter<FileLoadedEvent>,
    mut load_error: EventWriter<FileLoadErrorEvent>,
    mut sim_data: ResMut<SimulationData>,
    file_handle: Option<ResMut<FileHandle>>,
) {
    // Early return if no events
    if load_events.is_empty() {
        return;
    }

    for event in load_events.read() {
        info!("Received load file event: {:?}", event.path);

        // Attempt to load the file
        match load_file(&event.path, None) {
            Ok((trajectory, atom_data, bond_data, frame_provider, needs_topology)) => {
                info!(
                    "Successfully loaded file: {} atoms, {} frames, {} bonds",
                    atom_data.len(),
                    if frame_provider.is_some() {
                        frame_provider.as_ref().map(|p| p.num_frames()).unwrap_or(0)
                    } else {
                        trajectory.num_frames()
                    },
                    bond_data.len()
                );

                apply_load_result(
                    &mut sim_data,
                    trajectory.clone(),
                    atom_data.clone(),
                    bond_data,
                    frame_provider,
                    needs_topology,
                );

                // Update file handle - handle resource outside of event loop
                let format = FileFormat::from_path(&event.path);
                let handle = FileHandle::new(event.path.clone(), format);

                if file_handle.is_some() {
                    // File handle resource already exists, update it
                    // This will be handled by the resource system
                    commands.insert_resource(handle);
                } else {
                    // Create new file handle resource
                    commands.insert_resource(handle);
                }

                // Send success event
                load_success.send(FileLoadedEvent {
                    path: event.path.clone(),
                    num_atoms: atom_data.len(),
                    num_frames: sim_data.num_frames(),
                });
            }
            Err(_e) => {
                error!("Failed to load file {:?}: {:?}", event.path, _e);

                // Send error event
                load_error.send(FileLoadErrorEvent {
                    path: event.path.clone(),
                    error: _e.to_string(),
                });
            }
        }
    }
}

/// Load a file provided via CLI argument
pub fn load_cli_file(cli_arg: Res<CliFileArg>, mut load_events: EventWriter<LoadFileEvent>) {
    if let Some(ref path) = cli_arg.0 {
        if path.exists() {
            info!("Loading CLI-provided file: {:?}", path);
            load_events.send(LoadFileEvent { path: path.clone() });
        } else {
            warn!("CLI file not found: {:?}", path);
        }
    }
}

/// Startup system to load a default file (optional)
pub fn load_default_file(_commands: Commands, mut load_events: EventWriter<LoadFileEvent>) {
    // Check if there's a test file in the examples directory
    let test_files = vec![
        "examples/test.xyz",
        "examples/test.pdb",
        "test.xyz",
        "test.pdb",
    ];

    for file in test_files {
        let path = PathBuf::from(file);
        if path.exists() {
            info!("Found test file, loading: {:?}", path);
            load_events.send(LoadFileEvent { path });
            return;
        }
    }

    info!("No default file found, starting empty");
}

/// Print simulation data statistics
pub fn print_simulation_data(sim_data: Res<SimulationData>) {
    if sim_data.loaded {
        info!(
            "Simulation data: {} atoms, {} frames, {:.2} fs",
            sim_data.num_atoms(),
            sim_data.num_frames(),
            sim_data.total_time()
        );
    }
}

/// Apply topology file to the currently loaded DCD trajectory.
pub fn handle_load_topology_events(
    mut events: EventReader<LoadTopologyEvent>,
    mut sim_data: ResMut<SimulationData>,
    mut topology_state: ResMut<TopologyState>,
    mut applied: EventWriter<TopologyAppliedEvent>,
    mut load_error: EventWriter<FileLoadErrorEvent>,
) {
    for event in events.read() {
        match load_topology(&event.path) {
            Ok((atom_data, bond_data)) => {
                match sim_data.apply_topology(atom_data.clone(), bond_data.clone()) {
                    Ok(()) => {
                        topology_state.path = Some(event.path.clone());
                        applied.send(TopologyAppliedEvent {
                            topology_path: event.path.clone(),
                            num_atoms: atom_data.len(),
                        });
                        info!("Applied topology from {:?}", event.path);
                    }
                    Err(e) => {
                        load_error.send(FileLoadErrorEvent {
                            path: event.path.clone(),
                            error: e,
                        });
                    }
                }
            }
            Err(e) => {
                load_error.send(FileLoadErrorEvent {
                    path: event.path.clone(),
                    error: e.to_string(),
                });
            }
        }
    }
}

/// Track DCD loads that still need a topology file.
pub fn track_topology_requirement(
    mut load_events: EventReader<FileLoadedEvent>,
    file_handle: Option<Res<FileHandle>>,
    mut topology_state: ResMut<TopologyState>,
    sim_data: Res<SimulationData>,
) {
    for event in load_events.read() {
        if sim_data.needs_topology {
            if let Some(handle) = file_handle.as_ref() {
                if handle.format == FileFormat::DCD {
                    topology_state.pending_dcd = Some(event.path.clone());
                }
            }
        }
    }
}

/// Register loading resources and events. Systems are registered centrally in systems::register.
pub fn register(app: &mut App) {
    let (cli_path, cli_topology) = parse_cli_args();

    app.init_resource::<SimulationData>()
        .insert_resource(CliFileArg(cli_path))
        .insert_resource(CliTopologyArg(cli_topology))
        .init_resource::<TopologyState>()
        .init_resource::<AsyncLoadState>()
        .add_event::<LoadFileEvent>()
        .add_event::<FileLoadedEvent>()
        .add_event::<FileLoadErrorEvent>()
        .add_event::<LoadTopologyEvent>()
        .add_event::<TopologyAppliedEvent>();

    info!("Loading resources registered");
}

/// Parse CLI arguments: `cargo run -- traj.dcd --topology struct.pdb`
fn parse_cli_args() -> (Option<PathBuf>, Option<PathBuf>) {
    let args: Vec<String> = std::env::args().collect();
    let mut trajectory = None;
    let mut topology = None;
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--topology" | "-t" => {
                i += 1;
                if i < args.len() {
                    topology = Some(PathBuf::from(&args[i]));
                }
            }
            arg if !arg.starts_with('-') && trajectory.is_none() => {
                trajectory = Some(PathBuf::from(arg));
            }
            _ => {}
        }
        i += 1;
    }

    (trajectory, topology)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_format_detection() {
        assert_eq!(
            FileFormat::from_path(Path::new("test.xyz")),
            FileFormat::XYZ
        );
        assert_eq!(
            FileFormat::from_path(Path::new("test.pdb")),
            FileFormat::PDB
        );
        assert_eq!(
            FileFormat::from_path(Path::new("test.gro")),
            FileFormat::GRO
        );
        assert_eq!(
            FileFormat::from_path(Path::new("test.dcd")),
            FileFormat::DCD
        );
    }

    #[test]
    fn test_content_format_detection() {
        let xyz_content = "3\nwater\nO 0.0 0.0 0.0\nH 0.0 0.9 0.0\nH 0.0 -0.9 0.0";
        assert_eq!(FileFormat::from_content(xyz_content), FileFormat::XYZ);

        let pdb_content = "ATOM      1  N   ALA A   1       0.000   0.000   0.000";
        assert_eq!(FileFormat::from_content(pdb_content), FileFormat::PDB);

        assert_eq!(
            FileFormat::from_bytes(&84_i32.to_le_bytes()),
            FileFormat::DCD
        );
    }

    #[test]
    fn test_simulation_data() {
        let trajectory = Trajectory::new(PathBuf::from("test"), 100, 1.0);
        let atom_data = (0..100)
            .map(|i| {
                AtomData::new(
                    i,
                    crate::core::atom::Element::C,
                    0,
                    "UNK".to_string(),
                    "A".to_string(),
                    format!("C{}", i),
                )
            })
            .collect();

        let sim_data = SimulationData::new(trajectory, atom_data);
        assert!(sim_data.loaded);
        assert_eq!(sim_data.num_atoms(), 100);
    }

    #[test]
    fn test_file_handle() {
        let path = PathBuf::from("test.xyz");
        let handle = FileHandle::new(path.clone(), FileFormat::XYZ);
        assert_eq!(handle.path, path);
        assert_eq!(handle.format, FileFormat::XYZ);
        assert_eq!(handle.name, "test.xyz");
    }
}
