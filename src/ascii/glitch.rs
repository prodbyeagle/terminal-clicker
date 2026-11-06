//! ASCII-Rauschen: Text entschlüsselt sich, Ziffern scramblen.

use crate::rng::Rng;
use std::time::Duration;

/// Das Rauschen wechselt alle 40 ms.
const TICK_MS: u128 = 40;

const GLITCH: &[char] = &[
    '#', '%', '&', '@', '$', '*', '+', '=', '?', '/', '\\', '<', '>', '[', ']', '{', '}', '!', '~',
    '^', '0', '1', ':', ';',
];

fn glitch_char(i: usize, tick: u64) -> char {
    let mut h =
        (i as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ tick.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    h ^= h >> 31;
    GLITCH[(h % GLITCH.len() as u64) as usize]
}

/// Text, der aus zufälligen ASCII-Zeichen in den echten Text übergeht.
/// `progress` 0…1; Zeichen lösen sich von links nach rechts auf.
/// `age` lässt das Rauschen flackern; gleich alte Texte flackern gleich.
pub fn decode_text(text: &str, progress: f32, age: Duration) -> String {
    let tick = (age.as_millis() / TICK_MS) as u64;
    if progress >= 1.0 {
        return text.to_string();
    }
    let n = text.chars().count().max(1) as f32;
    text.chars()
        .enumerate()
        .map(|(i, c)| {
            let resolve_at = 0.25 + 0.75 * (i as f32 / n);
            if c == ' ' || progress >= resolve_at {
                c
            } else {
                glitch_char(i, tick)
            }
        })
        .collect()
}

pub fn scramble_frames(rng: &mut Rng, width: usize, count: usize) -> Vec<String> {
    (0..count)
        .map(|_| (0..width).map(|_| rng.digit()).collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_resolves_left_to_right() {
        let text = "> installiert cron.d";
        let start = decode_text(text, 0.0, Duration::ZERO);
        assert_eq!(start.chars().count(), text.chars().count());
        assert_ne!(start, text);
        // Leerzeichen bleiben, damit die Wortform schon zu sehen ist.
        assert_eq!(start.chars().nth(1), Some(' '));
        assert_eq!(decode_text(text, 1.0, Duration::ZERO), text);
        let mid = decode_text(text, 0.6, Duration::ZERO);
        assert!(mid.starts_with("> inst"));
    }
}
