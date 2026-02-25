use std::path::PathBuf;

// Include the parser directly
include!("../src/io/gro.rs");

fn main() {
    let path = PathBuf::from("examples/water.gro");

    println!("Testing GRO parser with file: {:?}", path);
    println!("File exists: {}", path.exists());

    if path.exists() {
        // Read file content
        let content = std::fs::read_to_string(&path).unwrap();
        println!("\n--- File Content ---");
        println!("{}", content);

        // Parse the file
        println!("\n--- Parsing ---");
        match GroParser::parse_file(&path) {
            Ok(trajectory) => {
                println!("✅ Successfully parsed!");
                println!("Title: {}", trajectory.metadata.title);
                println!("Software: {}", trajectory.metadata.software);
                println!("Atoms: {}", trajectory.num_atoms);
                println!("Frames: {}", trajectory.num_frames());

                // Get frame data
                if let Some(frame) = trajectory.get_frame(0) {
                    println!("\n--- Atom Data ---");
                    for atom_id in 0..trajectory.num_atoms as u32 {
                        if let Some(pos) = frame.get_position(atom_id) {
                            println!("Atom {}: {:?}", atom_id, pos);

                            // Check for velocities
                            if let Some(velocities) = &frame.velocities {
                                if let Some(vel) = velocities.get(&atom_id) {
                                    println!("  Velocity: {:?}", vel);
                                }
                            }
                        }
                    }

                    // Check for box dimensions
                    if let Some(box_size) = frame.box_size {
                        println!("\n--- Box Dimensions ---");
                        println!("Box: {:.4} x {:.4} x {:.4} nm",
                            box_size[0], box_size[1], box_size[2]);
                    }
                }

                println!("\n✅ All tests passed!");
            }
            Err(e) => {
                println!("❌ Error parsing file: {:?}", e);
                std::process::exit(1);
            }
        }
    } else {
        println!("❌ File not found!");
        std::process::exit(1);
    }
}
