//! Color utility functions and conversions

use bevy::prelude::*;

/// Convert CPK color name to Bevy Color
pub fn cpk_color_to_bevy(color_name: &str) -> Color {
    match color_name {
        "white" => Color::srgb(1.0, 1.0, 1.0),
        "red" => Color::srgb(1.0, 0.0, 0.0),
        "blue" => Color::srgb(0.0, 0.0, 1.0),
        "green" => Color::srgb(0.0, 1.0, 0.0),
        "yellow" => Color::srgb(1.0, 1.0, 0.0),
        "orange" => Color::srgb(1.0, 0.5, 0.0),
        "purple" => Color::srgb(0.5, 0.0, 0.5),
        "cyan" => Color::srgb(0.0, 1.0, 1.0),
        "magenta" => Color::srgb(1.0, 0.0, 1.0),
        "gray" => Color::srgb(0.5, 0.5, 0.5),
        "dark_gray" => Color::srgb(0.25, 0.25, 0.25),
        "light_gray" => Color::srgb(0.75, 0.75, 0.75),
        "black" => Color::srgb(0.0, 0.0, 0.0),
        "brown" => Color::srgb(0.6, 0.4, 0.2),
        "pink" => Color::srgb(1.0, 0.75, 0.8),
        _ => Color::srgb(1.0, 1.0, 1.0),
    }
}

/// Parse hex color string to Bevy Color
pub fn hex_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

    Some(Color::srgb_u8(r, g, b))
}

/// Create a gradient from two colors
pub fn gradient(color_a: Color, color_b: Color, t: f32) -> Color {
    // Convert to Srgba for linear interpolation
    let ca = color_a.to_srgba();
    let cb = color_b.to_srgba();

    Color::srgba(
        ca.red + (cb.red - ca.red) * t,
        ca.green + (cb.green - ca.green) * t,
        ca.blue + (cb.blue - ca.blue) * t,
        ca.alpha + (cb.alpha - ca.alpha) * t,
    )
}

/// Interpolate between multiple colors
pub fn color_map(colors: &[Color], t: f32) -> Color {
    if colors.is_empty() {
        return Color::WHITE;
    }
    if colors.len() == 1 {
        return colors[0];
    }

    let num_segments = (colors.len() - 1) as f32;
    let t_clamped = t.clamp(0.0, 1.0);
    let segment = (t_clamped * num_segments).floor() as usize;
    let segment_t = (t_clamped * num_segments) - segment as f32;

    let next_segment = (segment + 1).min(colors.len() - 1);
    gradient(colors[segment], colors[next_segment], segment_t)
}

/// Get a color from a palette by index
pub fn palette_color(index: usize, palette: &[Color]) -> Color {
    if palette.is_empty() {
        return Color::WHITE;
    }
    palette[index % palette.len()]
}

/// Default color palette for molecules
pub fn default_palette() -> Vec<Color> {
    vec![
        Color::srgb(0.9, 0.9, 0.9),  // White
        Color::srgb(0.9, 0.1, 0.1),  // Red
        Color::srgb(0.1, 0.1, 0.9),  // Blue
        Color::srgb(0.1, 0.8, 0.1),  // Green
        Color::srgb(0.9, 0.9, 0.1),  // Yellow
        Color::srgb(0.9, 0.6, 0.1),  // Orange
        Color::srgb(0.6, 0.1, 0.9),  // Purple
        Color::srgb(0.1, 0.9, 0.9),  // Cyan
    ]
}
