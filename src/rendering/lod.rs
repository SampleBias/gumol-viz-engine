//! Level-of-detail selection for atom sphere meshes.

use bevy::prelude::*;

/// Sphere mesh quality levels (latitudes × longitudes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AtomLod {
    #[default]
    High,
    Medium,
    Low,
    Point,
}

impl AtomLod {
    pub fn mesh_resolution(self) -> (u32, u32) {
        match self {
            AtomLod::High => (16, 32),
            AtomLod::Medium => (8, 16),
            AtomLod::Low => (4, 8),
            AtomLod::Point => (2, 4),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            AtomLod::High => "High",
            AtomLod::Medium => "Medium",
            AtomLod::Low => "Low",
            AtomLod::Point => "Point",
        }
    }

    /// Select LOD from approximate screen-space size in pixels (with hysteresis).
    pub fn from_screen_pixels(pixels: f32, current: AtomLod) -> AtomLod {
        const HIGH_IN: f32 = 12.0;
        const MED_IN: f32 = 6.0;
        const LOW_IN: f32 = 2.0;
        const HIGH_OUT: f32 = 10.0;
        const MED_OUT: f32 = 4.0;
        const LOW_OUT: f32 = 1.0;

        match current {
            AtomLod::High if pixels < HIGH_OUT => AtomLod::Medium,
            AtomLod::Medium => {
                if pixels >= HIGH_IN {
                    AtomLod::High
                } else if pixels < MED_OUT {
                    AtomLod::Low
                } else {
                    AtomLod::Medium
                }
            }
            AtomLod::Low => {
                if pixels >= MED_IN {
                    AtomLod::Medium
                } else if pixels < LOW_OUT {
                    AtomLod::Point
                } else {
                    AtomLod::Low
                }
            }
            AtomLod::Point if pixels >= LOW_IN => AtomLod::Low,
            _ => current,
        }
    }
}

/// Estimate screen-space diameter in pixels for a world-space sphere.
pub fn estimate_screen_pixels(
    world_radius: f32,
    world_position: Vec3,
    camera_transform: &GlobalTransform,
    projection: &Projection,
    viewport_height: f32,
) -> f32 {
    let camera_pos = camera_transform.translation();
    let distance = camera_pos.distance(world_position).max(0.001);

    let world_diameter = world_radius * 2.0;

    match projection {
        Projection::Perspective(persp) => {
            let fov = persp.fov;
            let projected = (world_diameter / distance) / (2.0 * (fov * 0.5).tan());
            projected * viewport_height
        }
        Projection::Orthographic(ortho) => {
            let scale = viewport_height / ortho.scale;
            world_diameter * scale
        }
    }
}

/// Pick a single LOD for an entire element batch (all atoms share mesh).
pub fn select_batch_lod(
    sample_position: Vec3,
    world_radius: f32,
    camera_transform: &GlobalTransform,
    projection: &Projection,
    viewport_height: f32,
    current: AtomLod,
) -> AtomLod {
    let pixels = estimate_screen_pixels(
        world_radius,
        sample_position,
        camera_transform,
        projection,
        viewport_height,
    );
    AtomLod::from_screen_pixels(pixels, current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lod_hysteresis() {
        let lod = AtomLod::from_screen_pixels(5.0, AtomLod::Medium);
        assert_eq!(lod, AtomLod::Medium);
        let lod = AtomLod::from_screen_pixels(15.0, AtomLod::Medium);
        assert_eq!(lod, AtomLod::High);
    }
}
