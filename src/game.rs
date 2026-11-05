//! Spiellogik ohne UI. Kennt GPUI nicht und ist komplett ohne Fenster testbar.
//! Zeit wird immer von außen als Sekunden (`f64`) hereingereicht.
//!
//! Alle Mengen sind `f64`: damit gibt es praktisch kein Limit mehr (bis ~10^308),
//! statt bei `u64::MAX` (≈ 18,45 EB) stehen zu bleiben.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// Was ein Upgrade pro Level bewirkt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Effect {
    /// Passives Einkommen: +n Bytes pro Sekunde und Level.
    PerSecond(f64),
    /// Bytes pro Klick: +n pro Level (vor den Multiplikatoren).
    ClickAdd(f64),
    /// Multiplikator pro Klick: ×n pro Level (multiplikativ).
    ClickMul(f64),
    /// Multiplikator auf alles passive Einkommen: ×n pro Level (multiplikativ).
    PassiveMul(f64),
    /// Chance auf einen kritischen Klick (×[`CRIT_MULT`]): +n (0.02 = 2 %) pro Level.
    Crit(f64),
    /// Jeder Klick bringt zusätzlich n Sekunden passives Einkommen pro Level.
    ClickFromPassive(f64),
    /// Events kommen öfter (Abstand ×0.88) und halten länger (+12 %) pro Level.
    EventBoost,
    /// Verdient offline weiter: +n (0.1 = 10 %) des passiven Einkommens pro Level.
    Offline(f64),
    /// Alle Upgrades werden billiger: Kosten ×n pro Level.
    Discount(f64),
}

#[derive(Debug, Clone, Copy)]
pub struct UpgradeDef {
    pub id: &'static str,
    pub base_cost: f64,
    /// Kostenfaktor pro Level.
    pub growth: f64,
    /// Freigeschaltet, sobald insgesamt so viele Bytes verdient wurden.
    pub unlock_at: f64,
    pub effect: Effect,
}

/// Alle Upgrades in Anzeigereihenfolge, sortiert nach `unlock_at` (ein Test
/// prüft das). Der Spielstand speichert Level per `id`, alte Saves bleiben gültig.
pub const UPGRADES: &[UpgradeDef] = &[
    up("auto_click.sh", 15.0, 1.15, 0.0, Effect::PerSecond(1.0)),
    up("macro.kbd", 40.0, 1.25, 20.0, Effect::ClickAdd(1.0)),
    up("double.bin", 100.0, 3.0, 50.0, Effect::ClickMul(2.0)),
    up("cron.d", 500.0, 1.15, 400.0, Effect::PerSecond(10.0)),
    up("lucky.bit", 1_200.0, 2.2, 1_000.0, Effect::Crit(0.02)),
    up("gzip.d", 2_500.0, 3.0, 2_000.0, Effect::PassiveMul(1.5)),
    up("fork_daemon", 6_000.0, 1.15, 5_000.0, Effect::PerSecond(80.0)),
    up("notify.d", 9_000.0, 3.0, 7_500.0, Effect::EventBoost),
    up("hotkey.sh", 12_000.0, 1.3, 10_000.0, Effect::ClickAdd(25.0)),
    up("systemd.service", 25_000.0, 2.5, 20_000.0, Effect::Offline(0.1)),
    up("mmap.so", 40_000.0, 1.15, 30_000.0, Effect::PerSecond(450.0)),
    up("overclock.cfg", 100_000.0, 5.0, 80_000.0, Effect::ClickMul(3.0)),
    up("pipe.sh", 250_000.0, 2.5, 200_000.0, Effect::ClickFromPassive(0.5)),
    up("kernel.ko", 500_000.0, 1.15, 400_000.0, Effect::PerSecond(2_500.0)),
    up("compiler.o3", 1.5e6, 4.0, 1.2e6, Effect::Discount(0.95)),
    up("botnet.py", 6e6, 1.15, 5e6, Effect::PerSecond(18_000.0)),
    up("cluster.yaml", 8e7, 1.15, 6e7, Effect::PerSecond(140_000.0)),
    up("neural.link", 3e8, 8.0, 2.5e8, Effect::ClickMul(5.0)),
    up("quantum.qpu", 1e9, 1.15, 8e8, Effect::PerSecond(1.2e6)),
    up("ai.model", 4e9, 6.0, 3e9, Effect::PassiveMul(2.0)),
    up("dyson.sphere", 1.5e10, 1.15, 1.2e10, Effect::PerSecond(9e6)),
    up("simulation.exe", 2e11, 1.15, 1.5e11, Effect::PerSecond(7e7)),
    up("multiverse.fork", 3e12, 1.15, 2.5e12, Effect::PerSecond(5e8)),
    up("blackhole.dll", 5e13, 1.15, 4e13, Effect::PerSecond(4e9)),
    up("heat_death.sh", 1e15, 1.15, 8e14, Effect::PerSecond(3.5e10)),
    up("big_bang.iso", 2e16, 1.15, 1.6e16, Effect::PerSecond(3e11)),
    up("singularity.ai", 1e17, 10.0, 8e16, Effect::PassiveMul(3.0)),
    up("omega.point", 5e17, 1.15, 4e17, Effect::PerSecond(2.8e12)),
    up("infinity.loop", 1e19, 1.15, 8e18, Effect::PerSecond(2.5e13)),
    up("beyond.null", 3e20, 1.15, 2.4e20, Effect::PerSecond(2.2e14)),
];

