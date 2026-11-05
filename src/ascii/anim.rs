//! Winzige Frame-Animationen aus Zeichen.
//!
//! Eine Animation ist eine Liste von Frames (Strings) plus Frame-Dauer.
//! Der `Animator` hält die laufenden Animationen mit Startzeit; `render`
//! fragt nur ab, welcher Frame zur vergangenen Zeit gehört.

use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimKind {
    /// `+N` steigt über dem Button auf. `x` ist der horizontale Versatz in px.
    Float { id: u64, x: f32 },
    /// Rahmen des Buttons zuckt.
    ButtonPulse,
    /// Upgrade-Zeile blinkt invertiert.
    RowBlink(usize),
    /// Zähler-Ziffern scramblen.
    Scramble,
    /// Datenpaket eines passiven Upgrades (`source` = Index in `UPGRADES`).
    /// `x`/`y` sind Anteile (0…1) der Hauptfläche.
    Packet { id: u64, source: usize, x: f32, y: f32 },
}

impl AnimKind {
    /// Gleichartige Animationen ersetzen sich gegenseitig (außer Floats).
    fn replaces(&self, other: &AnimKind) -> bool {
        match (self, other) {
            (AnimKind::Float { .. } | AnimKind::Packet { .. }, _) => false,
            (a, b) => a == b,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Anim {
    pub kind: AnimKind,
    pub frames: Vec<String>,
    pub frame_dur: Duration,
    pub start: Instant,
}

impl Anim {
    pub fn total(&self) -> Duration {
        self.frame_dur * self.frames.len() as u32
    }

    pub fn is_done(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.start) >= self.total()
    }

    pub fn frame_index(&self, now: Instant) -> Option<usize> {
        let elapsed = now.saturating_duration_since(self.start);
        let i = (elapsed.as_secs_f64() / self.frame_dur.as_secs_f64()) as usize;
        (i < self.frames.len()).then_some(i)
    }

    pub fn frame(&self, now: Instant) -> Option<&str> {
        self.frame_index(now).map(|i| self.frames[i].as_str())
    }

    /// 0.0 … 1.0 über die gesamte Laufzeit.
    pub fn progress(&self, now: Instant) -> f32 {
        let elapsed = now.saturating_duration_since(self.start).as_secs_f32();
        (elapsed / self.total().as_secs_f32().max(0.001)).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Default)]
pub struct Animator {
    anims: Vec<Anim>,
    next_id: u64,
    pub rng: Rng,
}

impl Animator {
    pub fn new() -> Self {
        Self {
            anims: Vec::new(),
            next_id: 0,
            rng: Rng::new(0x5eed_c11c),
        }
    }

    pub fn play(&mut self, kind: AnimKind, frames: Vec<String>, frame_ms: u64, now: Instant) {
        self.anims.retain(|a| !kind.replaces(&a.kind));
        self.anims.push(Anim {
            kind,
            frames,
            frame_dur: Duration::from_millis(frame_ms),
            start: now,
        });
    }

    pub fn next_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    pub fn prune(&mut self, now: Instant) {
        self.anims.retain(|a| !a.is_done(now));
    }

    pub fn clear(&mut self) {
        self.anims.clear();
    }

    pub fn is_active(&self) -> bool {
        !self.anims.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Anim> {
        self.anims.iter()
    }

    pub fn find(&self, kind: AnimKind) -> Option<&Anim> {
        self.anims.iter().find(|a| a.kind == kind)
    }
}

/// Kleiner xorshift-Zufall – reicht für Deko, keine Abhängigkeit nötig.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Default for Rng {
    fn default() -> Self {
        Self::new(0x9e37_79b9)
    }
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Gleichverteilt in [lo, hi).
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        let unit = (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32;
        lo + (hi - lo) * unit
    }

    pub fn digit(&mut self) -> char {
        (b'0' + (self.next_u64() % 10) as u8) as char
    }
}

/// Frames für den Meilenstein-Scramble: `count` zufällige Ziffernfolgen
/// in der Länge des Zielwerts.
pub fn scramble_frames(rng: &mut Rng, width: usize, count: usize) -> Vec<String> {
    (0..count)
        .map(|_| (0..width).map(|_| rng.digit()).collect())
        .collect()
}

/// Richtung, in die eine Ziffer rollt.
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
    pub start: Option<Instant>,
}

/// Zähler, dessen Ziffern beim Ändern einzeln kurz durchrollen – wie bei
/// einem mechanischen Zählwerk. Jede Stelle rollt nur, wenn sie sich ändert.
#[derive(Debug, Clone)]
pub struct CounterRoll {
    digits: Vec<DigitRoll>,
    last_value: u64,
    duration: Duration,
}

impl CounterRoll {
    pub fn new(duration_ms: u64) -> Self {
        Self {
            digits: Vec::new(),
            last_value: 0,
            duration: Duration::from_millis(duration_ms),
        }
    }

