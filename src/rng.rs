//! Kleiner Zufallsgenerator (xorshift64) für Spiel und Deko – keine Abhängigkeit nötig.

#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// splitmix64 als Startwert: verteilt auch kleine oder ähnliche Seeds über alle Bits.
    pub fn new(seed: u64) -> Self {
        let mut z = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        Self((z ^ (z >> 31)).max(1))
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Gleichverteilt in [0, 1).
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Gleichverteilt in [lo, hi).
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit() as f32
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.unit() < p
    }

    pub fn digit(&mut self) -> char {
        (b'0' + (self.next_u64() % 10) as u8) as char
    }

    pub fn weighted<T: Copy>(&mut self, items: &[(T, f64)]) -> T {
        let total: f64 = items.iter().map(|(_, w)| w).sum();
        let mut r = self.unit() * total;
        for &(item, w) in items {
            r -= w;
            if r < 0.0 {
                return item;
            }
        }
        items[items.len() - 1].0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_respects_weights() {
        let mut rng = Rng::new(7);
        let picks = (0..10_000)
            .filter(|_| rng.weighted(&[(true, 3.0), (false, 1.0)]))
            .count();
        assert!((7_000..8_000).contains(&picks), "{picks}");
    }
}
