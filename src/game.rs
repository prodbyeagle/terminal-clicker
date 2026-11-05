//! Spiellogik ohne UI. Kennt GPUI nicht und ist komplett ohne Fenster testbar.
//! Zeit wird immer von außen als Sekunden (`f64`) hereingereicht.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// Was ein Upgrade pro Level bewirkt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Effect {
    /// Passives Einkommen: +n Bytes pro Sekunde und Level.
    PerSecond(f64),
    /// Bytes pro Klick: +n pro Level (vor den Multiplikatoren).
    ClickAdd(u64),
    /// Multiplikator pro Klick: ×n pro Level (multiplikativ).
    ClickMul(u64),
    /// Multiplikator auf alles passive Einkommen: ×n pro Level (multiplikativ).
    PassiveMul(f64),
}

#[derive(Debug, Clone, Copy)]
pub struct UpgradeDef {
    pub id: &'static str,
    pub base_cost: f64,
    /// Kostenfaktor pro Level.
    pub growth: f64,
    /// Freigeschaltet, sobald insgesamt so viele Bytes verdient wurden.
    pub unlock_at: u64,
    pub effect: Effect,
}

/// Alle Upgrades in Anzeigereihenfolge. Neue einfach hinten anhängen –
/// der Spielstand speichert Level per `id`, alte Saves bleiben gültig.
pub const UPGRADES: &[UpgradeDef] = &[
    up("auto_click.sh", 15.0, 1.15, 0, Effect::PerSecond(1.0)),
    up("macro.kbd", 40.0, 1.25, 20, Effect::ClickAdd(1)),
    up("double.bin", 100.0, 3.0, 50, Effect::ClickMul(2)),
    up("cron.d", 500.0, 1.15, 400, Effect::PerSecond(10.0)),
    up("gzip.d", 2_500.0, 3.0, 2_000, Effect::PassiveMul(1.5)),
    up("fork_daemon", 6_000.0, 1.15, 5_000, Effect::PerSecond(80.0)),
    up("hotkey.sh", 12_000.0, 1.3, 10_000, Effect::ClickAdd(25)),
    up("mmap.so", 40_000.0, 1.15, 30_000, Effect::PerSecond(450.0)),
    up("overclock.cfg", 100_000.0, 5.0, 80_000, Effect::ClickMul(3)),
    up("kernel.ko", 500_000.0, 1.15, 400_000, Effect::PerSecond(2_500.0)),
    up("botnet.py", 6_000_000.0, 1.15, 5_000_000, Effect::PerSecond(18_000.0)),
    up("cluster.yaml", 80_000_000.0, 1.15, 60_000_000, Effect::PerSecond(140_000.0)),
    up("quantum.qpu", 1_000_000_000.0, 1.15, 800_000_000, Effect::PerSecond(1_200_000.0)),
];

const fn up(id: &'static str, base_cost: f64, growth: f64, unlock_at: u64, effect: Effect) -> UpgradeDef {
    UpgradeDef {
        id,
        base_cost,
        growth,
        unlock_at,
        effect,
    }
}

/// Zählerstände, bei denen der Zähler scrambled.
pub const MILESTONES: &[u64] = &[
    1_000,
    10_000,
    100_000,
    1_000_000,
    10_000_000,
    100_000_000,
    1_000_000_000,
    10_000_000_000,
];

/// Zeitfenster für den gleitenden Klicks/s-Schnitt.
pub const RATE_WINDOW: f64 = 5.0;

