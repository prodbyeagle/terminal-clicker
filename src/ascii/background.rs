//! ASCII-Hintergrund: Gitter aus Zeichenzellen, pro Zelle ein 3D-Value-Noise
//! n(x, y, t). `t` läuft langsam weiter – das ergibt das sanfte Wabern.

const RAMP: &[char] = &[' ', ' ', '.', '·', ':', '-', '=', '+', '*', '#'];

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

/// Erzeugt das komplette Gitter als Zeilen-Strings (eine Zeile = ein Text-Element,
/// nicht jede Zelle einzeln). `bias` verschiebt die Rampe: mehr = dichter.
pub fn render_rows(cols: usize, rows: usize, t: f32, bias: f32) -> Vec<String> {
    (0..rows)
        .map(|y| {
            (0..cols)
                .map(|x| {
                    // Zellen sind etwa doppelt so hoch wie breit → y stärker skalieren.
                    let n = fbm(x as f32 * 0.07, y as f32 * 0.15, t * 0.35);
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
    fn noise_is_smooth_but_not_flat() {
        let a = value_noise(1.0, 1.0, 1.0);
        let b = value_noise(1.001, 1.0, 1.0);
        assert!((a - b).abs() < 0.01);
        let spread = (0..50).map(|i| value_noise(i as f32 * 0.7, 0.3, 0.5));
        let (lo, hi) = spread.fold((1f32, 0f32), |(lo, hi), v| (lo.min(v), hi.max(v)));
        assert!(hi - lo > 0.3, "{lo}..{hi}");
    }

    #[test]
    fn rows_have_requested_size() {
        let rows = render_rows(40, 10, 0.0, 0.0);
        assert_eq!(rows.len(), 10);
        assert!(rows.iter().all(|r| r.chars().count() == 40));
    }

    #[test]
    fn noise_stays_in_range() {
        for i in 0..1000 {
            let n = value_noise(
                i as f32 * 0.13,
                i as f32 * 0.27,
                i as f32 * 0.09,
            );
    
            assert!((0.0..=1.0).contains(&n));
        }
    }
    
    #[test]
    fn rendering_is_deterministic() {
        let a = render_rows(40, 10, 1.5, 0.0);
        let b = render_rows(40, 10, 1.5, 0.0);
    
        assert_eq!(a, b);
    }
    
    #[test]
    fn empty_dimensions_work() {
        assert!(render_rows(0, 10, 0.0, 0.0).iter().all(String::is_empty));
        assert!(render_rows(10, 0, 0.0, 0.0).is_empty());
    }
}