const fn up(id: &'static str, base_cost: f64, growth: f64, unlock_at: f64, effect: Effect) -> UpgradeDef {
    UpgradeDef {
        id,
        base_cost,
        growth,
        unlock_at,
        effect,
    }
}

/// Ab hier löst jede neue Zehnerpotenz einen Meilenstein aus (1e3, 1e4, …).
pub const FIRST_MILESTONE_EXP: i32 = 3;

/// Kritischer Klick: so viel mal mehr.
pub const CRIT_MULT: f64 = 10.0;
/// Höchstens so wahrscheinlich ist ein kritischer Klick.
pub const CRIT_CAP: f64 = 0.5;
/// Offline-Einkommen zählt höchstens so lange (8 h).
pub const OFFLINE_CAP_SECS: f64 = 8.0 * 3600.0;

/// Zeitfenster für den gleitenden Klicks/s-Schnitt.
pub const RATE_WINDOW: f64 = 5.0;

#[derive(Debug, Clone, PartialEq)]
pub enum GameEvent {
    /// Neue Zehnerpotenz erreicht (Wert ist die Potenz, z. B. 1e6).
    Milestone(f64),
    Unlocked(&'static str),
    Bought { id: &'static str, level: u32 },
    BuffEnded(BuffKind),
}

// ── Events ────────────────────────────────────────────────────────────────

/// Zufalls-Events, die auf dem Bildschirm aufploppen und angeklickt werden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// ×4 auf allen Ertrag für 30 s.
    Boost,
    /// ×10 pro Klick für 15 s.
    Frenzy,
    /// Sofort ein Batzen Bytes.
    Windfall,
    /// Jeder Klick bringt 10 s passives Einkommen, 15 s lang.
    Chain,
    /// Alle Upgrades −50 %, 30 s lang.
    Sale,
    /// Glücksspiel: meist ×7 auf alles, manchmal ×0.5.
    Glitch,
    /// Bedrohung: anklicken wehrt ab (+Bytes), ignorieren = ×0.5 B/s für 30 s.
    Ddos,
}

/// Wie oft ein Event (relativ) auftaucht.
pub const EVENT_WEIGHTS: &[(EventKind, f64)] = &[
    (EventKind::Boost, 25.0),
    (EventKind::Frenzy, 15.0),
    (EventKind::Windfall, 15.0),
    (EventKind::Chain, 12.0),
    (EventKind::Sale, 12.0),
    (EventKind::Glitch, 11.0),
    (EventKind::Ddos, 10.0),
];

impl EventKind {
    pub fn title(self) -> &'static str {
        match self {
            EventKind::Boost => "BOOST ×4",
            EventKind::Frenzy => "KLICK-RAUSCH ×10",
            EventKind::Windfall => "DATENFUND",
            EventKind::Chain => "KETTENREAKTION",
            EventKind::Sale => "RABATT -50 %",
            EventKind::Glitch => "GLITCH ???",
            EventKind::Ddos => "DDOS-ANGRIFF",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            EventKind::Boost => "×4 auf allen ertrag, 30 s",
            EventKind::Frenzy => "×10 pro klick, 15 s",
            EventKind::Windfall => "sofort ein batzen bytes",
            EventKind::Chain => "jeder klick = 10 s passiv, 15 s",
            EventKind::Sale => "alle upgrades halber preis, 30 s",
            EventKind::Glitch => "meist ×7 auf alles. manchmal nicht.",
            EventKind::Ddos => "abwehren! sonst ×0.5 B/s, 30 s",
        }
    }

