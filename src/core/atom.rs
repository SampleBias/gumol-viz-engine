//! Atom component and data structures

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Atom component attached to Bevy entities
#[derive(Component, Clone, Debug, Reflect, Default)]
#[reflect(Component)]
pub struct Atom {
    /// Unique atom identifier
    pub id: u32,
    /// Element type
    pub element: Element,
    /// Current position (updated by timeline system)
    pub position: Vec3,
    /// Residue identifier
    pub residue_id: u32,
    /// Residue name (e.g., "ALA", "GLY")
    pub residue_name: String,
    /// Chain identifier
    pub chain_id: String,
    /// B-factor (temperature factor)
    pub b_factor: f32,
    /// Occupancy
    pub occupancy: f32,
    /// Atom name (e.g., "CA", "N", "O")
    pub name: String,
}

/// Static atom data (loaded once)
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Debug, Serialize, Deserialize)]
pub struct AtomData {
    /// Unique atom identifier
    pub id: u32,
    /// Element type
    pub element: Element,
    /// Residue identifier
    pub residue_id: u32,
    /// Residue name
    pub residue_name: String,
    /// Chain identifier
    pub chain_id: String,
    /// Atom name
    pub name: String,
    /// Charge
    pub charge: f32,
    /// Mass
    pub mass: f32,
    /// Position (for PDB files)
    pub position: Vec3,
    /// Occupancy
    pub occupancy: f32,
    /// B-factor (temperature factor)
    pub b_factor: f32,
}

impl AtomData {
    /// Create a new atom data structure
    pub fn new(
        id: u32,
        element: Element,
        residue_id: u32,
        residue_name: String,
        chain_id: String,
        name: String,
    ) -> Self {
        Self {
            id,
            element,
            residue_id,
            residue_name,
            chain_id,
            name,
            charge: 0.0,
            mass: element.mass(),
            position: Vec3::ZERO,
            occupancy: 1.0,
            b_factor: 0.0,
        }
    }
}

/// Chemical element enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
#[reflect(Debug, PartialEq, Hash)]
pub enum Element {
    // Period 1
    H,
    He,
    // Period 2
    Li,
    Be,
    B,
    C,
    N,
    O,
    F,
    Ne,
    // Period 3
    Na,
    Mg,
    Al,
    Si,
    P,
    S,
    Cl,
    Ar,
    // Period 4
    K,
    Ca,
    Sc,
    Ti,
    V,
    Cr,
    Mn,
    Fe,
    Co,
    Ni,
    Cu,
    Zn,
    Ga,
    Ge,
    As,
    Se,
    Br,
    Kr,
    // Period 5
    Rb,
    Sr,
    Y,
    Zr,
    Nb,
    Mo,
    Tc,
    Ru,
    Rh,
    Pd,
    Ag,
    Cd,
    In,
    Sn,
    Sb,
    Te,
    I,
    Xe,
    // Period 6
    Cs,
    Ba,
    La,
    Ce,
    Pr,
    Nd,
    Pm,
    Sm,
    Eu,
    Gd,
    Tb,
    Dy,
    Ho,
    Er,
    Tm,
    Yb,
    Lu,
    Hf,
    Ta,
    W,
    Re,
    Os,
    Ir,
    Pt,
    Au,
    Hg,
    Tl,
    Pb,
    Bi,
    Po,
    At,
    Rn,
    // Period 7
    Fr,
    Ra,
    Ac,
    Th,
    Pa,
    U,
    Np,
    Pu,
    Am,
    Cm,
    Bk,
    Cf,
    Es,
    Fm,
    Md,
    No,
    Lr,
    /// Unknown or custom element
    Unknown,
}

impl Default for Element {
    fn default() -> Self {
        Element::Unknown
    }
}

