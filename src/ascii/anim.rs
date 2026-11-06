//! Winzige Frame-Animationen aus Zeichen.
//!
//! Eine Animation ist eine Liste von Frames plus Frame-Dauer. Der `Animator`
//! hält die laufenden Animationen mit Startzeit; beim Zeichnen wird nur
//! nachgesehen, welcher Frame zur vergangenen Zeit gehört. Was eine Animation
//! *ist*, legt der Aufrufer über den Schlüssel `K` fest.

use std::time::{Duration, Instant};

pub trait AnimKey: Copy + PartialEq {
    /// Mehrere gleichzeitig erlaubt? Sonst ersetzt eine neue die laufende gleicher Art.
    fn stacks(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone)]
pub struct Anim<K> {
    pub kind: K,
    pub frames: Vec<String>,
    frame_dur: Duration,
    start: Instant,
}

impl<K> Anim<K> {
    fn total(&self) -> Duration {
        self.frame_dur * self.frames.len() as u32
    }

    fn frame_index(&self, now: Instant) -> Option<usize> {
        let elapsed = now.saturating_duration_since(self.start);
        let i = (elapsed.as_secs_f64() / self.frame_dur.as_secs_f64()) as usize;
        (i < self.frames.len()).then_some(i)
    }

    fn is_done(&self, now: Instant) -> bool {
        self.frame_index(now).is_none()
    }

    pub fn frame(&self, now: Instant) -> Option<&str> {
        self.frame_index(now).map(|i| self.frames[i].as_str())
    }

    /// An in geraden Frames, aus in ungeraden.
    pub fn blink_on(&self, now: Instant) -> bool {
        self.frame_index(now).is_some_and(|i| i % 2 == 0)
    }

    pub fn progress(&self, now: Instant) -> f32 {
        let elapsed = now.saturating_duration_since(self.start).as_secs_f32();
        (elapsed / self.total().as_secs_f32().max(0.001)).clamp(0.0, 1.0)
    }

    pub fn age(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.start)
    }
}

#[derive(Debug)]
pub struct Animator<K> {
    anims: Vec<Anim<K>>,
}

impl<K: AnimKey> Default for Animator<K> {
    fn default() -> Self {
        Self { anims: Vec::new() }
    }
}

impl<K: AnimKey> Animator<K> {
    pub fn play(&mut self, kind: K, frames: Vec<String>, frame_ms: u64, now: Instant) {
        if !kind.stacks() {
            self.anims.retain(|a| a.kind != kind);
        }
        self.anims.push(Anim {
            kind,
            frames,
            frame_dur: Duration::from_millis(frame_ms),
            start: now,
        });
    }

    /// `phases` Blink-Phasen (an/aus/an/…).
    pub fn blink(&mut self, kind: K, phases: usize, frame_ms: u64, now: Instant) {
        self.play(kind, vec![String::new(); phases], frame_ms, now);
    }

    pub fn prune(&mut self, now: Instant) {
        self.anims.retain(|a| !a.is_done(now));
    }

    pub fn clear(&mut self) {
        self.anims.clear();
    }

    pub fn is_active(&self, now: Instant) -> bool {
        self.anims.iter().any(|a| !a.is_done(now))
    }

    /// Nur laufende Animationen.
    pub fn iter(&self, now: Instant) -> impl Iterator<Item = &Anim<K>> {
        self.anims.iter().filter(move |a| !a.is_done(now))
    }

    pub fn find(&self, kind: K, now: Instant) -> Option<&Anim<K>> {
        self.iter(now).find(|a| a.kind == kind)
    }

    pub fn count(&self, now: Instant, pred: impl Fn(&K) -> bool) -> usize {
        self.iter(now).filter(|a| pred(&a.kind)).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Key {
        Single,
        Many,
    }

    impl AnimKey for Key {
        fn stacks(&self) -> bool {
            *self == Key::Many
        }
    }

    #[test]
    fn frames_follow_time() {
        let mut a = Animator::default();
        let t0 = Instant::now();
        a.play(Key::Single, vec!["a".into(), "b".into()], 100, t0);
        let anim = a.find(Key::Single, t0).unwrap();
        assert_eq!(anim.frame(t0), Some("a"));
        assert_eq!(anim.frame(t0 + Duration::from_millis(150)), Some("b"));
        assert!(!a.is_active(t0 + Duration::from_millis(250)));
        assert!(
            a.find(Key::Single, t0 + Duration::from_millis(250))
                .is_none()
        );
    }

    #[test]
    fn single_kinds_replace_and_stacking_kinds_pile_up() {
        let mut a = Animator::default();
        let t0 = Instant::now();
        a.play(Key::Single, vec!["alt".into()], 50, t0);
        a.play(Key::Single, vec!["neu".into()], 50, t0);
        a.play(Key::Many, vec!["+1".into()], 50, t0);
        a.play(Key::Many, vec!["+1".into()], 50, t0);
        assert_eq!(a.find(Key::Single, t0).unwrap().frame(t0), Some("neu"));
        assert_eq!(a.count(t0, |k| *k == Key::Single), 1);
        assert_eq!(a.count(t0, |k| *k == Key::Many), 2);
    }
}
