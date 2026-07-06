//! Shared helpers for criterion benchmarks.

#![allow(dead_code)]

use bevy::prelude::*;
use gumol_viz_engine::core::atom::{AtomData, Element};
use gumol_viz_engine::core::trajectory::{FrameData, Trajectory};
use std::collections::HashMap;
use std::path::PathBuf;

pub fn synthetic_atom_data(count: usize) -> Vec<AtomData> {
    (0..count)
        .map(|i| {
            AtomData::new(
                i as u32,
                if i % 3 == 0 {
                    Element::C
                } else if i % 3 == 1 {
                    Element::H
                } else {
                    Element::O
                },
                (i / 10) as u32,
                "UNK".into(),
                "A".into(),
                format!("A{i}"),
            )
        })
        .collect()
}

pub fn synthetic_positions(count: usize) -> HashMap<u32, Vec3> {
    (0..count)
        .map(|i| {
            let id = i as u32;
            (
                id,
                Vec3::new(
                    (i as f32 * 1.5).sin() * 10.0,
                    (i as f32 * 0.7).cos() * 10.0,
                    (i as f32 * 0.3).sin() * 10.0,
                ),
            )
        })
        .collect()
}

pub fn synthetic_trajectory(atom_count: usize, frame_count: usize) -> Trajectory {
    let mut trajectory = Trajectory::new(PathBuf::from("bench.xyz"), atom_count, 1.0);
    for f in 0..frame_count {
        let mut frame = FrameData::new(f, f as f32);
        for i in 0..atom_count {
            frame.set_position(i as u32, Vec3::new(i as f32 * 0.1, f as f32 * 0.01, 0.0));
        }
        trajectory.frames.push(frame);
    }
    trajectory
}
