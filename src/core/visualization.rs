//! Visualization styles and rendering modes

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Component that controls how an entity is rendered
#[derive(Component, Clone, Debug, Reflect, Default)]
#[reflect(Component)]
pub struct VisualizationStyle {
    /// Rendering mode for this entity
    pub render_mode: RenderMode,
    /// Custom color override (None means use default)
    pub color: Option<Color>,
    /// Scale factor for size
    pub scale: f32,
    /// Visibility
    pub visible: bool,
    /// Opacity (0.0 to 1.0)
    pub opacity: f32,
    /// Emissive glow intensity
    pub emissive: f32,
}

impl VisualizationStyle {
    /// Create a new visualization style
    pub fn new(render_mode: RenderMode) -> Self {
        Self {
            render_mode,
            color: None,
            scale: 1.0,
            visible: true,
            opacity: 1.0,
            emissive: 0.0,
        }
    }

    /// Create a CPK (space-filling) style
    pub fn cpk() -> Self {
        Self::new(RenderMode::CPK)
    }

    /// Create a ball-and-stick style
    pub fn ball_and_stick() -> Self {
        Self::new(RenderMode::BallAndStick)
    }

    /// Create a licorice style
    pub fn licorice() -> Self {
        Self::new(RenderMode::Licorice)
    }

    /// Create a wireframe style
    pub fn wireframe() -> Self {
        Self::new(RenderMode::Wireframe)
    }
}

/// Rendering mode for molecules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
#[reflect(Debug, PartialEq, Hash)]
pub enum RenderMode {
    /// CPK (space-filling) - atoms as van der Waals spheres
    CPK,
    /// Ball-and-stick - reduced atom size with bond cylinders
    BallAndStick,
    /// Licorice - small atoms with thick bonds
    Licorice,
    /// Wireframe - lines only
    Wireframe,
    /// Surface - molecular surface (solvent-accessible)
    Surface,
    /// Cartoon - protein secondary structure as ribbons
    Cartoon,
    /// Tube - smooth tube following backbone
    Tube,
    /// Trace - lines following backbone
    Trace,
    /// Points - small points at atom positions
    Points,
}

impl Default for RenderMode {
    fn default() -> Self {
        RenderMode::CPK
    }
}

impl RenderMode {
    /// Get the atom scale factor for this render mode
    pub fn atom_scale(&self) -> f32 {
        match self {
            RenderMode::CPK => 1.0,
            RenderMode::BallAndStick => 0.5,
            RenderMode::Licorice => 0.2,
            RenderMode::Wireframe => 0.0,
            RenderMode::Surface => 0.0,
            RenderMode::Cartoon => 0.0,
            RenderMode::Tube => 0.0,
            RenderMode::Trace => 0.0,
            RenderMode::Points => 0.3,
        }
    }

    /// Get the bond thickness multiplier for this render mode
    pub fn bond_thickness(&self) -> f32 {
        match self {
            RenderMode::CPK => 0.0,
            RenderMode::BallAndStick => 1.0,
            RenderMode::Licorice => 2.0,
            RenderMode::Wireframe => 0.1,
            RenderMode::Surface => 0.0,
            RenderMode::Cartoon => 0.0,
            RenderMode::Tube => 0.5,
            RenderMode::Trace => 0.1,
            RenderMode::Points => 0.0,
        }
    }

    /// Check if this render mode shows bonds
    pub fn shows_bonds(&self) -> bool {
        matches!(
            self,
            RenderMode::BallAndStick
                | RenderMode::Licorice
                | RenderMode::Wireframe
                | RenderMode::Tube
                | RenderMode::Trace
        )
    }

    /// Check if this render mode shows atoms as spheres
    pub fn shows_atoms(&self) -> bool {
        matches!(
            self,
            RenderMode::CPK | RenderMode::BallAndStick | RenderMode::Licorice | RenderMode::Points
        )
    }
}

/// Coloring scheme for molecules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
#[reflect(Debug, PartialEq, Hash)]
pub enum ColorScheme {
    /// CPK colors (element-based)
    CPK,
    /// Residue type
    Residue,
    /// Chain ID
    Chain,
    /// B-factor (temperature factor)
    BFactor,
    /// Secondary structure
    SecondaryStructure,
    /// Molecule type
    Molecule,
    /// Uniform single color
    Uniform,
    /// Gradient along x-axis
    GradientX,
    /// Gradient along y-axis
    GradientY,
    /// Gradient along z-axis
    GradientZ,
    /// Charge-based coloring
    Charge,
    /// Custom property
    Custom,
}

impl Default for ColorScheme {
    fn default() -> Self {
        ColorScheme::CPK
    }
}

/// Material properties for rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialProperties {
    /// Base color
    pub base_color: Color,
    /// Metallic factor (0.0 to 1.0)
    pub metallic: f32,
    /// Roughness factor (0.0 to 1.0)
    pub roughness: f32,
    /// Emissive color
    pub emissive: Color,
    /// Transmission (transparency, 0.0 to 1.0)
    pub transmission: f32,
    /// Refractive index
    pub ior: f32,
    /// Thickness for subsurface scattering
    pub thickness: f32,
}

impl Default for MaterialProperties {
    fn default() -> Self {
        Self {
            base_color: Color::WHITE,
            metallic: 0.1,
            roughness: 0.3,
            emissive: Color::BLACK,
            transmission: 0.0,
            ior: 1.5,
            thickness: 0.5,
        }
    }
}

