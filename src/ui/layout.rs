//! Maße, die mehrere Teile der Oberfläche teilen müssen.

pub const FONT: &str = "JetBrains Mono";
pub const FONT_SIZE: f32 = 14.;
pub const LINE_H: f32 = 20.;

/// Zeichenbreite der Monospace-Schrift: JetBrains Mono hat 600/1000 em Vorschub.
pub fn char_width(font_size: f32) -> f32 {
    font_size * 0.6
}
