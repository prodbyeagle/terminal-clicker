//! ASCII-Hintergrund: Gitter aus Zeichenzellen. Ein Muster liefert pro Zelle
//! einen Wert 0…1 zur Zeit `t`, ein Zeichensatz macht daraus ein Zeichen.

use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

/// Welches Muster den Hintergrund füllt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Pattern {
    /// Weiches Rauschen, das langsam wabert.
    #[default]
    Clouds,
    /// Überlagerte Sinuswellen.
    Waves,
    /// Klassischer Plasma-Effekt.
    Plasma,
    /// Ringe, die von der Mitte nach außen laufen.
    Rings,
    /// Fallende Zeichenspalten.
    Rain,
    /// Vereinzelte, funkelnde Punkte.
    Stars,
}

impl Pattern {
    pub const ALL: [Pattern; 6] = [
        Pattern::Clouds,
        Pattern::Waves,
        Pattern::Plasma,
        Pattern::Rings,
        Pattern::Rain,
        Pattern::Stars,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Pattern::Clouds => "wolken",
            Pattern::Waves => "wellen",
            Pattern::Plasma => "plasma",
            Pattern::Rings => "ringe",
            Pattern::Rain => "regen",
            Pattern::Stars => "sterne",
        }
    }
}

/// Aus welchen Zeichen der Hintergrund besteht – jeweils von leer nach dicht.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Charset {
    #[default]
    Classic,
    Blocks,
    Binary,
    Dots,
    Digits,
}

impl Charset {
    pub const ALL: [Charset; 5] = [
        Charset::Classic,
        Charset::Blocks,
        Charset::Binary,
        Charset::Dots,
        Charset::Digits,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Charset::Classic => "klassisch",
            Charset::Blocks => "blöcke",
            Charset::Binary => "binär",
            Charset::Dots => "punkte",
            Charset::Digits => "zahlen",
        }
    }

    /// Erstes Zeichen ist immer leer.
    fn ramp(self) -> &'static [char] {
        match self {
            Charset::Classic => &[' ', ' ', '.', '·', ':', '-', '=', '+', '*', '#'],
            Charset::Blocks => &[' ', ' ', '░', '░', '▒', '▒', '▓', '█'],
            Charset::Binary => &[' ', ' ', ' ', '0', '1', '1'],
            Charset::Dots => &[' ', ' ', '.', '·', '•', '●'],
            Charset::Digits => &[' ', ' ', '1', '2', '3', '4', '5', '6', '7', '8', '9'],
        }
    }
}

/// Alles, was das Aussehen bestimmt.
#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub pattern: Pattern,
    pub charset: Charset,
    /// Verschiebt die Dichte: positiv = mehr Zeichen, negativ = weniger.
    pub bias: f32,
    /// Größe der Formen: > 1 = gröber, < 1 = feiner.
    pub scale: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            pattern: Pattern::Clouds,
            charset: Charset::Classic,
            bias: 0.0,
            scale: 1.0,
        }
    }
}

fn hash(x: i32, y: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B9)
        ^ (y as u32).wrapping_mul(0x85EB_CA6B)
        ^ (z as u32).wrapping_mul(0xC2B2_AE35);

    h ^= h >> 16;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;

    (h >> 8) as f32 / 16_777_215.0
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Value-Noise in 3D, Ergebnis in 0.0 … 1.0.
fn value_noise(x: f32, y: f32, z: f32) -> f32 {
    let (xi, yi, zi) = (x.floor() as i32, y.floor() as i32, z.floor() as i32);
    let (xf, yf, zf) = (
        smooth(x - xi as f32),
        smooth(y - yi as f32),
        smooth(z - zi as f32),
    );
    let c = |dx, dy, dz| hash(xi + dx, yi + dy, zi + dz);
    let x00 = lerp(c(0, 0, 0), c(1, 0, 0), xf);
    let x10 = lerp(c(0, 1, 0), c(1, 1, 0), xf);
    let x01 = lerp(c(0, 0, 1), c(1, 0, 1), xf);
    let x11 = lerp(c(0, 1, 1), c(1, 1, 1), xf);
    lerp(lerp(x00, x10, yf), lerp(x01, x11, yf), zf)
}

fn fbm(x: f32, y: f32, z: f32) -> f32 {
    let a = value_noise(x, y, z);
    let b = value_noise(x * 2.03 + 17.0, y * 2.03 + 5.0, z * 1.7 + 3.0);
    (a * 0.67 + b * 0.33).clamp(0.0, 1.0)
}

/// Flächige Muster: Kontrast anheben und um `bias` verschieben.
fn shade(n: f32, bias: f32) -> f32 {
    (n - 0.5) * 1.8 + 0.4 + bias
}