#[derive(Debug, Clone, PartialEq)]
pub enum GameEvent {
    Milestone(u64),
    Unlocked(&'static str),
    Bought { id: &'static str, level: u32 },
    BuffEnded(EventKind),
}

/// Zufalls-Events, die auf dem Bildschirm aufploppen und angeklickt werden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// ×4 auf allen Ertrag (Klick und passiv) für 30 s.
    Boost,
    /// ×10 pro Klick für 15 s.
    Frenzy,
    /// Sofort ein Batzen Bytes.
    Windfall,
}

pub const BOOST_MULT: f64 = 4.0;
pub const BOOST_SECS: f64 = 30.0;
pub const FRENZY_MULT: f64 = 10.0;
pub const FRENZY_SECS: f64 = 15.0;

impl EventKind {
    pub fn title(self) -> &'static str {
        match self {
            EventKind::Boost => "BOOST ×4",
            EventKind::Frenzy => "KLICK-RAUSCH ×10",
            EventKind::Windfall => "DATENFUND",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            EventKind::Boost => "×4 auf allen ertrag, 30 s",
            EventKind::Frenzy => "×10 pro klick, 15 s",
            EventKind::Windfall => "sofort ein batzen bytes",
        }
    }
}

/// Ein laufender Bonus aus einem Event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Buff {
    pub kind: EventKind,
    pub remaining: f64,
    pub duration: f64,
}

/// Ergebnis eines eingesammelten Events.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventOutcome {
    Buff { kind: EventKind, seconds: f64 },
    Bytes(u64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BuyError {
    Locked,
    TooExpensive { cost: u64 },
    Unknown,
}

/// Serialisierbarer Spielstand.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SaveData {
    pub bytes: f64,
    pub total_earned: f64,
    pub total_clicks: u64,
    pub levels: HashMap<String, u32>,
    /// Unix-Sekunden – für späteres Offline-Einkommen.
    pub saved_at: u64,
    /// Gesamte Spielzeit in Sekunden.
    pub played_seconds: f64,
}

#[derive(Debug, Clone)]
pub struct Game {
    pub bytes: f64,
    pub total_earned: f64,
    pub total_clicks: u64,
    /// Gesamte Spielzeit in Sekunden (Echtzeit, ohne Zeitraffer).
    pub played_seconds: f64,
    levels: Vec<u32>,
    unlocked: Vec<bool>,
    click_times: VecDeque<f64>,
    /// Laufende Event-Boni (nicht gespeichert).
    buffs: Vec<Buff>,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    pub fn new() -> Self {
        let mut game = Self {
            bytes: 0.0,
            total_earned: 0.0,
            total_clicks: 0,
            played_seconds: 0.0,
            levels: vec![0; UPGRADES.len()],
            unlocked: vec![false; UPGRADES.len()],
            click_times: VecDeque::new(),
            buffs: Vec::new(),
        };
        game.refresh_unlocks();
        game
    }

    pub fn from_save(save: &SaveData) -> Self {
        let mut game = Self::new();
        game.bytes = save.bytes.max(0.0);
        game.total_earned = save.total_earned.max(game.bytes);
        game.total_clicks = save.total_clicks;
        game.played_seconds = save.played_seconds.max(0.0);
        for (i, def) in UPGRADES.iter().enumerate() {
            game.levels[i] = save.levels.get(def.id).copied().unwrap_or(0);
        }
        game.refresh_unlocks();
        game
    }

    pub fn to_save(&self, now_unix: u64) -> SaveData {
        SaveData {
            bytes: self.bytes,
            total_earned: self.total_earned,
            total_clicks: self.total_clicks,
            levels: UPGRADES
                .iter()
                .zip(&self.levels)
                .filter(|(_, l)| **l > 0)
                .map(|(d, l)| (d.id.to_string(), *l))
                .collect(),
            saved_at: now_unix,
            played_seconds: self.played_seconds,
        }
    }

    /// Ganze Bytes, wie sie angezeigt werden.
    pub fn whole_bytes(&self) -> u64 {
        self.bytes.floor() as u64
    }

    pub fn level(&self, index: usize) -> u32 {
        self.levels.get(index).copied().unwrap_or(0)
    }

    pub fn is_unlocked(&self, index: usize) -> bool {
        self.unlocked.get(index).copied().unwrap_or(false)
    }

    pub fn per_click(&self) -> u64 {
        let mut base: u64 = 1;
        let mut mul: u64 = 1;
        for (def, &lvl) in UPGRADES.iter().zip(&self.levels) {
            match def.effect {
                Effect::ClickAdd(n) => base = base.saturating_add(n.saturating_mul(lvl as u64)),
                Effect::ClickMul(m) => mul = mul.saturating_mul(m.saturating_pow(lvl)),
                _ => {}
            }
        }
        let value = base.saturating_mul(mul);
        let buff = self.click_buff();
        if buff > 1.0 {
            (value as f64 * buff).min(u64::MAX as f64) as u64
        } else {
            value
        }
    }

