//! Spiellogik ohne UI. Kennt GPUI nicht und ist komplett ohne Fenster testbar.
//! Zeit wird immer von außen als Sekunden (`f64`) hereingereicht.
//!
//! Alle Mengen sind `f64`: damit gibt es praktisch kein Limit (bis ~10^308).

pub mod events;
pub mod upgrades;

use crate::rng::Rng;
use events::{BuffKind, Buffs};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use upgrades::Effect;
pub use upgrades::UPGRADES;

/// Ab hier löst jede neue Zehnerpotenz einen Meilenstein aus (1e3, 1e4, …).
const FIRST_MILESTONE_EXP: i32 = 3;
/// Höchstens so viele Meilensteine pro Schritt, damit ein riesiger Sprung das Log nicht flutet.
const MAX_MILESTONES_AT_ONCE: usize = 3;

pub const CRIT_MULT: f64 = 10.0;
const CRIT_CAP: f64 = 0.5;
pub const OFFLINE_CAP_SECS: f64 = 8.0 * 3600.0;
/// Rabatt-Upgrades sparen höchstens so viel (Faktor nie unter diesem Wert).
const DISCOUNT_FLOOR: f64 = 0.25;
const RATE_WINDOW: f64 = 5.0;

#[derive(Debug, Clone, PartialEq)]
pub enum GameEvent {
    /// Neue Zehnerpotenz erreicht (Wert ist die Potenz, z. B. 1e6).
    Milestone(f64),
    Unlocked(&'static str),
    /// `index` in `UPGRADES`.
    Bought {
        index: usize,
        level: u32,
    },
    BuffEnded(BuffKind),
    Reset,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClickResult {
    pub gain: f64,
    pub crit: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BuyError {
    Locked,
    TooExpensive { cost: f64 },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SaveData {
    pub bytes: f64,
    pub total_earned: f64,
    pub total_clicks: u64,
    pub levels: HashMap<String, u32>,
    /// Unix-Sekunden des letzten Speicherns – für Offline-Einkommen.
    pub saved_at: u64,
    pub played_seconds: f64,
}

#[derive(Debug, Clone)]
pub struct Game {
    pub bytes: f64,
    pub total_earned: f64,
    pub total_clicks: u64,
    /// Echtzeit, ohne Zeitraffer.
    pub played_seconds: f64,
    levels: Vec<u32>,
    unlocked: Vec<bool>,
    click_times: VecDeque<f64>,
    /// Laufende Event-Wirkungen; werden nicht gespeichert.
    buffs: Buffs,
    rng: Rng,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    /// Neues Spiel mit festem Zufall (für Tests); im Spiel `seeded`.
    pub fn new() -> Self {
        Self::seeded(0)
    }

    /// `seed` bestimmt kritische Klicks und Glitches.
    pub fn seeded(seed: u64) -> Self {
        let mut game = Self {
            bytes: 0.0,
            total_earned: 0.0,
            total_clicks: 0,
            played_seconds: 0.0,
            levels: vec![0; UPGRADES.len()],
            unlocked: vec![false; UPGRADES.len()],
            click_times: VecDeque::new(),
            buffs: Buffs::default(),
            rng: Rng::new(seed),
        };
        game.unlock_new();
        game
    }

    fn from_save(save: &SaveData, seed: u64) -> Self {
        let mut game = Self::seeded(seed);
        game.bytes = save.bytes.max(0.0);
        game.total_earned = save.total_earned.max(game.bytes);
        game.total_clicks = save.total_clicks;
        game.played_seconds = save.played_seconds.max(0.0);
        for (i, def) in UPGRADES.iter().enumerate() {
            game.levels[i] = save.levels.get(def.id).copied().unwrap_or(0);
        }
        game.unlock_new();
        game
    }

    /// Spielstand fortsetzen und das Offline-Einkommen seit `save.saved_at`
    /// gutschreiben. Gibt auch zurück, was dabei passiert ist (z. B. Freischaltungen).
    pub fn resume(save: &SaveData, now_unix: u64, seed: u64) -> (Self, f64, Vec<GameEvent>) {
        let mut game = Self::from_save(save, seed);
        let away = if save.saved_at > 0 {
            now_unix.saturating_sub(save.saved_at) as f64
        } else {
            0.0
        };
        let mut events = Vec::new();
        let gained = game.apply_offline(away, &mut events);
        (game, gained, events)
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

    pub fn whole_bytes(&self) -> f64 {
        self.bytes.floor()
    }

    pub fn level(&self, index: usize) -> u32 {
        self.levels[index]
    }

    pub fn is_unlocked(&self, index: usize) -> bool {
        self.unlocked[index]
    }

    fn effect_sum(&self, pick: impl Fn(Effect) -> Option<f64>) -> f64 {
        UPGRADES
            .iter()
            .zip(&self.levels)
            .filter_map(|(def, &lvl)| pick(def.effect).map(|n| n * lvl as f64))
            .sum()
    }

    fn effect_product(&self, pick: impl Fn(Effect) -> Option<f64>) -> f64 {
        UPGRADES
            .iter()
            .zip(&self.levels)
            .filter_map(|(def, &lvl)| pick(def.effect).map(|m| m.powi(lvl as i32)))
            .product()
    }

    /// Bytes pro Klick, ohne kritischen Treffer.
    pub fn per_click(&self) -> f64 {
        let base = 1.0
            + self.effect_sum(|e| match e {
                Effect::ClickAdd(n) => Some(n),
                _ => None,
            });
        let mul = self.effect_product(|e| match e {
            Effect::ClickMul(m) => Some(m),
            _ => None,
        });
        let passive_secs = self.effect_sum(|e| match e {
            Effect::ClickFromPassive(s) => Some(s),
            _ => None,
        }) + self.buffs.click_passive_secs();
        base * mul * self.buffs.click_mult() + passive_secs * self.per_second()
    }

    pub fn crit_chance(&self) -> f64 {
        self.effect_sum(|e| match e {
            Effect::Crit(c) => Some(c),
            _ => None,
        })
        .min(CRIT_CAP)
    }

    pub fn passive_multiplier(&self) -> f64 {
        self.effect_product(|e| match e {
            Effect::PassiveMul(m) => Some(m),
            _ => None,
        })
    }

    /// Passives Einkommen eines einzelnen Upgrades in B/s, inklusive Multiplikatoren.
    pub fn source_rate(&self, index: usize) -> f64 {
        match UPGRADES.get(index).map(|d| d.effect) {
            Some(Effect::PerSecond(n)) => {
                n * self.level(index) as f64 * self.passive_multiplier() * self.buffs.passive_mult()
            }
            _ => 0.0,
        }
    }

    pub fn per_second(&self) -> f64 {
        (0..UPGRADES.len()).map(|i| self.source_rate(i)).sum()
    }

    pub fn offline_fraction(&self) -> f64 {
        self.effect_sum(|e| match e {
            Effect::Offline(f) => Some(f),
            _ => None,
        })
        .min(1.0)
    }

    pub fn cost_multiplier(&self) -> f64 {
        let upgrades = self
            .effect_product(|e| match e {
                Effect::Discount(m) => Some(m),
                _ => None,
            })
            .max(DISCOUNT_FLOOR);
        upgrades * self.buffs.cost_mult()
    }

    pub fn cost(&self, index: usize) -> f64 {
        let def = &UPGRADES[index];
        (def.base_cost * def.growth.powi(self.level(index) as i32) * self.cost_multiplier()).ceil()
    }

    pub fn can_afford(&self, index: usize) -> bool {
        self.bytes >= self.cost(index)
    }

    pub fn upgrades_bought(&self) -> u32 {
        self.levels.iter().sum()
    }

    pub fn unlocked_count(&self) -> usize {
        self.unlocked.iter().filter(|u| **u).count()
    }

    pub fn clicks_per_second(&self, now: f64) -> f64 {
        let n = self
            .click_times
            .iter()
            .filter(|&&t| now - t <= RATE_WINDOW)
            .count();
        n as f64 / RATE_WINDOW
    }

    pub fn click(&mut self, now: f64, events: &mut Vec<GameEvent>) -> ClickResult {
        let crit = self.rng.chance(self.crit_chance());
        let gain = self.per_click() * if crit { CRIT_MULT } else { 1.0 };
        self.total_clicks += 1;
        self.click_times.push_back(now);
        self.prune_clicks(now);
        self.earn(gain, events);
        ClickResult { gain, crit }
    }

    /// `real_dt` zählt als Spielzeit; `time_scale` (Admin-Zeitraffer) beschleunigt
    /// nur Einkommen und Event-Wirkungen.
    /// Gibt zurück, wie viel jede Einkommensquelle (Index in `UPGRADES`) verdient hat.
    pub fn tick(
        &mut self,
        real_dt: f64,
        time_scale: f64,
        now: f64,
        events: &mut Vec<GameEvent>,
    ) -> Vec<(usize, f64)> {
        let real_dt = real_dt.max(0.0);
        let dt = real_dt * time_scale;
        self.played_seconds += real_dt;
        self.prune_clicks(now);
        let income: Vec<(usize, f64)> = (0..UPGRADES.len())
            .map(|i| (i, self.source_rate(i) * dt))
            .filter(|(_, earned)| *earned > 0.0)
            .collect();
        self.earn(income.iter().map(|(_, e)| e).sum(), events);
        self.buffs.tick(dt, events);
        income
    }

    fn apply_offline(&mut self, seconds: f64, events: &mut Vec<GameEvent>) -> f64 {
        let amount =
            self.per_second() * seconds.clamp(0.0, OFFLINE_CAP_SECS) * self.offline_fraction();
        self.earn(amount, events);
        amount
    }

    pub fn buy(&mut self, index: usize, events: &mut Vec<GameEvent>) -> Result<u32, BuyError> {
        if !self.is_unlocked(index) {
            return Err(BuyError::Locked);
        }
        let cost = self.cost(index);
        if !self.can_afford(index) {
            return Err(BuyError::TooExpensive { cost });
        }
        self.bytes -= cost;
        self.levels[index] += 1;
        let level = self.levels[index];
        events.push(GameEvent::Bought { index, level });
        Ok(level)
    }

    // ── Admin ─────────────────────────────────────────────────────────────
    // Laufen über dieselben Wege wie das normale Spiel, damit Meilensteine
    // und Freischaltungen mitkommen.

    /// Zählt wie verdient: schaltet frei und löst Meilensteine aus.
    pub fn admin_grant(&mut self, amount: f64, events: &mut Vec<GameEvent>) {
        self.earn(amount, events);
    }

    pub fn admin_unlock_all(&mut self, events: &mut Vec<GameEvent>) {
        for (def, unlocked) in UPGRADES.iter().zip(self.unlocked.iter_mut()) {
            if !*unlocked {
                *unlocked = true;
                events.push(GameEvent::Unlocked(def.id));
            }
        }
    }

    pub fn admin_level_all(&mut self, events: &mut Vec<GameEvent>) {
        for level in &mut self.levels {
            *level += 1;
        }
        events.extend(self.unlock_new().into_iter().map(GameEvent::Unlocked));
    }

    /// Alles auf Anfang; die Spielzeit bleibt.
    pub fn admin_reset(&mut self, events: &mut Vec<GameEvent>) {
        let played = self.played_seconds;
        let rng = self.rng.clone();
        *self = Self::new();
        self.played_seconds = played;
        self.rng = rng;
        events.push(GameEvent::Reset);
    }

    fn earn(&mut self, amount: f64, events: &mut Vec<GameEvent>) {
        if amount <= 0.0 {
            return;
        }
        let before = self.whole_bytes();
        self.bytes += amount;
        self.total_earned += amount;
        let exp = |v: f64| {
            if v >= 1.0 {
                v.log10().floor() as i32
            } else {
                -1
            }
        };
        let from = exp(before).max(FIRST_MILESTONE_EXP - 1);
        let reached: Vec<i32> = (from + 1..=exp(self.whole_bytes())).collect();
        let shown = &reached[reached.len().saturating_sub(MAX_MILESTONES_AT_ONCE)..];
        events.extend(shown.iter().map(|&e| GameEvent::Milestone(10f64.powi(e))));
        events.extend(self.unlock_new().into_iter().map(GameEvent::Unlocked));
    }

    /// Schaltet frei, was genug verdient hat (oder schon gekauft ist); gibt die neuen IDs zurück.
    fn unlock_new(&mut self) -> Vec<&'static str> {
        let mut new = Vec::new();
        for (i, def) in UPGRADES.iter().enumerate() {
            if !self.unlocked[i] && (self.total_earned >= def.unlock_at || self.levels[i] > 0) {
                self.unlocked[i] = true;
                new.push(def.id);
            }
        }
        new
    }

    fn prune_clicks(&mut self, now: f64) {
        while self
            .click_times
            .front()
            .is_some_and(|&t| now - t > RATE_WINDOW)
        {
            self.click_times.pop_front();
        }
    }
}

#[cfg(test)]
pub(crate) mod testkit {
    use super::*;

    pub fn idx(id: &str) -> usize {
        UPGRADES.iter().position(|d| d.id == id).unwrap()
    }

    /// Alles freigeschaltet und genug Bytes für beliebige Käufe.
    pub fn unlocked_rich() -> Game {
        let mut g = Game::new();
        g.admin_unlock_all(&mut vec![]);
        g.bytes = 1e30;
        g
    }
}

#[cfg(test)]
mod tests {
    use super::testkit::{idx, unlocked_rich};
    use super::*;

    /// Der Zahlenwert des Effekts eines Upgrades aus dem Katalog.
    fn effect_value(id: &str) -> f64 {
        match UPGRADES[idx(id)].effect {
            Effect::PerSecond(n)
            | Effect::ClickAdd(n)
            | Effect::ClickMul(n)
            | Effect::PassiveMul(n)
            | Effect::Crit(n)
            | Effect::ClickFromPassive(n)
            | Effect::Offline(n)
            | Effect::Discount(n) => n,
            Effect::EventBoost => 1.0,
        }
    }

    #[test]
    fn click_counts_up() {
        let mut g = Game::new();
        let mut ev = vec![];
        assert_eq!(g.click(0.0, &mut ev).gain, 1.0);
        g.click(0.1, &mut ev);
        assert_eq!(g.whole_bytes(), 2.0);
        assert_eq!(g.total_clicks, 2);
    }

    #[test]
    fn no_limit_beyond_u64() {
        let mut g = Game::new();
        let mut ev = vec![];
        g.admin_grant(1e25, &mut ev);
        g.admin_grant(1e25, &mut ev);
        assert_eq!(g.whole_bytes(), 2e25);
        assert!(ev.contains(&GameEvent::Milestone(1e25)));
    }

    #[test]
    fn auto_click_gives_passive_income() {
        let mut g = Game::new();
        let mut ev = vec![];
        g.bytes = 15.0;
        g.buy(idx("auto_click.sh"), &mut ev).unwrap();
        assert_eq!(g.bytes, 0.0);
        for i in 0..10 {
            g.tick(0.1, 1.0, i as f64 * 0.1, &mut ev);
        }
        assert!((g.bytes - effect_value("auto_click.sh")).abs() < 1e-9);
    }

    #[test]
    fn buying_costs_bytes_and_makes_the_next_level_dearer() {
        let mut g = Game::new();
        let mut ev = vec![];
        let i = idx("auto_click.sh");
        let first = g.cost(i);
        g.bytes = 1000.0;
        g.buy(i, &mut ev).unwrap();
        assert_eq!(g.bytes, 1000.0 - first);
        assert!(g.cost(i) > first);
    }

    #[test]
    fn double_multiplies_click() {
        let mut g = unlocked_rich();
        let mut ev = vec![];
        let d = idx("double.bin");
        let base = g.per_click();
        g.buy(d, &mut ev).unwrap();
        assert_eq!(g.per_click(), base * effect_value("double.bin"));
        g.buy(d, &mut ev).unwrap();
        assert_eq!(g.per_click(), base * effect_value("double.bin").powi(2));
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
        let mut g = unlocked_rich();
        let mut ev = vec![];
        g.buy(idx("macro.kbd"), &mut ev).unwrap();
        g.buy(idx("macro.kbd"), &mut ev).unwrap();
        let added = 1.0 + 2.0 * effect_value("macro.kbd");
        assert_eq!(g.per_click(), added);
        g.buy(idx("double.bin"), &mut ev).unwrap();
        assert_eq!(g.per_click(), added * effect_value("double.bin"));
    }

    #[test]
    fn passive_multiplier_scales_all_sources() {
        let mut g = unlocked_rich();
        let mut ev = vec![];
        g.buy(idx("auto_click.sh"), &mut ev).unwrap();
        g.buy(idx("cron.d"), &mut ev).unwrap();
        let (total, cron) = (g.per_second(), g.source_rate(idx("cron.d")));
        g.buy(idx("gzip.d"), &mut ev).unwrap();
        let mult = effect_value("gzip.d");
        assert!((g.per_second() - total * mult).abs() < 1e-9);
        assert!((g.source_rate(idx("cron.d")) - cron * mult).abs() < 1e-9);
    }

    #[test]
    fn crit_chance_is_capped_and_crits_happen() {
        let mut g = unlocked_rich();
        let mut ev = vec![];
        for _ in 0..40 {
            g.buy(idx("lucky.bit"), &mut ev).unwrap();
        }
        assert_eq!(g.crit_chance(), CRIT_CAP);
        let crits = (0..400)
            .filter(|i| g.click(*i as f64, &mut ev).crit)
            .count();
        assert!((120..280).contains(&crits), "{crits} krits");
    }

    #[test]
    fn click_from_passive_adds_seconds_of_income() {
        let mut g = unlocked_rich();
        let mut ev = vec![];
        g.buy(idx("cron.d"), &mut ev).unwrap();
        let click = g.per_click();
        g.buy(idx("pipe.sh"), &mut ev).unwrap();
        assert_eq!(
            g.per_click(),
            click + effect_value("pipe.sh") * g.per_second()
        );
    }

    #[test]
    fn discount_makes_everything_cheaper_but_has_floor() {
        let mut g = unlocked_rich();
        let mut ev = vec![];
        let base = g.cost(idx("kernel.ko"));
        g.buy(idx("compiler.o3"), &mut ev).unwrap();
        assert!(g.cost(idx("kernel.ko")) < base);
        for _ in 0..30 {
            g.buy(idx("compiler.o3"), &mut ev).unwrap();
        }
        assert!(g.cost(idx("kernel.ko")) >= (base * DISCOUNT_FLOOR).floor());
    }

    #[test]
    fn offline_income_needs_upgrade_and_is_capped() {
        let mut g = unlocked_rich();
        let mut ev = vec![];
        g.buy(idx("auto_click.sh"), &mut ev).unwrap();
        assert_eq!(g.apply_offline(3600.0, &mut ev), 0.0);
        g.buy(idx("systemd.service"), &mut ev).unwrap();
        let share = effect_value("systemd.service");
        assert!((g.apply_offline(100.0, &mut ev) - g.per_second() * 100.0 * share).abs() < 1e-9);
        assert_eq!(
            g.apply_offline(1e9, &mut ev),
            g.apply_offline(OFFLINE_CAP_SECS, &mut ev)
        );
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
        assert_eq!(g.whole_bytes(), 1_000_000.0);
        assert!(g.is_unlocked(idx("kernel.ko")));
        assert!(ev.contains(&GameEvent::Milestone(1e6)));
    }

    #[test]
    fn admin_level_all_and_reset() {
        let mut g = Game::new();
        let mut ev = vec![];
        g.played_seconds = 42.0;
        g.admin_level_all(&mut ev);
        assert_eq!(g.upgrades_bought(), UPGRADES.len() as u32);
        assert_eq!(g.unlocked_count(), UPGRADES.len());
        assert!(ev.contains(&GameEvent::Unlocked(UPGRADES[UPGRADES.len() - 1].id)));
        g.admin_reset(&mut ev);
        assert_eq!(g.upgrades_bought(), 0);
        assert_eq!(g.whole_bytes(), 0.0);
        assert_eq!(g.played_seconds, 42.0);
    }

    #[test]
    fn too_expensive() {
        let mut g = Game::new();
        let mut ev = vec![];
        assert_eq!(
            g.buy(0, &mut ev),
            Err(BuyError::TooExpensive { cost: g.cost(0) })
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
            .filter(|e| **e == GameEvent::Milestone(1000.0))
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
    fn resume_pays_offline_time_since_the_save() {
        let mut g = unlocked_rich();
        let mut ev = vec![];
        g.buy(idx("auto_click.sh"), &mut ev).unwrap();
        g.buy(idx("systemd.service"), &mut ev).unwrap();
        g.bytes = 0.0;
        let save = g.to_save(1_000);
        let (_, gained, _) = Game::resume(&save, 1_000 + 100, 0);
        assert!((gained - g.apply_offline(100.0, &mut vec![])).abs() < 1e-9);
        let (_, never_saved, _) = Game::resume(
            &SaveData {
                saved_at: 0,
                ..save
            },
            1_000_000,
            0,
        );
        assert_eq!(never_saved, 0.0);
    }

    #[test]
    fn save_roundtrip() {
        let mut g = Game::new();
        let mut ev = vec![];
        g.admin_grant(500.0, &mut ev);
        g.buy(0, &mut ev).unwrap();
        let save = g.to_save(123);
        let json = serde_json::to_string(&save).unwrap();
        let back: SaveData = serde_json::from_str(&json).unwrap();
        let g2 = Game::from_save(&back, 0);
        assert_eq!(g2.level(0), 1);
        assert_eq!(g2.bytes, g.bytes);
        for (i, def) in UPGRADES.iter().enumerate() {
            assert_eq!(g2.is_unlocked(i), g.is_unlocked(i), "{}", def.id);
        }
    }

    #[test]
    fn old_save_without_fields_loads() {
        let back: SaveData = serde_json::from_str(r#"{"bytes": 7}"#).unwrap();
        let g = Game::from_save(&back, 0);
        assert_eq!(g.whole_bytes(), 7.0);
    }
}