    /// Bedrohungen werden anders dargestellt und strafen das Ignorieren.
    pub fn is_threat(self) -> bool {
        self == EventKind::Ddos
    }
}

/// Laufende Wirkungen (aus Events).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuffKind {
    Boost,
    Frenzy,
    Chain,
    Sale,
    Overdrive,
    Lag,
}

pub const BOOST_MULT: f64 = 4.0;
pub const BOOST_SECS: f64 = 30.0;
pub const FRENZY_MULT: f64 = 10.0;
pub const FRENZY_SECS: f64 = 15.0;
pub const CHAIN_CLICK_SECS: f64 = 10.0;
pub const CHAIN_SECS: f64 = 15.0;
pub const SALE_MULT: f64 = 0.5;
pub const SALE_SECS: f64 = 30.0;
pub const OVERDRIVE_MULT: f64 = 7.0;
pub const OVERDRIVE_SECS: f64 = 20.0;
pub const LAG_MULT: f64 = 0.5;
pub const LAG_SECS: f64 = 30.0;
/// Chance, dass ein Glitch gut ausgeht.
pub const GLITCH_WIN: f64 = 0.65;

impl BuffKind {
    pub fn title(self) -> &'static str {
        match self {
            BuffKind::Boost => "BOOST ×4",
            BuffKind::Frenzy => "KLICK-RAUSCH ×10",
            BuffKind::Chain => "KETTENREAKTION",
            BuffKind::Sale => "RABATT -50 %",
            BuffKind::Overdrive => "OVERDRIVE ×7",
            BuffKind::Lag => "LAG ×0.5",
        }
    }

    /// Negative Wirkung (für die Darstellung).
    pub fn is_bad(self) -> bool {
        self == BuffKind::Lag
    }

    /// Was die Wirkung gerade tut (für die Bonus-Anzeige).
    pub fn effect_hint(self) -> &'static str {
        match self {
            BuffKind::Boost => "×4 alles",
            BuffKind::Frenzy => "×10 klick",
            BuffKind::Chain => "klick = 10 s passiv",
            BuffKind::Sale => "halber preis",
            BuffKind::Overdrive => "×7 alles",
            BuffKind::Lag => "×0.5 B/s",
        }
    }
}

/// Ein laufender Bonus (oder Malus).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Buff {
    pub kind: BuffKind,
    pub remaining: f64,
    pub duration: f64,
}

/// Ergebnis eines eingesammelten (oder verpassten) Events.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventOutcome {
    Buff { kind: BuffKind, seconds: f64 },
    Bytes(f64),
    /// DDoS abgewehrt: Belohnung in Bytes.
    Defended(f64),
}