    /// Gemeinsamer Multiplikator auf alles passive Einkommen.
    pub fn passive_multiplier(&self) -> f64 {
        UPGRADES
            .iter()
            .zip(&self.levels)
            .map(|(def, &lvl)| match def.effect {
                Effect::PassiveMul(m) => m.powi(lvl as i32),
                _ => 1.0,
            })
            .product()
    }

    /// Passives Einkommen eines einzelnen Upgrades in B/s (inkl. Multiplikator).
    pub fn source_rate(&self, index: usize) -> f64 {
        match UPGRADES.get(index).map(|d| d.effect) {
            Some(Effect::PerSecond(n)) => {
                n * self.level(index) as f64 * self.passive_multiplier() * self.passive_buff()
            }
            _ => 0.0,
        }
    }

    pub fn per_second(&self) -> f64 {
        (0..UPGRADES.len()).map(|i| self.source_rate(i)).sum()
    }

    pub fn cost(&self, index: usize) -> u64 {
        let def = &UPGRADES[index];
        (def.base_cost * def.growth.powi(self.level(index) as i32)).ceil() as u64
    }

    /// Ein Klick zur Zeit `now` (Sekunden). Gibt die verdienten Bytes zurück.
    pub fn click(&mut self, now: f64, events: &mut Vec<GameEvent>) -> u64 {
        let gain = self.per_click();
        self.total_clicks += 1;
        self.click_times.push_back(now);
        self.prune_clicks(now);
        self.earn(gain as f64, events);
        gain
    }

    /// Passives Einkommen über `dt` Sekunden.
    pub fn tick(&mut self, dt: f64, now: f64, events: &mut Vec<GameEvent>) {
        self.prune_clicks(now);
        let gain = self.per_second() * dt.max(0.0);
        if gain > 0.0 {
            self.earn(gain, events);
        }
        for buff in &mut self.buffs {
            buff.remaining -= dt.max(0.0);
        }
        self.buffs.retain(|b| {
            let alive = b.remaining > 0.0;
            if !alive {
                events.push(GameEvent::BuffEnded(b.kind));
            }
            alive
        });
    }

    // ── Events ────────────────────────────────────────────────────────────

    /// Ein angeklicktes Event auslösen. Gleiches Event nochmal = Zeit wieder voll.
    pub fn trigger_event(&mut self, kind: EventKind, events: &mut Vec<GameEvent>) -> EventOutcome {
        let duration = match kind {
            EventKind::Boost => BOOST_SECS,
            EventKind::Frenzy => FRENZY_SECS,
            EventKind::Windfall => {
                // Wie ein Glücksfund: 15 % vom Kontostand, aber höchstens
                // 15 Minuten passives Einkommen (plus ein kleiner Sockel).
                let amount = (self.bytes * 0.15).min(self.per_second() * 900.0).floor() + 50.0;
                self.earn(amount, events);
                return EventOutcome::Bytes(amount as u64);
            }
        };
        match self.buffs.iter_mut().find(|b| b.kind == kind) {
            Some(b) => b.remaining = duration,
            None => self.buffs.push(Buff { kind, remaining: duration, duration }),
        }
        EventOutcome::Buff { kind, seconds: duration }
    }

    pub fn buffs(&self) -> &[Buff] {
        &self.buffs
    }

    /// Multiplikator der laufenden Boni auf passives Einkommen.
    pub fn passive_buff(&self) -> f64 {
        self.buffs
            .iter()
            .map(|b| if b.kind == EventKind::Boost { BOOST_MULT } else { 1.0 })
            .product()
    }

