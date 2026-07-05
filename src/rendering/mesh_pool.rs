//! Shared atom sphere meshes keyed by LOD level (avoids per-element duplication).

use crate::core::atom::Element;
use crate::rendering::generate_atom_mesh_sphere;
use crate::rendering::lod::AtomLod;
use bevy::prelude::*;
use std::collections::HashMap;

/// Cache of atom sphere meshes by (quantized radius, LOD).
#[derive(Resource, Default, Debug)]
pub struct AtomMeshPool {
    meshes: HashMap<(u32, AtomLod), Handle<Mesh>>,
    current_lod: AtomLod,
}

impl AtomMeshPool {
    fn radius_key(radius: f32) -> u32 {
        (radius * 1000.0).round() as u32
    }

    pub fn current_lod(&self) -> AtomLod {
        self.current_lod
    }

    pub fn set_current_lod(&mut self, lod: AtomLod) {
        self.current_lod = lod;
    }

    /// Get or create a shared sphere mesh for the given radius and LOD.
    pub fn get_atom_mesh(
        &mut self,
        assets: &mut Assets<Mesh>,
        element: Element,
        lod: AtomLod,
    ) -> Handle<Mesh> {
        let radius = element.vdw_radius() * 0.5;
        let key = (Self::radius_key(radius), lod);

        if let Some(handle) = self.meshes.get(&key) {
            return handle.clone();
        }

        let (lat, lon) = lod.mesh_resolution();
        let mesh = assets.add(generate_atom_mesh_sphere(radius, lat, lon));
        self.meshes.insert(key, mesh.clone());
        mesh
    }

    pub fn clear(&mut self) {
        self.meshes.clear();
        self.current_lod = AtomLod::default();
    }
}

pub fn register(app: &mut App) {
    app.init_resource::<AtomMeshPool>();
}