/// Ergebnis eines Klicks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClickResult {
    pub gain: f64,
    pub crit: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BuyError {
    Locked,
    TooExpensive { cost: f64 },
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
    /// Unix-Sekunden des letzten Speicherns – für Offline-Einkommen.
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
    /// Laufende Event-Wirkungen (nicht gespeichert).
    buffs: Vec<Buff>,
    /// Zufall für kritische Klicks und Glitches (xorshift).
    rng: u64,
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
            rng: 0x2545_f491_4f6c_dd1d,
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

    /// Zufall neu setzen (die UI nimmt z. B. die Uhrzeit).
    pub fn seed(&mut self, seed: u64) {
        // splitmix64: verteilt auch kleine/ähnliche Seeds gut über alle Bits.
        let mut z = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        self.rng = (z ^ (z >> 31)).max(1);
    }

    fn random(&mut self) -> f64 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        (x >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Ganze Bytes, wie sie angezeigt werden.
    pub fn whole_bytes(&self) -> f64 {
        self.bytes.floor()
    }

    pub fn level(&self, index: usize) -> u32 {
        self.levels.get(index).copied().unwrap_or(0)
    }

    pub fn is_unlocked(&self, index: usize) -> bool {
        self.unlocked.get(index).copied().unwrap_or(false)
    }

    /// Summe bzw. Produkt einer Effekt-Art über alle gekauften Level.
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

    fn buff_product(&self, pick: impl Fn(BuffKind) -> f64) -> f64 {
        self.buffs.iter().map(|b| pick(b.kind)).product()
    }

    fn has_buff(&self, kind: BuffKind) -> bool {
        self.buffs.iter().any(|b| b.kind == kind)
    }

    // ── Werte ─────────────────────────────────────────────────────────────

    /// Bytes pro Klick (ohne kritischen Treffer).
    pub fn per_click(&self) -> f64 {
        let base = 1.0 + self.effect_sum(|e| match e {
            Effect::ClickAdd(n) => Some(n),
            _ => None,
        });
        let mul = self.effect_product(|e| match e {
            Effect::ClickMul(m) => Some(m),
            _ => None,
        });
        let mut passive_secs = self.effect_sum(|e| match e {
            Effect::ClickFromPassive(s) => Some(s),
            _ => None,
        });
        if self.has_buff(BuffKind::Chain) {
            passive_secs += CHAIN_CLICK_SECS;
        }
        base * mul * self.click_buff() + passive_secs * self.per_second()
    }

    /// Chance auf einen kritischen Klick (0…[`CRIT_CAP`]).
    pub fn crit_chance(&self) -> f64 {
        self.effect_sum(|e| match e {
            Effect::Crit(c) => Some(c),
            _ => None,
        })
        .min(CRIT_CAP)
    }

    /// Gemeinsamer Multiplikator der Upgrades auf alles passive Einkommen.
    pub fn passive_multiplier(&self) -> f64 {
        self.effect_product(|e| match e {
            Effect::PassiveMul(m) => Some(m),
            _ => None,
        })
    }

    /// Passives Einkommen eines einzelnen Upgrades in B/s (inkl. Multiplikatoren).
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

    /// Anteil des passiven Einkommens, der offline weiterläuft (0…1).
    pub fn offline_fraction(&self) -> f64 {
        self.effect_sum(|e| match e {
            Effect::Offline(f) => Some(f),
            _ => None,
        })
        .min(1.0)
    }

    /// Kostenfaktor für alle Upgrades (Rabatt-Upgrades und -Events).
    pub fn cost_multiplier(&self) -> f64 {
        let upgrades = self
            .effect_product(|e| match e {
                Effect::Discount(m) => Some(m),
                _ => None,
            })
            .max(0.25);
        let sale = if self.has_buff(BuffKind::Sale) { SALE_MULT } else { 1.0 };
        upgrades * sale
    }

    pub fn cost(&self, index: usize) -> f64 {
        let def = &UPGRADES[index];
        (def.base_cost * def.growth.powi(self.level(index) as i32) * self.cost_multiplier()).ceil()
    }

    fn event_boost_levels(&self) -> i32 {
        UPGRADES
            .iter()
            .zip(&self.levels)
            .filter(|(d, _)| d.effect == Effect::EventBoost)
            .map(|(_, &l)| l as i32)
            .sum()
    }

    /// Faktor auf den Abstand zwischen zwei Events (kleiner = öfter).
    pub fn event_interval_multiplier(&self) -> f64 {
        0.88f64.powi(self.event_boost_levels()).max(0.35)
    }

    /// Faktor auf die Dauer der Event-Boni.
    pub fn event_duration_multiplier(&self) -> f64 {
        1.0 + 0.12 * self.event_boost_levels() as f64
    }

    // ── Spielen ───────────────────────────────────────────────────────────

    /// Ein Klick zur Zeit `now` (Sekunden).
    pub fn click(&mut self, now: f64, events: &mut Vec<GameEvent>) -> ClickResult {
        let crit = self.crit_chance() > 0.0 && self.random() < self.crit_chance();
        let gain = self.per_click() * if crit { CRIT_MULT } else { 1.0 };
        self.total_clicks += 1;
        self.click_times.push_back(now);
        self.prune_clicks(now);
        self.earn(gain, events);
        ClickResult { gain, crit }
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

    /// Offline-Einkommen für `seconds` Abwesenheit gutschreiben (höchstens 8 h).
    pub fn apply_offline(&mut self, seconds: f64, events: &mut Vec<GameEvent>) -> f64 {
        let amount = self.per_second() * seconds.clamp(0.0, OFFLINE_CAP_SECS) * self.offline_fraction();
        if amount > 0.0 {
            self.earn(amount, events);
        }
        amount
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
        if self.bytes < cost {
            return Err(BuyError::TooExpensive { cost });
        }
        self.bytes -= cost;
        self.levels[index] += 1;
        let level = self.levels[index];
        events.push(GameEvent::Bought { id: def.id, level });
        Ok(level)
    }

    // ── Events ────────────────────────────────────────────────────────────

    fn add_buff(&mut self, kind: BuffKind, seconds: f64) -> EventOutcome {
        match self.buffs.iter_mut().find(|b| b.kind == kind) {
            Some(b) => {
                b.remaining = seconds;
                b.duration = seconds;
            }
            None => self.buffs.push(Buff { kind, remaining: seconds, duration: seconds }),
        }
        EventOutcome::Buff { kind, seconds }
    }

    /// Ein angeklicktes Event auslösen. Gleiche Wirkung nochmal = Zeit wieder voll.
    pub fn trigger_event(&mut self, kind: EventKind, events: &mut Vec<GameEvent>) -> EventOutcome {
        let dm = self.event_duration_multiplier();
        match kind {
            EventKind::Boost => self.add_buff(BuffKind::Boost, BOOST_SECS * dm),
            EventKind::Frenzy => self.add_buff(BuffKind::Frenzy, FRENZY_SECS * dm),
            EventKind::Chain => self.add_buff(BuffKind::Chain, CHAIN_SECS * dm),
            EventKind::Sale => self.add_buff(BuffKind::Sale, SALE_SECS * dm),
            EventKind::Windfall => {
                // Wie ein Glücksfund: 15 % vom Kontostand, aber höchstens
                // 15 Minuten passives Einkommen (plus ein kleiner Sockel).
                let amount = (self.bytes * 0.15).min(self.per_second() * 900.0).floor() + 50.0;
                self.earn(amount, events);
                EventOutcome::Bytes(amount)
            }
            EventKind::Glitch => {
                if self.random() < GLITCH_WIN {
                    self.add_buff(BuffKind::Overdrive, OVERDRIVE_SECS * dm)
                } else {
                    // Pech wird nicht durch notify.d verlängert.
                    self.add_buff(BuffKind::Lag, LAG_SECS / 3.0)
                }
            }
            EventKind::Ddos => {
                let reward = (self.per_second() * 60.0).floor() + 50.0;
                self.earn(reward, events);
                EventOutcome::Defended(reward)
            }
        }
    }

    /// Ein Event ist abgelaufen, ohne angeklickt zu werden.
    pub fn miss_event(&mut self, kind: EventKind) -> Option<EventOutcome> {
        kind.is_threat().then(|| self.add_buff(BuffKind::Lag, LAG_SECS))
    }

    pub fn buffs(&self) -> &[Buff] {
        &self.buffs
    }

    /// Multiplikator der laufenden Wirkungen auf passives Einkommen.
    pub fn passive_buff(&self) -> f64 {
        self.buff_product(|k| match k {
            BuffKind::Boost => BOOST_MULT,
            BuffKind::Overdrive => OVERDRIVE_MULT,
            BuffKind::Lag => LAG_MULT,
            _ => 1.0,
        })
    }

    /// Multiplikator der laufenden Wirkungen pro Klick.
    pub fn click_buff(&self) -> f64 {
        self.buff_product(|k| match k {
            BuffKind::Boost => BOOST_MULT,
            BuffKind::Frenzy => FRENZY_MULT,
            BuffKind::Overdrive => OVERDRIVE_MULT,
            _ => 1.0,
        })
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
        // Jede neu erreichte Zehnerpotenz ab 1e3 – höchstens drei pro Schritt,
        // damit ein riesiger Sprung (Admin) das Log nicht flutet.
        let exp = |v: f64| if v >= 1.0 { v.log10().floor() as i32 } else { -1 };
        let (from, to) = (exp(before).max(FIRST_MILESTONE_EXP - 1), exp(after));
        for e in (from + 1..=to).rev().take(3).collect::<Vec<_>>().into_iter().rev() {
            events.push(GameEvent::Milestone(10f64.powi(e)));
        }
        for (def, unlocked) in UPGRADES.iter().zip(self.unlocked.iter_mut()) {
            if !*unlocked && self.total_earned >= def.unlock_at {
                *unlocked = true;
                events.push(GameEvent::Unlocked(def.id));
            }
        }
    }

    fn refresh_unlocks(&mut self) {
        for (i, def) in UPGRADES.iter().enumerate() {
            if self.total_earned >= def.unlock_at || self.levels[i] > 0 {
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

    fn unlocked_rich() -> Game {
        let mut g = Game::new();
        let mut ev = vec![];
        g.admin_unlock_all(&mut ev);
        g.bytes = 1e30;
        g
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
        assert!(g.whole_bytes() > u64::MAX as f64);
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
            g.tick(0.1, i as f64 * 0.1, &mut ev);
        }
        assert!((g.bytes - 1.0).abs() < 1e-9);
    }

    #[test]
    fn cost_grows_per_level() {
        let mut g = Game::new();
        let mut ev = vec![];
        let i = idx("auto_click.sh");
        assert_eq!(g.cost(i), 15.0);
        g.bytes = 1000.0;
        g.buy(i, &mut ev).unwrap();
        assert_eq!(g.cost(i), (15.0f64 * 1.15).ceil());
    }

    #[test]
    fn double_multiplies_click() {
        let mut g = unlocked_rich();
        let mut ev = vec![];
        let d = idx("double.bin");
        g.buy(d, &mut ev).unwrap();
        assert_eq!(g.per_click(), 2.0);
        g.buy(d, &mut ev).unwrap();
        assert_eq!(g.per_click(), 4.0);
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
        assert_eq!(g.per_click(), 3.0);
        g.buy(idx("double.bin"), &mut ev).unwrap();
        assert_eq!(g.per_click(), 6.0);
    }

    #[test]
    fn passive_multiplier_scales_all_sources() {
        let mut g = unlocked_rich();
        let mut ev = vec![];
        g.buy(idx("auto_click.sh"), &mut ev).unwrap();
        g.buy(idx("cron.d"), &mut ev).unwrap();
        assert_eq!(g.per_second(), 11.0);
        g.buy(idx("gzip.d"), &mut ev).unwrap();
        assert!((g.per_second() - 16.5).abs() < 1e-9);
        assert!((g.source_rate(idx("cron.d")) - 15.0).abs() < 1e-9);
    }

    #[test]
    fn crit_chance_is_capped_and_crits_happen() {
        let mut g = unlocked_rich();
        let mut ev = vec![];
        for _ in 0..40 {
            g.buy(idx("lucky.bit"), &mut ev).unwrap();
        }
        assert_eq!(g.crit_chance(), CRIT_CAP);
        let crits = (0..400).filter(|i| g.click(*i as f64, &mut ev).crit).count();
        assert!((120..280).contains(&crits), "{crits} krits");
    }

    #[test]
    fn click_from_passive_adds_seconds_of_income() {
        let mut g = unlocked_rich();
        let mut ev = vec![];
        g.buy(idx("cron.d"), &mut ev).unwrap();
        g.buy(idx("pipe.sh"), &mut ev).unwrap();
        // 1 B Grundwert + 0.5 s × 10 B/s
        assert_eq!(g.per_click(), 6.0);
    }

    #[test]
    fn discount_makes_everything_cheaper_but_has_floor() {
        let mut g = unlocked_rich();
        let mut ev = vec![];
        let base = g.cost(idx("kernel.ko"));
        g.buy(idx("compiler.o3"), &mut ev).unwrap();
        assert_eq!(g.cost(idx("kernel.ko")), (base * 0.95).ceil());
        for _ in 0..30 {
            g.buy(idx("compiler.o3"), &mut ev).unwrap();
        }
        assert!(g.cost_multiplier() >= 0.25);
    }

    #[test]
    fn event_boost_shortens_gap_and_extends_duration() {
        let mut g = unlocked_rich();
        let mut ev = vec![];
        g.buy(idx("notify.d"), &mut ev).unwrap();
        assert!(g.event_interval_multiplier() < 1.0);
        let EventOutcome::Buff { seconds, .. } = g.trigger_event(EventKind::Boost, &mut ev) else {
            panic!("boost ist ein buff");
        };
        assert!(seconds > BOOST_SECS);
    }

    #[test]
    fn offline_income_needs_upgrade_and_is_capped() {
        let mut g = unlocked_rich();
        let mut ev = vec![];
        g.buy(idx("auto_click.sh"), &mut ev).unwrap();
        assert_eq!(g.apply_offline(3600.0, &mut ev), 0.0);
        g.buy(idx("systemd.service"), &mut ev).unwrap();
        assert!((g.apply_offline(100.0, &mut ev) - 10.0).abs() < 1e-9);
        let capped = g.apply_offline(1e9, &mut ev);
        assert!((capped - OFFLINE_CAP_SECS * 0.1).abs() < 1e-6);
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
        g.played_seconds = 42.0;
        g.admin_level_all();
        assert_eq!(g.upgrades_bought(), UPGRADES.len() as u32);
        assert_eq!(g.unlocked_count(), UPGRADES.len());
        g.admin_reset();
        assert_eq!(g.upgrades_bought(), 0);
        assert_eq!(g.whole_bytes(), 0.0);
        assert_eq!(g.played_seconds, 42.0);
    }

    #[test]
    fn boost_multiplies_click_and_passive_then_ends() {
        let mut g = Game::new();
        let mut ev = vec![];
        g.bytes = 15.0;
        g.buy(idx("auto_click.sh"), &mut ev).unwrap();
        g.trigger_event(EventKind::Boost, &mut ev);
        assert_eq!(g.per_click(), 4.0);
        assert_eq!(g.per_second(), 4.0);
        g.tick(BOOST_SECS + 0.1, 0.0, &mut ev);
        assert!(ev.contains(&GameEvent::BuffEnded(BuffKind::Boost)));
        assert_eq!(g.per_click(), 1.0);
        assert_eq!(g.per_second(), 1.0);
    }

    #[test]
    fn frenzy_only_clicks_and_retrigger_refreshes() {
        let mut g = Game::new();
        let mut ev = vec![];
        g.trigger_event(EventKind::Frenzy, &mut ev);
        assert_eq!(g.per_click(), 10.0);
        g.tick(10.0, 0.0, &mut ev);
        g.trigger_event(EventKind::Frenzy, &mut ev);
        assert_eq!(g.buffs().len(), 1);
        assert_eq!(g.buffs()[0].remaining, FRENZY_SECS);
    }

    #[test]
    fn sale_halves_costs() {
        let mut g = Game::new();
        let mut ev = vec![];
        let base = g.cost(0);
        g.trigger_event(EventKind::Sale, &mut ev);
        assert_eq!(g.cost(0), (base * SALE_MULT).ceil());
    }

    #[test]
    fn chain_turns_clicks_into_passive_income() {
        let mut g = unlocked_rich();
        let mut ev = vec![];
        g.buy(idx("cron.d"), &mut ev).unwrap();
        g.trigger_event(EventKind::Chain, &mut ev);
        assert_eq!(g.per_click(), 1.0 + CHAIN_CLICK_SECS * 10.0);
    }

    #[test]
    fn glitch_is_usually_good() {
        let mut wins = 0;
        for seed in 1..200u64 {
            let mut g = Game::new();
            g.seed(seed * 7919);
            let mut ev = vec![];
            if g.trigger_event(EventKind::Glitch, &mut ev)
                == (EventOutcome::Buff { kind: BuffKind::Overdrive, seconds: OVERDRIVE_SECS })
            {
                wins += 1;
            }
        }
        assert!((90..170).contains(&wins), "{wins} gewonnen");
    }

    #[test]
    fn ddos_rewards_defense_and_punishes_ignoring() {
        let mut g = Game::new();
        let mut ev = vec![];
        g.bytes = 15.0;
        g.buy(idx("auto_click.sh"), &mut ev).unwrap();
        assert_eq!(g.trigger_event(EventKind::Ddos, &mut ev), EventOutcome::Defended(110.0));
        assert!(g.miss_event(EventKind::Boost).is_none());
        g.miss_event(EventKind::Ddos);
        assert_eq!(g.per_second(), LAG_MULT);
    }

    #[test]
    fn windfall_is_capped_by_income() {
        let mut g = Game::new();
        let mut ev = vec![];
        g.bytes = 1_000_000.0;
        // Kein passives Einkommen: nur der Sockel.
        assert_eq!(g.trigger_event(EventKind::Windfall, &mut ev), EventOutcome::Bytes(50.0));
    }

    #[test]
    fn too_expensive() {
        let mut g = Game::new();
        let mut ev = vec![];
        assert_eq!(g.buy(0, &mut ev), Err(BuyError::TooExpensive { cost: 15.0 }));
    }

    #[test]
    fn milestone_fires_once() {
        let mut g = Game::new();
        let mut ev = vec![];
        g.bytes = 999.0;
        g.total_earned = 999.0;
        g.click(0.0, &mut ev);
        g.click(0.1, &mut ev);
        let n = ev.iter().filter(|e| **e == GameEvent::Milestone(1000.0)).count();
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
        assert_eq!(g.whole_bytes(), 7.0);
    }
}