    /// Multiplikator der laufenden Boni pro Klick.
    pub fn click_buff(&self) -> f64 {
        self.buffs
            .iter()
            .map(|b| match b.kind {
                EventKind::Boost => BOOST_MULT,
                EventKind::Frenzy => FRENZY_MULT,
                EventKind::Windfall => 1.0,
            })
            .product()
    }

    /// Klicks pro Sekunde, gleitend über die letzten `RATE_WINDOW` Sekunden.
    pub fn clicks_per_second(&self, now: f64) -> f64 {
        let n = self
            .click_times
            .iter()
            .filter(|&&t| now - t <= RATE_WINDOW)
            .count();
        n as f64 / RATE_WINDOW
    }

    pub fn buy(&mut self, index: usize, events: &mut Vec<GameEvent>) -> Result<u32, BuyError> {
        let Some(def) = UPGRADES.get(index) else {
            return Err(BuyError::Unknown);
        };
        if !self.is_unlocked(index) {
            return Err(BuyError::Locked);
        }
        let cost = self.cost(index);
        if self.bytes < cost as f64 {
            return Err(BuyError::TooExpensive { cost });
        }
        self.bytes -= cost as f64;
        self.levels[index] += 1;
        let level = self.levels[index];
        events.push(GameEvent::Bought { id: def.id, level });
        Ok(level)
    }

    // ── Admin ─────────────────────────────────────────────────────────────
    // Werkzeuge zum Testen/Schummeln. Laufen über dieselben Wege wie das
    // normale Spiel, damit Meilensteine und Freischaltungen mitkommen.

    /// Bytes gutschreiben (zählt wie verdient: schaltet frei, löst Meilensteine aus).
    pub fn admin_grant(&mut self, amount: f64, events: &mut Vec<GameEvent>) {
        if amount > 0.0 {
            self.earn(amount, events);
        }
    }

    /// Alle Upgrades freischalten, ohne Bytes zu verdienen.
    pub fn admin_unlock_all(&mut self, events: &mut Vec<GameEvent>) {
        for (def, unlocked) in UPGRADES.iter().zip(self.unlocked.iter_mut()) {
            if !*unlocked {
                *unlocked = true;
                events.push(GameEvent::Unlocked(def.id));
            }
        }
    }

    /// Jedes Upgrade gratis um ein Level erhöhen (schaltet es dabei frei).
    pub fn admin_level_all(&mut self) {
        for (level, unlocked) in self.levels.iter_mut().zip(self.unlocked.iter_mut()) {
            *level += 1;
            *unlocked = true;
        }
    }

    /// Alles auf Anfang. Die Spielzeit bleibt erhalten.
    pub fn admin_reset(&mut self) {
        let played = self.played_seconds;
        *self = Self::new();
        self.played_seconds = played;
    }

    pub fn upgrades_bought(&self) -> u32 {
        self.levels.iter().sum()
    }

    pub fn unlocked_count(&self) -> usize {
        self.unlocked.iter().filter(|u| **u).count()
    }

    fn earn(&mut self, amount: f64, events: &mut Vec<GameEvent>) {
        let before = self.whole_bytes();
        self.bytes += amount;
        self.total_earned += amount;
        let after = self.whole_bytes();
        for &m in MILESTONES {
            if before < m && after >= m {
                events.push(GameEvent::Milestone(m));
            }
        }
        for (def, unlocked) in UPGRADES.iter().zip(self.unlocked.iter_mut()) {
            if !*unlocked && self.total_earned >= def.unlock_at as f64 {
                *unlocked = true;
                events.push(GameEvent::Unlocked(def.id));
            }
        }
    }

    fn refresh_unlocks(&mut self) {
        for (i, def) in UPGRADES.iter().enumerate() {
            if self.total_earned >= def.unlock_at as f64 || self.levels[i] > 0 {
                self.unlocked[i] = true;
            }
        }
    }