impl MaterialProperties {
    /// Create a standard material for atoms
    pub fn atom_material(color: Color) -> Self {
        Self {
            base_color: color,
            metallic: 0.1,
            roughness: 0.2,
            ..default()
        }
    }

    /// Create a material for bonds
    pub fn bond_material() -> Self {
        Self {
            base_color: Color::srgb(0.7, 0.7, 0.7),
            metallic: 0.2,
            roughness: 0.3,
            ..default()
        }
    }

    /// Create a glass-like material
    pub fn glass_material(color: Color) -> Self {
        Self {
            base_color: color,
            metallic: 0.0,
            roughness: 0.0,
            transmission: 0.9,
            ior: 1.5,
            ..default()
        }
    }

    /// Create a metallic material
    pub fn metallic_material(color: Color) -> Self {
        Self {
            base_color: color,
            metallic: 0.9,
            roughness: 0.2,
            ..default()
        }
    }
}

/// Color palette for common coloring schemes
pub struct ColorPalette;

impl ColorPalette {
    /// Get residue color
    pub fn residue_color(residue_name: &str) -> Color {
        match residue_name.to_uppercase().as_str() {
            "ALA" => Color::srgb(0.9, 0.9, 0.1),    // yellow
            "ARG" => Color::srgb(0.1, 0.0, 0.9),    // blue
            "ASN" => Color::srgb(0.8, 0.7, 0.8),    // light purple
            "ASP" => Color::srgb(0.9, 0.1, 0.1),    // red
            "CYS" => Color::srgb(0.9, 0.9, 0.1),    // yellow
            "GLN" => Color::srgb(0.8, 0.7, 0.8),    // light purple
            "GLU" => Color::srgb(0.9, 0.1, 0.1),    // red
            "GLY" => Color::srgb(0.9, 0.9, 0.9),    // white
            "HIS" => Color::srgb(0.1, 0.5, 0.9),    // light blue
            "ILE" => Color::srgb(0.1, 0.9, 0.1),    // green
            "LEU" => Color::srgb(0.1, 0.9, 0.1),    // green
            "LYS" => Color::srgb(0.1, 0.0, 0.9),    // blue
            "MET" => Color::srgb(0.9, 0.9, 0.1),    // yellow
            "PHE" => Color::srgb(0.6, 0.1, 0.6),    // purple
            "PRO" => Color::srgb(0.9, 0.9, 0.1),    // yellow
            "SER" => Color::srgb(0.9, 0.9, 0.1),    // yellow
            "THR" => Color::srgb(0.9, 0.9, 0.1),    // yellow
            "TRP" => Color::srgb(0.6, 0.1, 0.6),    // purple
            "TYR" => Color::srgb(0.6, 0.1, 0.6),    // purple
            "VAL" => Color::srgb(0.1, 0.9, 0.1),    // green
            "HOH" | "WAT" => Color::srgb(0.1, 0.5, 0.9), // light blue (water)
            _ => Color::srgb(0.5, 0.5, 0.5),        // gray
        }
    }

    /// Get secondary structure color
    pub fn secondary_structure_color(ss: crate::core::molecule::SecondaryStructure) -> Color {
        use crate::core::molecule::SecondaryStructure;

        match ss {
            SecondaryStructure::AlphaHelix => Color::srgb(0.9, 0.1, 0.1),   // red
            SecondaryStructure::ThreeTenHelix => Color::srgb(0.9, 0.3, 0.1), // orange-red
            SecondaryStructure::PiHelix => Color::srgb(0.9, 0.2, 0.1),    // orange-red
            SecondaryStructure::BetaStrand | SecondaryStructure::BetaSheet => Color::srgb(0.1, 0.1, 0.9), // blue
            SecondaryStructure::Turn => Color::srgb(0.1, 0.9, 0.1),       // green
            SecondaryStructure::Coil | SecondaryStructure::Unknown => Color::srgb(0.9, 0.9, 0.9), // white
        }
    }

    /// Get chain color
    pub fn chain_color(chain_id: &str) -> Color {
        let hash = chain_id.chars().fold(0u32, |acc, c| acc.wrapping_mul(31).wrapping_add(c as u32));
        let hue = (hash % 360) as f32 / 360.0;
        Color::hsla(hue, 0.8, 0.5, 1.0)
    }

    /// Get b-factor color (blue to red gradient)
    pub fn b_factor_color(b_factor: f32, min_b_factor: f32, max_b_factor: f32) -> Color {
        let t = if max_b_factor > min_b_factor {
            (b_factor - min_b_factor) / (max_b_factor - min_b_factor)
        } else {
            0.5
        };
        let t = t.clamp(0.0, 1.0);
        Color::srgb(t, 0.0, 1.0 - t) // Blue to red
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_mode_scales() {
        assert_eq!(RenderMode::CPK.atom_scale(), 1.0);
        assert_eq!(RenderMode::BallAndStick.atom_scale(), 0.5);
        assert_eq!(RenderMode::Licorice.atom_scale(), 0.2);
    }

    #[test]
    fn test_render_mode_bonds() {
        assert!(RenderMode::BallAndStick.shows_bonds());
        assert!(!RenderMode::CPK.shows_bonds());
    }

    #[test]
    fn test_residue_colors() {
        let ala_color = ColorPalette::residue_color("ALA");
        let gly_color = ColorPalette::residue_color("GLY");
        assert!(ala_color != gly_color);
    }
}
