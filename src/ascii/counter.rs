//! Zählwerk: Ziffern rollen beim Ändern einzeln durch.

use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollDir {
    /// Wert gestiegen: neue Ziffer kommt von unten.
    Up,
    /// Wert gesunken (z. B. nach einem Kauf): neue Ziffer kommt von oben.
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DigitRoll {
    pub prev: char,
    pub cur: char,
    pub dir: RollDir,
    start: Option<Instant>,
}

/// Zähler, dessen Ziffern beim Ändern einzeln kurz durchrollen – wie bei
/// einem mechanischen Zählwerk. Jede Stelle rollt nur, wenn sie sich ändert.
#[derive(Debug, Clone)]
pub struct CounterRoll {
    digits: Vec<DigitRoll>,
    last_value: f64,
    duration: Duration,
}

impl CounterRoll {
    pub fn new(duration_ms: u64) -> Self {
        Self {
            digits: Vec::new(),
            last_value: 0.,
            duration: Duration::from_millis(duration_ms),
        }
    }

    /// Neuen Wert übernehmen. `animate = false` setzt ihn ohne Rollen.
    pub fn update(&mut self, value: f64, text: &str, animate: bool, now: Instant) {
        let chars: Vec<char> = text.chars().collect();
        let dir = if value >= self.last_value {
            RollDir::Up
        } else {
            RollDir::Down
        };
        let old = std::mem::take(&mut self.digits);

        // Rechtsbündig vergleichen, damit 9999 → 10000 die Einer richtig zuordnet.
        let offset = chars.len() as isize - old.len() as isize;
        self.digits = chars
            .iter()
            .enumerate()
            .map(|(i, &c)| {
                let before = usize::try_from(i as isize - offset)
                    .ok()
                    .and_then(|j| old.get(j).copied());
                let rolling = |d: &DigitRoll| {
                    d.start
                        .is_some_and(|t| now.saturating_duration_since(t) < self.duration)
                };
                match before {
                    Some(d) if d.cur == c => d,
                    // Ändert sich eine Ziffer schneller, als sie rollt (hohes Einkommen),
                    // nicht neu starten – sonst hinge die Anzeige ständig hinterher.
                    // Die laufende Rolle landet einfach auf dem neuesten Wert.
                    Some(d) if animate && rolling(&d) => DigitRoll { cur: c, ..d },
                    Some(d) if animate => DigitRoll {
                        prev: d.cur,
                        cur: c,
                        dir,
                        start: Some(now),
                    },
                    // Neue Stelle vorne (z. B. 9999 → 10000) rollt aus dem Leeren herein.
                    None if animate => DigitRoll {
                        prev: ' ',
                        cur: c,
                        dir,
                        start: Some(now),
                    },
                    _ => DigitRoll {
                        prev: c,
                        cur: c,
                        dir,
                        start: None,
                    },
                }
            })
            .collect();
        self.last_value = value;
    }

    pub fn digits(&self) -> &[DigitRoll] {
        &self.digits
    }

    /// Fortschritt 0…1 mit Ease-out, `None` wenn die Stelle gerade still steht.
    pub fn progress(&self, digit: &DigitRoll, now: Instant) -> Option<f32> {
        let start = digit.start?;
        let t = now.saturating_duration_since(start).as_secs_f32()
            / self.duration.as_secs_f32().max(0.001);
        (t < 1.0).then(|| 1.0 - (1.0 - t).powi(3))
    }

    pub fn is_active(&self, now: Instant) -> bool {
        self.digits.iter().any(|d| self.progress(d, now).is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_rolls_only_changed_digits() {
        let t0 = Instant::now();
        let mut c = CounterRoll::new(150);
        c.update(41., "0041", false, t0);
        c.update(42., "0042", true, t0);
        let rolling: Vec<bool> = c
            .digits()
            .iter()
            .map(|d| c.progress(d, t0).is_some())
            .collect();
        assert_eq!(rolling, [false, false, false, true]);
        let last = c.digits()[3];
        assert_eq!((last.prev, last.cur, last.dir), ('1', '2', RollDir::Up));
        assert!(!c.is_active(t0 + Duration::from_millis(200)));
    }

    #[test]
    fn fast_changes_land_on_the_newest_value_without_restarting() {
        let t0 = Instant::now();
        let mut c = CounterRoll::new(150);
        c.update(1., "0001", false, t0);
        c.update(2., "0002", true, t0);
        c.update(3., "0003", true, t0 + Duration::from_millis(50));
        assert_eq!(c.digits()[3].cur, '3');
        // Ein Neustart bei 50 ms liefe bis 200 ms; die erste Rolle endet bei 150 ms.
        assert!(!c.is_active(t0 + Duration::from_millis(160)));
    }

    #[test]
    fn counter_rolls_down_and_widens() {
        let t0 = Instant::now();
        let mut c = CounterRoll::new(150);
        c.update(9999., "9999", false, t0);
        c.update(10000., "10000", true, t0);
        assert_eq!(c.digits().len(), 5);
        assert_eq!(c.digits()[4].prev, '9');
        c.update(500., "0500", true, t0 + Duration::from_millis(200));
        assert_eq!((c.digits()[1].cur, c.digits()[1].dir), ('5', RollDir::Down));
    }
}
