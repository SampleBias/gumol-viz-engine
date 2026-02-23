//! File loading system
//!
//! This system handles loading molecular trajectory files and storing
//! the parsed data in Bevy resources.

use crate::core::atom::AtomData;
use crate::core::trajectory::Trajectory;
use crate::core::atom::Element;
use crate::io::{FileFormat, IOResult};
use crate::io::xyz::XYZParser;
use crate::io::pdb::PDBParser;
use bevy::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// Resource containing the loaded simulation data
#[derive(Resource, Clone, Debug)]
pub struct SimulationData {
    /// The trajectory data
    pub trajectory: Trajectory,
    /// Atom metadata (static data that doesn't change between frames)
    pub atom_data: Vec<AtomData>,
    /// Whether data is loaded
    pub loaded: bool,
}

impl Default for SimulationData {
    fn default() -> Self {
        Self {
            trajectory: Trajectory::new(PathBuf::new(), 0, 1.0),
            atom_data: Vec::new(),
            loaded: false,
        }
    }
}

impl SimulationData {
    /// Create simulation data from a trajectory and atom data
    pub fn new(trajectory: Trajectory, atom_data: Vec<AtomData>) -> Self {
        Self {
            trajectory,
            atom_data,
            loaded: true,
        }
    }

    /// Get the number of atoms in the simulation
    pub fn num_atoms(&self) -> usize {
        self.trajectory.num_atoms
    }

    /// Get the number of frames in the trajectory
    pub fn num_frames(&self) -> usize {
        self.trajectory.num_frames()
    }

    /// Get the total simulation time
    pub fn total_time(&self) -> f32 {
        self.trajectory.total_time
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
fn load_file(path: &Path) -> IOResult<(Trajectory, Vec<AtomData>)> {
    let format = FileFormat::from_path(path);

    info!("Loading file: {:?} (format: {:?})", path, format);

    match format {
        FileFormat::XYZ => {
            let trajectory = XYZParser::parse_file(path)?;

            // For XYZ files, we need to create atom data from the first frame
            let atom_data = create_atom_data_from_xyz(&trajectory)?;

            Ok((trajectory, atom_data))
        }
        FileFormat::PDB => {
            // PDB parsing needs to return atom data
            let trajectory = PDBParser::parse_file(path)?;

            // Create atom data from the trajectory
            let atom_data = create_atom_data_from_pdb(&trajectory)?;

            Ok((trajectory, atom_data))
        }
        _ => Err(crate::io::IOError::UnsupportedFormat(format!("{:?}", format))),
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
                            0,                             // residue ID
                            "UNK".to_string(),             // residue name
                            "A".to_string(),               // chain ID
                            parts[0].to_string(),          // atom name (use element symbol)
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

/// Create atom data from PDB trajectory
fn create_atom_data_from_pdb(trajectory: &Trajectory) -> IOResult<Vec<AtomData>> {
    // For PDB files, we need to parse the atom data
    // This is a placeholder - in a real implementation, the PDB parser would return this
    let mut atom_data = Vec::new();

    if let Some(first_frame) = trajectory.get_frame(0) {
        for atom_id in first_frame.atom_ids() {
            atom_data.push(AtomData::new(
                *atom_id,
                crate::core::atom::Element::C, // Default to carbon
                0,
                "UNK".to_string(),
                "A".to_string(),
                format!("ATOM{}", atom_id),
            ));
        }
    }

    Ok(atom_data)
}

/// System to handle file loading events
pub fn handle_load_file_events(
    mut commands: Commands,
    mut load_events: EventReader<LoadFileEvent>,
    mut load_success: EventWriter<FileLoadedEvent>,
    mut load_error: EventWriter<FileLoadErrorEvent>,
    mut sim_data: ResMut<SimulationData>,
    mut file_handle: Option<ResMut<FileHandle>>,
) {
    // Early return if no events
    if load_events.is_empty() {
        return;
    }

    for event in load_events.read() {
        info!("Received load file event: {:?}", event.path);

        // Attempt to load the file
        match load_file(&event.path) {
            Ok((trajectory, atom_data)) => {
                info!(
                    "Successfully loaded file: {} atoms, {} frames",
                    atom_data.len(),
                    trajectory.num_frames()
                );

                // Update simulation data resource
                sim_data.trajectory = trajectory.clone();
                sim_data.atom_data = atom_data.clone();
                sim_data.loaded = true;

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
                    num_frames: trajectory.num_frames(),
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

/// Startup system to load a default file (optional)
pub fn load_default_file(
    mut commands: Commands,
    mut load_events: EventWriter<LoadFileEvent>,
) {
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

/// Register all loading systems
pub fn register(app: &mut App) {
    let cli_path = std::env::args()
        .nth(1)
        .map(PathBuf::from);

    app.init_resource::<SimulationData>()
        .insert_resource(CliFileArg(cli_path))
        .add_event::<LoadFileEvent>()
        .add_event::<FileLoadedEvent>()
        .add_event::<FileLoadErrorEvent>()
        .add_systems(Startup, load_cli_file)
        .add_systems(Update, handle_load_file_events)
        .add_systems(Update, print_simulation_data);

    info!("Loading systems registered");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_format_detection() {
        assert_eq!(FileFormat::from_path(Path::new("test.xyz")), FileFormat::XYZ);
        assert_eq!(FileFormat::from_path(Path::new("test.pdb")), FileFormat::PDB);
        assert_eq!(FileFormat::from_path(Path::new("test.gro")), FileFormat::GRO);
        assert_eq!(FileFormat::from_path(Path::new("test.dcd")), FileFormat::DCD);
    }

    #[test]
    fn test_content_format_detection() {
        let xyz_content = "3\nwater\nO 0.0 0.0 0.0\nH 0.0 0.9 0.0\nH 0.0 -0.9 0.0";
        assert_eq!(FileFormat::from_content(xyz_content), FileFormat::XYZ);

        let pdb_content = "ATOM      1  N   ALA A   1       0.000   0.000   0.000";
        assert_eq!(FileFormat::from_content(pdb_content), FileFormat::PDB);
    }

    #[test]
    fn test_simulation_data() {
        let trajectory = Trajectory::new(PathBuf::from("test"), 100, 1.0);
        let atom_data = (0..100)
            .map(|i| AtomData::new(i, crate::core::atom::Element::C, 0, "UNK".to_string(), "A".to_string(), format!("C{}", i)))
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
