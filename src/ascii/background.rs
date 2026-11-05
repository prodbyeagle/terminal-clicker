//! ASCII-Hintergrund: Gitter aus Zeichenzellen, pro Zelle ein 3D-Value-Noise
//! n(x, y, t). `t` läuft langsam weiter – das ergibt das sanfte Wabern.

/// Zeichenrampe von leer nach dicht.
pub const RAMP: &[char] = &[' ', ' ', '.', '·', ':', '-', '=', '+', '*', '#'];

/// Zeitfaktor pro Geschwindigkeitsstufe (langsam, mittel, schnell).
pub const SPEEDS: [f32; 3] = [0.08, 0.18, 0.4];

/// Verschiebung der Rampe pro Dichtestufe (niedrig, mittel, hoch).
const DENSITY_BIAS: [f32; 3] = [-0.18, 0.0, 0.16];

fn hash(x: i32, y: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343)
        ^ (y as u32).wrapping_mul(0xd816_3841)
        ^ (z as u32).wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    (h & 0x00ff_ffff) as f32 / 0x00ff_ffff as f32
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Value-Noise in 3D, Ergebnis in 0.0 … 1.0.
pub fn value_noise(x: f32, y: f32, z: f32) -> f32 {
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

/// Zwei Oktaven, normalisiert auf 0.0 … 1.0.
pub fn fbm(x: f32, y: f32, z: f32) -> f32 {
    let a = value_noise(x, y, z);
    let b = value_noise(x * 2.03 + 17.0, y * 2.03 + 5.0, z * 1.7 + 3.0);
    (a * 0.67 + b * 0.33).clamp(0.0, 1.0)
}

/// Erzeugt das komplette Gitter als Zeilen-Strings
/// (eine Zeile = ein Text-Element, nicht jede Zelle einzeln).
pub fn render_rows(cols: usize, rows: usize, t: f32, density: u8) -> Vec<String> {
    let bias = DENSITY_BIAS[(density as usize).min(2)];
    (0..rows)
        .map(|y| {
            (0..cols)
                .map(|x| {
                    // Zellen sind etwa doppelt so hoch wie breit → y stärker skalieren.
                    let n = fbm(x as f32 * 0.07, y as f32 * 0.15, t);
                    let v = ((n - 0.5) * 1.8 + 0.4 + bias).clamp(0.0, 0.999);
                    RAMP[((v * RAMP.len() as f32) as usize).min(RAMP.len() - 1)]
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_in_range_and_smooth() {
        for i in 0..200 {
            let v = fbm(i as f32 * 0.37, i as f32 * 0.11, 0.5);
            assert!((0.0..=1.0).contains(&v));
        }
        let a = value_noise(1.0, 1.0, 1.0);
        let b = value_noise(1.001, 1.0, 1.0);
        assert!((a - b).abs() < 0.01);
    }

    #[test]
    fn rows_have_requested_size() {
        let rows = render_rows(40, 10, 0.0, 1);
        assert_eq!(rows.len(), 10);
        assert!(rows.iter().all(|r| r.chars().count() == 40));
    }
}