impl Element {
    /// Get the atomic number
    pub fn atomic_number(&self) -> u32 {
        match self {
            Element::H => 1,
            Element::He => 2,
            Element::Li => 3,
            Element::Be => 4,
            Element::B => 5,
            Element::C => 6,
            Element::N => 7,
            Element::O => 8,
            Element::F => 9,
            Element::Ne => 10,
            Element::Na => 11,
            Element::Mg => 12,
            Element::Al => 13,
            Element::Si => 14,
            Element::P => 15,
            Element::S => 16,
            Element::Cl => 17,
            Element::Ar => 18,
            Element::K => 19,
            Element::Ca => 20,
            Element::Sc => 21,
            Element::Ti => 22,
            Element::V => 23,
            Element::Cr => 24,
            Element::Mn => 25,
            Element::Fe => 26,
            Element::Co => 27,
            Element::Ni => 28,
            Element::Cu => 29,
            Element::Zn => 30,
            _ => 0,
        }
    }

    /// Get the atomic symbol
    pub fn symbol(&self) -> &'static str {
        match self {
            Element::H => "H",
            Element::He => "He",
            Element::Li => "Li",
            Element::Be => "Be",
            Element::B => "B",
            Element::C => "C",
            Element::N => "N",
            Element::O => "O",
            Element::F => "F",
            Element::Ne => "Ne",
            Element::Na => "Na",
            Element::Mg => "Mg",
            Element::Al => "Al",
            Element::Si => "Si",
            Element::P => "P",
            Element::S => "S",
            Element::Cl => "Cl",
            Element::Ar => "Ar",
            Element::K => "K",
            Element::Ca => "Ca",
            Element::Sc => "Sc",
            Element::Ti => "Ti",
            Element::V => "V",
            Element::Cr => "Cr",
            Element::Mn => "Mn",
            Element::Fe => "Fe",
            Element::Co => "Co",
            Element::Ni => "Ni",
            Element::Cu => "Cu",
            Element::Zn => "Zn",
            Element::Ga => "Ga",
            Element::Ge => "Ge",
            Element::As => "As",
            Element::Se => "Se",
            Element::Br => "Br",
            Element::Kr => "Kr",
            Element::Rb => "Rb",
            Element::Sr => "Sr",
            Element::Y => "Y",
            Element::Zr => "Zr",
            Element::Nb => "Nb",
            Element::Mo => "Mo",
            Element::Tc => "Tc",
            Element::Ru => "Ru",
            Element::Rh => "Rh",
            Element::Pd => "Pd",
            Element::Ag => "Ag",
            Element::Cd => "Cd",
            Element::In => "In",
            Element::Sn => "Sn",
            Element::Sb => "Sb",
            Element::Te => "Te",
            Element::I => "I",
            Element::Xe => "Xe",
            Element::Cs => "Cs",
            Element::Ba => "Ba",
            Element::La => "La",
            Element::Ce => "Ce",
            Element::Pr => "Pr",
            Element::Nd => "Nd",
            Element::Pm => "Pm",
            Element::Sm => "Sm",
            Element::Eu => "Eu",
            Element::Gd => "Gd",
            Element::Tb => "Tb",
            Element::Dy => "Dy",
            Element::Ho => "Ho",
            Element::Er => "Er",
            Element::Tm => "Tm",
            Element::Yb => "Yb",
            Element::Lu => "Lu",
            Element::Hf => "Hf",
            Element::Ta => "Ta",
            Element::W => "W",
            Element::Re => "Re",
            Element::Os => "Os",
            Element::Ir => "Ir",
            Element::Pt => "Pt",
            Element::Au => "Au",
            Element::Hg => "Hg",
            Element::Tl => "Tl",
            Element::Pb => "Pb",
            Element::Bi => "Bi",
            Element::Po => "Po",
            Element::At => "At",
            Element::Rn => "Rn",
            Element::Fr => "Fr",
            Element::Ra => "Ra",
            Element::Ac => "Ac",
            Element::Th => "Th",
            Element::Pa => "Pa",
            Element::U => "U",
            Element::Np => "Np",
            Element::Pu => "Pu",
            Element::Am => "Am",
            Element::Cm => "Cm",
            Element::Bk => "Bk",
            Element::Cf => "Cf",
            Element::Es => "Es",
            Element::Fm => "Fm",
            Element::Md => "Md",
            Element::No => "No",
            Element::Lr => "Lr",
            Element::Unknown => "X",
        }
    }

    /// Get the CPK (Corey-Pauling-Koltun) color for this element
    pub fn cpk_color(&self) -> [f32; 3] {
        match self {
            Element::H => [0.9, 0.9, 0.9],    // white
            Element::C => [0.2, 0.2, 0.2],    // gray
            Element::N => [0.1, 0.1, 0.8],    // blue
            Element::O => [0.8, 0.1, 0.1],    // red
            Element::F => [0.5, 0.8, 0.5],    // light green
            Element::P => [0.8, 0.5, 0.1],    // orange
            Element::S => [0.8, 0.8, 0.1],    // yellow
            Element::Cl => [0.1, 0.8, 0.1],   // green
            Element::Br => [0.5, 0.2, 0.2],   // dark red
            Element::I => [0.4, 0.1, 0.4],    // purple
            Element::He => [0.9, 0.0, 0.9],   // magenta
            Element::Li => [0.7, 0.0, 0.7],   // dark purple
            Element::Be => [0.5, 0.5, 0.5],   // light gray
            Element::B => [1.0, 0.7, 0.7],    // pink
            Element::Na => [0.5, 0.5, 0.5],   // light gray
            Element::Mg => [0.0, 0.0, 0.0],   // black
            Element::Al => [0.5, 0.5, 0.5],   // light gray
            Element::Si => [0.9, 0.7, 0.5],   // light brown
            Element::K => [0.5, 0.5, 0.5],    // light gray
            Element::Ca => [0.2, 0.8, 0.2],  // light green
            Element::Ti => [0.6, 0.6, 0.6],  // medium gray
            Element::Fe => [0.8, 0.2, 0.2],   // dark red
            Element::Cu => [0.7, 0.4, 0.2],   // brown
            Element::Zn => [0.4, 0.5, 0.4],   // dark green
            Element::Ag => [0.7, 0.7, 0.7],   // silver
            Element::Au => [0.8, 0.7, 0.2],   // gold
            _ => [0.5, 0.5, 0.5],             // gray (default)
        }
    }

    /// Get the CPK van der Waals radius (in Angstroms)
    pub fn vdw_radius(&self) -> f32 {
        match self {
            Element::H => 1.20,
            Element::He => 1.40,
            Element::Li => 1.82,
            Element::Be => 1.53,
            Element::B => 1.92,
            Element::C => 1.70,
            Element::N => 1.55,
            Element::O => 1.52,
            Element::F => 1.47,
            Element::Ne => 1.54,
            Element::Na => 2.27,
            Element::Mg => 1.73,
            Element::Al => 1.84,
            Element::Si => 2.10,
            Element::P => 1.80,
            Element::S => 1.80,
            Element::Cl => 1.75,
            Element::Ar => 1.88,
            Element::K => 2.75,
            Element::Ca => 2.31,
            Element::Sc => 2.15,
            Element::Ti => 2.11,
            Element::V => 2.07,
            Element::Cr => 2.06,
            Element::Mn => 2.05,
            Element::Fe => 2.04,
            Element::Co => 2.00,
            Element::Ni => 1.97,
            Element::Cu => 1.96,
            Element::Zn => 2.01,
            Element::Ga => 1.87,
            Element::Ge => 2.11,
            Element::As => 1.85,
            Element::Se => 1.90,
            Element::Br => 1.85,
            Element::Kr => 2.02,
            Element::Rb => 3.03,
            Element::Sr => 2.49,
            Element::Y => 2.32,
            Element::Zr => 2.23,
            Element::Nb => 2.18,
            Element::Mo => 2.17,
            Element::Tc => 2.16,
            Element::Ru => 2.13,
            Element::Rh => 2.10,
            Element::Pd => 2.10,
            Element::Ag => 2.11,
            Element::Cd => 2.18,
            Element::In => 2.20,
            Element::Sn => 2.17,
            Element::Sb => 2.06,
            Element::Te => 2.06,
            Element::I => 1.98,
            Element::Xe => 2.16,
            _ => 1.70, // Default to carbon
        }
    }

    /// Get the atomic mass (in atomic mass units)
    pub fn mass(&self) -> f32 {
        match self {
            Element::H => 1.008,
            Element::He => 4.003,
            Element::Li => 6.941,
            Element::Be => 9.012,
            Element::B => 10.811,
            Element::C => 12.011,
            Element::N => 14.007,
            Element::O => 15.999,
            Element::F => 18.998,
            Element::Ne => 20.180,
            Element::Na => 22.990,
            Element::Mg => 24.305,
            Element::Al => 26.982,
            Element::Si => 28.086,
            Element::P => 30.974,
            Element::S => 32.065,
            Element::Cl => 35.453,
            Element::Ar => 39.948,
            Element::K => 39.098,
            Element::Ca => 40.078,
            Element::Sc => 44.956,
            Element::Ti => 47.867,
            Element::V => 50.942,
            Element::Cr => 51.996,
            Element::Mn => 54.938,
            Element::Fe => 55.845,
            Element::Co => 58.933,
            Element::Ni => 58.693,
            Element::Cu => 63.546,
            Element::Zn => 65.409,
            Element::Ga => 69.723,
            Element::Ge => 72.64,
            Element::As => 74.922,
            Element::Se => 78.96,
            Element::Br => 79.904,
            Element::Kr => 83.798,
            Element::Rb => 85.468,
            Element::Sr => 87.62,
            Element::Y => 88.906,
            Element::Zr => 91.224,
            Element::Nb => 92.906,
            Element::Mo => 95.94,
            Element::Tc => 98.0,
            Element::Ru => 101.07,
            Element::Rh => 102.91,
            Element::Pd => 106.42,
            Element::Ag => 107.87,
            Element::Cd => 112.41,
            Element::In => 114.82,
            Element::Sn => 118.71,
            Element::Sb => 121.76,
            Element::Te => 127.60,
            Element::I => 126.90,
            Element::Xe => 131.29,
            Element::Cs => 132.91,
            Element::Ba => 137.33,
            Element::La => 138.91,
            Element::Ce => 140.12,
            Element::Pr => 140.91,
            Element::Nd => 144.24,
            Element::Pm => 145.0,
            Element::Sm => 150.36,
            Element::Eu => 151.96,
            Element::Gd => 157.25,
            Element::Tb => 158.93,
            Element::Dy => 162.50,
            Element::Ho => 164.93,
            Element::Er => 167.26,
            Element::Tm => 168.93,
            Element::Yb => 173.04,
            Element::Lu => 174.97,
            Element::Hf => 178.49,
            Element::Ta => 180.95,
            Element::W => 183.84,
            Element::Re => 186.21,
            Element::Os => 190.23,
            Element::Ir => 192.22,
            Element::Pt => 195.08,
            Element::Au => 196.97,
            Element::Hg => 200.59,
            Element::Tl => 204.38,
            Element::Pb => 207.2,
            Element::Bi => 208.98,
            Element::Th => 232.04,
            Element::Pa => 231.04,
            Element::U => 238.03,
            _ => 12.0,
        }
    }

    /// Parse an element from its symbol string
    pub fn from_symbol(s: &str) -> Result<Self, String> {
        let s_upper = s.to_uppercase();
        Ok(match s_upper.as_str() {
            "H" => Element::H,
            "HE" => Element::He,
            "LI" => Element::Li,
            "BE" => Element::Be,
            "B" => Element::B,
            "C" => Element::C,
            "N" => Element::N,
            "O" => Element::O,
            "F" => Element::F,
            "NE" => Element::Ne,
            "NA" => Element::Na,
            "MG" => Element::Mg,
            "AL" => Element::Al,
            "SI" => Element::Si,
            "P" => Element::P,
            "S" => Element::S,
            "CL" => Element::Cl,
            "AR" => Element::Ar,
            "K" => Element::K,
            "CA" => Element::Ca,
            "SC" => Element::Sc,
            "TI" => Element::Ti,
            "V" => Element::V,
            "CR" => Element::Cr,
            "MN" => Element::Mn,
            "FE" => Element::Fe,
            "CO" => Element::Co,
            "NI" => Element::Ni,
            "CU" => Element::Cu,
            "ZN" => Element::Zn,
            "GA" => Element::Ga,
            "GE" => Element::Ge,
            "AS" => Element::As,
            "SE" => Element::Se,
            "BR" => Element::Br,
            "KR" => Element::Kr,
            "RB" => Element::Rb,
            "SR" => Element::Sr,
            "Y" => Element::Y,
            "ZR" => Element::Zr,
            "NB" => Element::Nb,
            "MO" => Element::Mo,
            "TC" => Element::Tc,
            "RU" => Element::Ru,
            "RH" => Element::Rh,
            "PD" => Element::Pd,
            "AG" => Element::Ag,
            "CD" => Element::Cd,
            "IN" => Element::In,
            "SN" => Element::Sn,
            "SB" => Element::Sb,
            "TE" => Element::Te,
            "I" => Element::I,
            "XE" => Element::Xe,
            "CS" => Element::Cs,
            "BA" => Element::Ba,
            "LA" => Element::La,
            "CE" => Element::Ce,
            "PR" => Element::Pr,
            "ND" => Element::Nd,
            "PM" => Element::Pm,
            "SM" => Element::Sm,
            "EU" => Element::Eu,
            "GD" => Element::Gd,
            "TB" => Element::Tb,
            "DY" => Element::Dy,
            "HO" => Element::Ho,
            "ER" => Element::Er,
            "TM" => Element::Tm,
            "YB" => Element::Yb,
            "LU" => Element::Lu,
            "HF" => Element::Hf,
            "TA" => Element::Ta,
            "W" => Element::W,
            "RE" => Element::Re,
            "OS" => Element::Os,
            "IR" => Element::Ir,
            "PT" => Element::Pt,
            "AU" => Element::Au,
            "HG" => Element::Hg,
            "TL" => Element::Tl,
            "PB" => Element::Pb,
            "BI" => Element::Bi,
            "TH" => Element::Th,
            "PA" => Element::Pa,
            "U" => Element::U,
            _ => return Err(format!("Unknown element: {}", s)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_from_symbol() {
        assert_eq!(Element::from_symbol("C").unwrap(), Element::C);
        assert_eq!(Element::from_symbol("H").unwrap(), Element::H);
        assert_eq!(Element::from_symbol("FE").unwrap(), Element::Fe);
        assert!(Element::from_symbol("XX").is_err());
    }

    #[test]
    fn test_cpk_colors() {
        let carbon_color = Element::C.cpk_color();
        assert_eq!(carbon_color, [0.2, 0.2, 0.2]);

        let oxygen_color = Element::O.cpk_color();
        assert_eq!(oxygen_color, [0.8, 0.1, 0.1]);
    }

    #[test]
    fn test_vdw_radius() {
        assert_eq!(Element::H.vdw_radius(), 1.20);
        assert_eq!(Element::C.vdw_radius(), 1.70);
        assert_eq!(Element::O.vdw_radius(), 1.52);
    }

    #[test]
    fn test_atomic_mass() {
        assert!((Element::H.mass() - 1.008).abs() < 0.01);
        assert!((Element::C.mass() - 12.011).abs() < 0.01);
    }
}