/// Wert einer Zelle in 0…1. Zellen sind etwa doppelt so hoch wie breit,
/// deshalb wird `y` überall doppelt gewichtet.
fn sample(style: &Style, x: usize, y: usize, cols: usize, rows: usize, t: f32) -> f32 {
    let (xf, yf) = (x as f32 / style.scale, y as f32 * 2.0 / style.scale);
    let bias = style.bias;
    let v = match style.pattern {
        Pattern::Clouds => shade(fbm(xf * 0.07, yf * 0.075, t * 0.35), bias),
        Pattern::Waves => {
            let w = (xf * 0.16 + t * 2.0).sin() + (yf * 0.11 - t * 1.3 + xf * 0.04).sin();
            shade(0.5 + w * 0.25, bias)
        }
        Pattern::Plasma => {
            let (cx, cy) = (xf - cols as f32 / 2.0, yf - rows as f32);
            let d = (cx * cx + cy * cy).sqrt();
            let p = (xf * 0.09 + t).sin()
                + (yf * 0.07 - t * 1.3).sin()
                + ((xf + yf) * 0.05 + t * 0.7).sin()
                + (d * 0.08 - t).sin();
            shade(0.5 + p * 0.125, bias)
        }
        Pattern::Rings => {
            let (cx, cy) = (
                x as f32 - cols as f32 / 2.0,
                (y as f32 - rows as f32 / 2.0) * 2.0,
            );
            let d = (cx * cx + cy * cy).sqrt() / style.scale;
            shade(0.5 + 0.5 * (d * 0.35 - t * 3.0).sin(), bias)
        }
        Pattern::Rain => {
            // Jede Spalte hat eigene Geschwindigkeit, Länge und Startversatz.
            let column = x as i32;
            if hash(column, 0, 1) > 0.55 + bias * 2.0 {
                return 0.0;
            }
            let speed = 0.5 + hash(column, 0, 2);
            let tail = (6.0 + hash(column, 0, 3) * 10.0) * style.scale;
            let period = rows as f32 + tail + hash(column, 0, 4) * rows as f32;
            let head = (t * 40.0 * speed + hash(column, 0, 5) * period) % period;
            let behind = head - y as f32;
            if (0.0..tail).contains(&behind) {
                1.0 - behind / tail
            } else {
                0.0
            }
        }
        Pattern::Stars => {
            let (cell_x, cell_y) = (x as i32, y as i32);
            if hash(cell_x, cell_y, 7) < 0.965 - bias * 0.15 {
                return 0.0;
            }
            let twinkle = 0.5
                + 0.5
                    * (t * 8.0 * (0.5 + hash(cell_x, cell_y, 8)) + hash(cell_x, cell_y, 9) * TAU)
                        .sin();
            0.35 + 0.65 * twinkle
        }
    };
    v.clamp(0.0, 0.999)
}

/// Erzeugt das komplette Gitter als Zeilen-Strings (eine Zeile = ein Text-Element,
/// nicht jede Zelle einzeln).
pub fn render_rows(cols: usize, rows: usize, t: f32, style: &Style) -> Vec<String> {
    let ramp = style.charset.ramp();
    (0..rows)
        .map(|y| {
            (0..cols)
                .map(|x| {
                    let v = sample(style, x, y, cols, rows, t);
                    ramp[((v * ramp.len() as f32) as usize).min(ramp.len() - 1)]
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style(pattern: Pattern) -> Style {
        Style {
            pattern,
            ..Style::default()
        }
    }

    fn filled(rows: &[String]) -> usize {
        rows.iter()
            .flat_map(|r| r.chars())
            .filter(|c| *c != ' ')
            .count()
    }

    #[test]
    fn noise_is_smooth_but_not_flat() {
        let a = value_noise(1.0, 1.0, 1.0);
        let b = value_noise(1.001, 1.0, 1.0);
        assert!((a - b).abs() < 0.01);
        let spread = (0..50).map(|i| value_noise(i as f32 * 0.7, 0.3, 0.5));
        let (lo, hi) = spread.fold((1f32, 0f32), |(lo, hi), v| (lo.min(v), hi.max(v)));
        assert!(hi - lo > 0.3, "{lo}..{hi}");
    }

    #[test]
    fn noise_stays_in_range() {
        for i in 0..1000 {
            let n = value_noise(i as f32 * 0.13, i as f32 * 0.27, i as f32 * 0.09);
            assert!((0.0..=1.0).contains(&n));
        }
    }

    #[test]
    fn every_pattern_fills_the_grid_but_not_completely() {
        for pattern in Pattern::ALL {
            let rows = render_rows(60, 20, 1.5, &style(pattern));
            assert_eq!(rows.len(), 20, "{pattern:?}");
            assert!(rows.iter().all(|r| r.chars().count() == 60), "{pattern:?}");
            let n = filled(&rows);
            assert!(n > 0 && n < 60 * 20, "{pattern:?}: {n} gefüllt");
        }
    }

    #[test]
    fn every_pattern_moves_over_time() {
        for pattern in Pattern::ALL {
            let a = render_rows(60, 20, 1.0, &style(pattern));
            let b = render_rows(60, 20, 2.0, &style(pattern));
            assert_ne!(a, b, "{pattern:?} steht still");
        }
    }

    #[test]
    fn rendering_is_deterministic() {
        assert_eq!(
            render_rows(40, 10, 1.5, &Style::default()),
            render_rows(40, 10, 1.5, &Style::default())
        );
    }

    #[test]
    fn charsets_only_use_their_own_characters() {
        for charset in Charset::ALL {
            let rows = render_rows(
                60,
                20,
                1.5,
                &Style {
                    charset,
                    ..Style::default()
                },
            );
            assert!(
                rows.iter()
                    .flat_map(|r| r.chars())
                    .all(|c| charset.ramp().contains(&c)),
                "{charset:?}"
            );
        }
    }

    #[test]
    fn bias_controls_density() {
        for pattern in Pattern::ALL {
            let sparse = filled(&render_rows(
                60,
                20,
                1.5,
                &Style {
                    bias: -0.18,
                    ..style(pattern)
                },
            ));
            let dense = filled(&render_rows(
                60,
                20,
                1.5,
                &Style {
                    bias: 0.16,
                    ..style(pattern)
                },
            ));
            assert!(dense > sparse, "{pattern:?}: {sparse} vs {dense}");
        }
    }

    #[test]
    fn empty_dimensions_work() {
        assert!(
            render_rows(0, 10, 0.0, &Style::default())
                .iter()
                .all(String::is_empty)
        );
        assert!(render_rows(10, 0, 0.0, &Style::default()).is_empty());
    }
}