    fn prune_clicks(&mut self, now: f64) {
        while let Some(&t) = self.click_times.front() {
            if now - t > RATE_WINDOW {
                self.click_times.pop_front();
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idx(id: &str) -> usize {
        UPGRADES.iter().position(|d| d.id == id).unwrap()
    }

    #[test]
    fn click_counts_up() {
        let mut g = Game::new();
        let mut ev = vec![];
        assert_eq!(g.click(0.0, &mut ev), 1);
        g.click(0.1, &mut ev);
        assert_eq!(g.whole_bytes(), 2);
        assert_eq!(g.total_clicks, 2);
    }

    #[test]
    fn auto_click_gives_passive_income() {
        let mut g = Game::new();
        let mut ev = vec![];
        g.bytes = 15.0;
        g.buy(idx("auto_click.sh"), &mut ev).unwrap();
        assert_eq!(g.bytes, 0.0);
        for i in 0..10 {
            g.tick(0.1, i as f64 * 0.1, &mut ev);
        }
        assert!((g.bytes - 1.0).abs() < 1e-9);
    }

    #[test]
    fn cost_grows_per_level() {
        let mut g = Game::new();
        let mut ev = vec![];
        let i = idx("auto_click.sh");
        assert_eq!(g.cost(i), 15);
        g.bytes = 1000.0;
        g.buy(i, &mut ev).unwrap();
        assert_eq!(g.cost(i), (15.0f64 * 1.15).ceil() as u64);
    }

    #[test]
    fn double_multiplies_click() {
        let mut g = Game::new();
        let mut ev = vec![];
        g.total_earned = 100.0;
        g.refresh_unlocks();
        g.bytes = 1000.0;
        let d = idx("double.bin");
        g.buy(d, &mut ev).unwrap();
        assert_eq!(g.per_click(), 2);
        g.buy(d, &mut ev).unwrap();
        assert_eq!(g.per_click(), 4);
    }

    #[test]
    fn locked_until_threshold() {
        let mut g = Game::new();
        let mut ev = vec![];
        let d = idx("double.bin");
        g.bytes = 10_000.0;
        assert!(!g.is_unlocked(d));
        assert_eq!(g.buy(d, &mut ev), Err(BuyError::Locked));
        for i in 0..50 {
            g.click(i as f64, &mut ev);
        }
        assert!(g.is_unlocked(d));
        assert!(ev.contains(&GameEvent::Unlocked("double.bin")));
    }

    #[test]
    fn click_add_then_multiply() {
        let mut g = Game::new();
        let mut ev = vec![];
        g.total_earned = 1_000.0;
        g.refresh_unlocks();
        g.bytes = 10_000.0;
        g.buy(idx("macro.kbd"), &mut ev).unwrap();
        g.buy(idx("macro.kbd"), &mut ev).unwrap();
        assert_eq!(g.per_click(), 3);
        g.buy(idx("double.bin"), &mut ev).unwrap();
        assert_eq!(g.per_click(), 6);
    }

    #[test]
    fn passive_multiplier_scales_all_sources() {
        let mut g = Game::new();
        let mut ev = vec![];
        g.total_earned = 5_000.0;
        g.refresh_unlocks();
        g.bytes = 100_000.0;
        g.buy(idx("auto_click.sh"), &mut ev).unwrap();
        g.buy(idx("cron.d"), &mut ev).unwrap();
        assert_eq!(g.per_second(), 11.0);
        g.buy(idx("gzip.d"), &mut ev).unwrap();
        assert!((g.per_second() - 16.5).abs() < 1e-9);
        assert!((g.source_rate(idx("cron.d")) - 15.0).abs() < 1e-9);
    }

    #[test]
    fn upgrade_ids_unique_and_sorted_by_unlock() {
        for (i, a) in UPGRADES.iter().enumerate() {
            for b in &UPGRADES[i + 1..] {
                assert_ne!(a.id, b.id);
                assert!(a.unlock_at <= b.unlock_at, "{} vor {}", a.id, b.id);
            }
        }
    }

    #[test]
    fn admin_grant_counts_as_earned() {
        let mut g = Game::new();
        let mut ev = vec![];
        g.admin_grant(1_000_000.0, &mut ev);
        assert_eq!(g.whole_bytes(), 1_000_000);
        assert!(g.is_unlocked(idx("kernel.ko")));
        assert!(ev.contains(&GameEvent::Milestone(1_000_000)));
    }

    #[test]
    fn admin_level_all_and_reset() {
        let mut g = Game::new();
        g.played_seconds = 42.0;
        g.admin_level_all();
        assert_eq!(g.upgrades_bought(), UPGRADES.len() as u32);
        assert_eq!(g.unlocked_count(), UPGRADES.len());
        g.admin_reset();
        assert_eq!(g.upgrades_bought(), 0);
        assert_eq!(g.whole_bytes(), 0);
        assert_eq!(g.played_seconds, 42.0);
    }

    #[test]
    fn boost_multiplies_click_and_passive_then_ends() {
        let mut g = Game::new();
        let mut ev = vec![];
        g.bytes = 15.0;
        g.buy(idx("auto_click.sh"), &mut ev).unwrap();
        g.trigger_event(EventKind::Boost, &mut ev);
        assert_eq!(g.per_click(), 4);
        assert_eq!(g.per_second(), 4.0);
        g.tick(BOOST_SECS + 0.1, 0.0, &mut ev);
        assert!(ev.contains(&GameEvent::BuffEnded(EventKind::Boost)));
        assert_eq!(g.per_click(), 1);
        assert_eq!(g.per_second(), 1.0);
    }

    #[test]
    fn frenzy_only_clicks_and_retrigger_refreshes() {
        let mut g = Game::new();
        let mut ev = vec![];
        g.trigger_event(EventKind::Frenzy, &mut ev);
        assert_eq!(g.per_click(), 10);
        g.tick(10.0, 0.0, &mut ev);
        g.trigger_event(EventKind::Frenzy, &mut ev);
        assert_eq!(g.buffs().len(), 1);
        assert_eq!(g.buffs()[0].remaining, FRENZY_SECS);
    }

    #[test]
    fn windfall_is_capped_by_income() {
        let mut g = Game::new();
        let mut ev = vec![];
        g.bytes = 1_000_000.0;
        // Kein passives Einkommen: nur der Sockel.
        assert_eq!(g.trigger_event(EventKind::Windfall, &mut ev), EventOutcome::Bytes(50));
    }

    #[test]
    fn too_expensive() {
        let mut g = Game::new();
        let mut ev = vec![];
        assert_eq!(
            g.buy(0, &mut ev),
            Err(BuyError::TooExpensive { cost: 15 })
        );
    }

    #[test]
    fn milestone_fires_once() {
        let mut g = Game::new();
        let mut ev = vec![];
        g.bytes = 999.0;
        g.total_earned = 999.0;
        g.click(0.0, &mut ev);
        g.click(0.1, &mut ev);
        let n = ev
            .iter()
            .filter(|e| **e == GameEvent::Milestone(1000))
            .count();
        assert_eq!(n, 1);
    }

    #[test]
    fn click_rate_is_moving_average() {
        let mut g = Game::new();
        let mut ev = vec![];
        for i in 0..10 {
            g.click(i as f64 * 0.5, &mut ev);
        }
        // 10 Klicks in 4.5 s, Fenster 5 s
        assert!((g.clicks_per_second(4.5) - 2.0).abs() < 1e-9);
        // 20 s später ist alles rausgefallen
        assert_eq!(g.clicks_per_second(30.0), 0.0);
    }

    #[test]
    fn save_roundtrip() {
        let mut g = Game::new();
        let mut ev = vec![];
        g.bytes = 500.0;
        g.total_earned = 500.0;
        g.buy(0, &mut ev).unwrap();
        let save = g.to_save(123);
        let json = serde_json::to_string(&save).unwrap();
        let back: SaveData = serde_json::from_str(&json).unwrap();
        let g2 = Game::from_save(&back);
        assert_eq!(g2.level(0), 1);
        assert_eq!(g2.bytes, g.bytes);
        assert!(g2.is_unlocked(idx("cron.d")));
    }

    #[test]
    fn old_save_without_fields_loads() {
        let back: SaveData = serde_json::from_str(r#"{"bytes": 7}"#).unwrap();
        let g = Game::from_save(&back);
        assert_eq!(g.whole_bytes(), 7);
    }
}
