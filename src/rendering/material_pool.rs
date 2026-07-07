//! Pre-created CPK materials for all element types (one `StandardMaterial` per element).

use crate::core::atom::Element;
use bevy::prelude::*;
use std::collections::HashMap;

/// Pool of CPK-colored materials keyed by element (initialized once at startup).
#[derive(Resource, Debug)]
pub struct MaterialPool {
    materials: HashMap<Element, Handle<StandardMaterial>>,
}

impl MaterialPool {
    /// Pre-create one CPK material per element variant.
    pub fn initialize(materials: &mut Assets<StandardMaterial>) -> Self {
        let mut map = HashMap::with_capacity(Element::all_variants().len());
        for &element in Element::all_variants() {
            let color = element.cpk_color();
            let handle = materials.add(StandardMaterial {
                base_color: Color::srgb(color[0], color[1], color[2]),
                metallic: 0.1,
                perceptual_roughness: 0.2,
                ..default()
            });
            map.insert(element, handle);
        }
        Self { materials: map }
    }

    /// Lookup a pre-created material for an element.
    pub fn get(&self, element: Element) -> Handle<StandardMaterial> {
        self.materials
            .get(&element)
            .cloned()
            .unwrap_or_else(|| self.materials[&Element::Unknown].clone())
    }

    /// Number of pooled materials (all element variants).
    pub fn len(&self) -> usize {
        self.materials.len()
    }

    pub fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }
}

pub fn register(app: &mut App) {
    app.add_systems(Startup, init_material_pool);
}

fn init_material_pool(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    let pool = MaterialPool::initialize(&mut materials);
    info!(
        "Material pool initialized with {} CPK materials",
        pool.len()
    );
    commands.insert_resource(pool);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_pool_has_all_elements() {
        let mut materials = Assets::<StandardMaterial>::default();
        let pool = MaterialPool::initialize(&mut materials);
        assert_eq!(pool.len(), Element::all_variants().len());
        for &element in Element::all_variants() {
            assert!(pool.materials.contains_key(&element));
        }
    }

    #[test]
    fn test_material_pool_cpk_colors() {
        let mut materials = Assets::<StandardMaterial>::default();
        let pool = MaterialPool::initialize(&mut materials);
        for element in [Element::C, Element::O, Element::N] {
            let handle = pool.get(element);
            let mat = materials.get(&handle).unwrap();
            let cpk = element.cpk_color();
            assert!((mat.base_color.to_srgba().red - cpk[0]).abs() < 0.01);
            assert!((mat.base_color.to_srgba().green - cpk[1]).abs() < 0.01);
            assert!((mat.base_color.to_srgba().blue - cpk[2]).abs() < 0.01);
        }
    }
}