    /// Neuen Wert übernehmen. `animate = false` setzt ihn ohne Rollen.
    pub fn update(&mut self, value: u64, text: &str, animate: bool, now: Instant) {
        let chars: Vec<char> = text.chars().collect();
        let dir = if value >= self.last_value { RollDir::Up } else { RollDir::Down };
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
                    d.start.is_some_and(|t| now.saturating_duration_since(t) < self.duration)
                };
                match before {
                    Some(d) if d.cur == c => d,
                    // Ändert sich eine Ziffer schneller, als sie rollt (hohes Einkommen),
                    // nicht neu starten – sonst hinge die Anzeige ständig hinterher.
                    // Die laufende Rolle landet einfach auf dem neuesten Wert.
                    Some(d) if animate && rolling(&d) => DigitRoll { cur: c, ..d },
                    Some(d) if animate => DigitRoll { prev: d.cur, cur: c, dir, start: Some(now) },
                    // Neue Stelle vorne (z. B. 9999 → 10000) rollt aus dem Leeren herein.
                    None if animate => DigitRoll { prev: ' ', cur: c, dir, start: Some(now) },
                    _ => DigitRoll { prev: c, cur: c, dir, start: None },
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

/// Zeichen für den „Entschlüsseln“-Effekt.
pub const GLITCH: &[char] = &[
    '#', '%', '&', '@', '$', '*', '+', '=', '?', '/', '\\', '<', '>', '[', ']', '{', '}', '!',
    '~', '^', '0', '1', ':', ';',
];

fn glitch_char(i: usize, tick: u64) -> char {
    let mut h = (i as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ tick.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    h ^= h >> 31;
    GLITCH[(h % GLITCH.len() as u64) as usize]
}

/// Text, der aus zufälligen ASCII-Zeichen in den echten Text übergeht.
/// `progress` 0…1; Zeichen lösen sich von links nach rechts auf.
/// `tick` wechselt das Rauschen (z. B. alle 40 ms eins weiter).
pub fn decode_text(text: &str, progress: f32, tick: u64) -> String {
    if progress >= 1.0 {
        return text.to_string();
    }
    let n = text.chars().count().max(1) as f32;
    text.chars()
        .enumerate()
        .map(|(i, c)| {
            // Erstes Zeichen löst sich bei 25 %, das letzte bei 100 % auf.
            let resolve_at = 0.25 + 0.75 * (i as f32 / n);
            if c == ' ' || progress >= resolve_at {
                c
            } else {
                glitch_char(i, tick)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_follow_time() {
        let mut a = Animator::new();
        let t0 = Instant::now();
        a.play(
            AnimKind::ButtonPulse,
            vec!["a".into(), "b".into()],
            100,
            t0,
        );
        let anim = a.find(AnimKind::ButtonPulse).unwrap();
        assert_eq!(anim.frame(t0), Some("a"));
        assert_eq!(anim.frame(t0 + Duration::from_millis(150)), Some("b"));
        assert_eq!(anim.frame(t0 + Duration::from_millis(250)), None);
        a.prune(t0 + Duration::from_millis(250));
        assert!(!a.is_active());
    }

    #[test]
    fn counter_rolls_only_changed_digits() {
        let t0 = Instant::now();
        let mut c = CounterRoll::new(150);
        c.update(41, "0041", false, t0);
        c.update(42, "0042", true, t0);
        let d = c.digits();
        assert_eq!(d.len(), 4);
        assert!(d[..3].iter().all(|x| x.start.is_none()));
        assert_eq!((d[3].prev, d[3].cur, d[3].dir), ('1', '2', RollDir::Up));
        assert!(c.is_active(t0));
        assert!(!c.is_active(t0 + Duration::from_millis(200)));
    }

    #[test]
    fn fast_changes_do_not_restart_roll() {
        let t0 = Instant::now();
        let mut c = CounterRoll::new(150);
        c.update(1, "0001", false, t0);
        c.update(2, "0002", true, t0);
        c.update(3, "0003", true, t0 + Duration::from_millis(50));
        let d = c.digits()[3];
        assert_eq!((d.prev, d.cur, d.start), ('1', '3', Some(t0)));
        assert!(!c.is_active(t0 + Duration::from_millis(160)));
    }

    #[test]
    fn counter_rolls_down_and_widens() {
        let t0 = Instant::now();
        let mut c = CounterRoll::new(150);
        c.update(9999, "9999", false, t0);
        c.update(10000, "10000", true, t0);
        assert_eq!(c.digits().len(), 5);
        assert_eq!(c.digits()[4].prev, '9');
        c.update(500, "0500", true, t0 + Duration::from_millis(200));
        assert_eq!((c.digits()[1].cur, c.digits()[1].dir), ('5', RollDir::Down));
    }

    #[test]
    fn decode_resolves_left_to_right() {
        let text = "> installiert cron.d";
        let start = decode_text(text, 0.0, 3);
        assert_eq!(start.chars().count(), text.chars().count());
        assert_ne!(start, text);
        // Leerzeichen bleiben, damit die Wortform schon zu sehen ist.
        assert_eq!(start.chars().nth(1), Some(' '));
        assert_eq!(decode_text(text, 1.0, 3), text);
        let mid = decode_text(text, 0.6, 3);
        assert!(mid.starts_with("> inst"));
    }

    #[test]
    fn same_kind_replaces_but_floats_stack() {
        let mut a = Animator::new();
        let t0 = Instant::now();
        a.play(AnimKind::ButtonPulse, vec!["x".into()], 50, t0);
        a.play(AnimKind::ButtonPulse, vec!["y".into()], 50, t0);
        a.play(AnimKind::Float { id: 1, x: 0.0 }, vec!["+1".into()], 50, t0);
        a.play(AnimKind::Float { id: 2, x: 0.0 }, vec!["+1".into()], 50, t0);
        assert_eq!(a.iter().count(), 3);
    }
}
