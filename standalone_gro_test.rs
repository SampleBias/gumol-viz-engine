use std::fs;
use std::io::BufRead;

fn main() {
    println!("=== GRO Parser Standalone Test ===\n");

    // Test 1: Water molecule
    println!("Test 1: Water molecule (examples/water.gro)");
    test_water_gro();

    // Test 2: Alanine protein
    println!("\nTest 2: Alanine protein (examples/alanine.gro)");
    test_alanine_gro();

    println!("\n=== All Tests Passed! ===");
}

fn test_water_gro() {
    let path = "examples/water.gro";
    let file = fs::File::open(path).expect("Could not open water.gro");
    let reader = std::io::BufReader::new(file);

    // Read all lines
    let lines: Vec<String> = reader.lines()
        .map(|l| l.expect("Could not read line"))
        .collect();

    println!("File has {} lines", lines.len());

    // Parse title
    assert!(lines.len() >= 1, "Should have at least title line");
    let title = &lines[0];
    println!("Title: '{}'", title);
    assert!(title.contains("Water"), "Title should contain 'Water'");

    // Parse atom count
    assert!(lines.len() >= 2, "Should have atom count line");
    let atom_count: usize = lines[1].trim().parse().expect("Could not parse atom count");
    println!("Atom count: {}", atom_count);
    assert_eq!(atom_count, 3, "Should have 3 atoms");

    // Parse atom lines
    assert!(lines.len() >= 5, "Should have 3 atom lines + box line");
    
    // Atom 1: Oxygen
    let atom1_line = lines[2];
    println!("\nAtom 1 (Oxygen): '{}'", atom1_line);
    assert!(atom1_line.len() >= 68, "Atom line should be >= 68 chars with velocities");
    let x1 = atom1_line[20..28].trim().parse::<f32>().expect("Invalid X");
    let y1 = atom1_line[28..36].trim().parse::<f32>().expect("Invalid Y");
    let z1 = atom1_line[36..44].trim().parse::<f32>().expect("Invalid Z");
    assert!((x1 - 0.126).abs() < 0.001, "Oxygen X should be ~0.126");
    assert!((y1 - 0.639).abs() < 0.001, "Oxygen Y should be ~0.639");
    assert!((z1 - 0.322).abs() < 0.001, "Oxygen Z should be ~0.322");
    
    // Atom 2: Hydrogen 1
    let atom2_line = lines[3];
    println!("Atom 2 (H1): '{}'", atom2_line);
    let x2 = atom2_line[20..28].trim().parse::<f32>().expect("Invalid X");
    let y2 = atom2_line[28..36].trim().parse::<f32>().expect("Invalid Y");
    let z2 = atom2_line[36..44].trim().parse::<f32>().expect("Invalid Z");
    assert!((x2 - 0.187).abs() < 0.001, "H1 X should be ~0.187");
    
    // Atom 3: Hydrogen 2
    let atom3_line = lines[4];
    println!("Atom 3 (H2): '{}'", atom3_line);
    let x3 = atom3_line[20..28].trim().parse::<f32>().expect("Invalid X");
    let y3 = atom3_line[28..36].trim().parse::<f32>().expect("Invalid Y");
    let z3 = atom3_line[36..44].trim().parse::<f32>().expect("Invalid Z");
    assert!((x3 - 0.145).abs() < 0.001, "H2 X should be ~0.145");

    // Parse box dimensions
    let box_line = lines[5];
    println!("\nBox line: '{}'", box_line);
    let box_parts: Vec<&str> = box_line.split_whitespace().collect();
    assert_eq!(box_parts.len(), 3, "Should have 3 box dimensions");
    let xx = box_parts[0].parse::<f32>().expect("Invalid box xx");
    let yy = box_parts[1].parse::<f32>().expect("Invalid box yy");
    let zz = box_parts[2].parse::<f32>().expect("Invalid box zz");
    assert_eq!(xx, 0.0, "Box xx should be 0.0");
    assert_eq!(yy, 0.0, "Box yy should be 0.0");
    assert_eq!(zz, 0.0, "Box zz should be 0.0");

    // Parse velocities
    let vx1 = atom1_line[44..52].trim().parse::<f32>().expect("Invalid vx1");
    let vy1 = atom1_line[52..60].trim().parse::<f32>().expect("Invalid vy1");
    let vz1 = atom1_line[60..68].trim().parse::<f32>().expect("Invalid vz1");
    assert!((vx1 - 0.0001).abs() < 0.0001, "Oxygen vx should be ~0.0001");
    assert!((vy1 - 0.0002).abs() < 0.0001, "Oxygen vy should be ~0.0002");
    assert!((vz1 - 0.0003).abs() < 0.0001, "Oxygen vz should be ~0.0003");

    println!("\n✅ Water molecule test passed!");
}

fn test_alanine_gro() {
    let path = "examples/alanine.gro";
    let file = fs::File::open(path).expect("Could not open alanine.gro");
    let reader = std::io::BufReader::new(file);

    // Read all lines
    let lines: Vec<String> = reader.lines()
        .map(|l| l.expect("Could not read line"))
        .collect();

    println!("File has {} lines", lines.len());

    // Parse title
    let title = &lines[0];
    println!("Title: '{}'", title);
    assert!(title.contains("Alanine"), "Title should contain 'Alanine'");

    // Parse atom count
    let atom_count: usize = lines[1].trim().parse().expect("Could not parse atom count");
    println!("Atom count: {}", atom_count);
    assert_eq!(atom_count, 22, "Should have 22 atoms");

    // Parse first atom
    let first_atom_line = lines[2];
    println!("\nFirst atom: '{}'", first_atom_line);
    assert!(first_atom_line.len() >= 44, "Atom line should be >= 44 chars");
    let residue_name = &first_atom_line[5..10].trim();
    assert_eq!(residue_name, "ALA", "Residue should be ALA");

    println!("✅ Alanine test passed!");
}
