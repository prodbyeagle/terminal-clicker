//! Zufalls-Events: was sie bewirken, wann sie kommen, wie lange sie bleiben.

use super::{Effect, Game, GameEvent};
use crate::rng::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Boost,
    Frenzy,
    Windfall,
    Chain,
    Sale,
    Glitch,
    Ddos,
}

const BOOST_MULT: f64 = 4.0;
const FRENZY_MULT: f64 = 10.0;
const CHAIN_CLICK_SECS: f64 = 10.0;
const SALE_MULT: f64 = 0.5;
const OVERDRIVE_MULT: f64 = 7.0;
const LAG_MULT: f64 = 0.5;
/// Ein verpasster DDoS lahmt so lange.
const LAG_SECS: f64 = 30.0;
/// Pech beim Glitch ist kürzer als ein verpasster DDoS und wird nicht verlängert.
const GLITCH_LAG_SECS: f64 = 10.0;
const GLITCH_WIN: f64 = 0.65;
/// Datenfund: dieser Anteil vom Kontostand, …
const WINDFALL_SHARE: f64 = 0.15;
/// … aber höchstens so viele Sekunden passives Einkommen.
const WINDFALL_CAP_SECS: f64 = 900.0;
/// DDoS abgewehrt: so viele Sekunden passives Einkommen als Belohnung.
const DDOS_REWARD_SECS: f64 = 60.0;
/// Sockel für Datenfund und DDoS-Belohnung, damit auch am Anfang etwas kommt.
const REWARD_BASE: f64 = 50.0;

/// So lange (s) bleibt ein Event stehen, bevor es verpasst ist.
const EVENT_LIFETIME: f64 = 12.0;
/// Abstand zwischen zwei Events (zufällig dazwischen), in Sekunden.
const EVENT_GAP: (f64, f64) = (45.0, 110.0);
const FIRST_EVENT_GAP: (f64, f64) = (20.0, 40.0);
/// Pro Level `notify.d`: Abstand ×0.88 (nie unter ×0.35), Dauer +12 %.
const EVENT_BOOST_GAP: f64 = 0.88;
const EVENT_BOOST_GAP_FLOOR: f64 = 0.35;
const EVENT_BOOST_DURATION: f64 = 0.12;

/// Was ein eingesammeltes Event tut.
enum Action {
    Buff(BuffKind),
    Windfall,
    Gamble,
    Defend,
}

impl EventKind {
    pub const ALL: [EventKind; 7] = [
        EventKind::Boost,
        EventKind::Frenzy,
        EventKind::Windfall,
        EventKind::Chain,
        EventKind::Sale,
        EventKind::Glitch,
        EventKind::Ddos,
    ];

    /// Wie oft das Event (relativ) auftaucht.
    fn weight(self) -> f64 {
        match self {
            EventKind::Boost => 25.0,
            EventKind::Frenzy | EventKind::Windfall => 15.0,
            EventKind::Chain | EventKind::Sale => 12.0,
            EventKind::Glitch => 11.0,
            EventKind::Ddos => 10.0,
        }
    }

    fn action(self) -> Action {
        match self {
            EventKind::Boost => Action::Buff(BuffKind::Boost),
            EventKind::Frenzy => Action::Buff(BuffKind::Frenzy),
            EventKind::Chain => Action::Buff(BuffKind::Chain),
            EventKind::Sale => Action::Buff(BuffKind::Sale),
            EventKind::Windfall => Action::Windfall,
            EventKind::Glitch => Action::Gamble,
            EventKind::Ddos => Action::Defend,
        }
    }

    pub fn title(self) -> String {
        match self.action() {
            Action::Buff(buff) => buff.title(),
            Action::Windfall => "DATENFUND".into(),
            Action::Gamble => "GLITCH ???".into(),
            Action::Defend => "DDOS-ANGRIFF".into(),
        }
    }

    /// Name im Log und in Listen.
    pub fn name(self) -> String {
        self.title().to_lowercase()
    }

    pub fn description(self) -> String {
        match self.action() {
            Action::Buff(buff) => format!("{}, {} s", buff.effect_hint(), buff.base_secs()),
            Action::Windfall => "sofort ein batzen bytes".into(),
            Action::Gamble => format!("meist ×{OVERDRIVE_MULT} auf alles. manchmal nicht."),
            Action::Defend => format!("abwehren! sonst ×{LAG_MULT} B/s, {LAG_SECS} s"),
        }
    }

    pub fn is_threat(self) -> bool {
        matches!(self.action(), Action::Defend)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuffKind {
    Boost,
    Frenzy,
    Chain,
    Sale,
    Overdrive,
    Lag,
}

impl BuffKind {
    /// Dauer, wenn ein Event diese Wirkung startet (vor `notify.d`).
    fn base_secs(self) -> f64 {
        match self {
            BuffKind::Boost | BuffKind::Sale | BuffKind::Lag => 30.0,
            BuffKind::Frenzy | BuffKind::Chain => 15.0,
            BuffKind::Overdrive => 20.0,
        }
    }

    pub fn title(self) -> String {
        match self {
            BuffKind::Boost => format!("BOOST ×{BOOST_MULT}"),
            BuffKind::Frenzy => format!("KLICK-RAUSCH ×{FRENZY_MULT}"),
            BuffKind::Chain => "KETTENREAKTION".into(),
            BuffKind::Sale => format!("RABATT -{} %", (1.0 - SALE_MULT) * 100.0),
            BuffKind::Overdrive => format!("OVERDRIVE ×{OVERDRIVE_MULT}"),
            BuffKind::Lag => format!("LAG ×{LAG_MULT}"),
        }
    }

    pub fn name(self) -> String {
        self.title().to_lowercase()
    }

    pub fn effect_hint(self) -> String {
        match self {
            BuffKind::Boost => format!("×{BOOST_MULT} auf allen ertrag"),
            BuffKind::Frenzy => format!("×{FRENZY_MULT} pro klick"),
            BuffKind::Chain => format!("jeder klick = {CHAIN_CLICK_SECS} s passiv"),
            BuffKind::Sale => format!("alle upgrades ×{SALE_MULT}"),
            BuffKind::Overdrive => format!("×{OVERDRIVE_MULT} auf alles"),
            BuffKind::Lag => format!("×{LAG_MULT} B/s"),
        }
    }

    pub fn is_bad(self) -> bool {
        self == BuffKind::Lag
    }

    fn passive_mult(self) -> f64 {
        match self {
            BuffKind::Boost => BOOST_MULT,
            BuffKind::Overdrive => OVERDRIVE_MULT,
            BuffKind::Lag => LAG_MULT,
            BuffKind::Frenzy | BuffKind::Chain | BuffKind::Sale => 1.0,
        }
    }

    fn click_mult(self) -> f64 {
        match self {
            BuffKind::Boost => BOOST_MULT,
            BuffKind::Frenzy => FRENZY_MULT,
            BuffKind::Overdrive => OVERDRIVE_MULT,
            BuffKind::Chain | BuffKind::Sale | BuffKind::Lag => 1.0,
        }
    }

    /// Sekunden passives Einkommen, die jeder Klick zusätzlich bringt.
    fn click_passive_secs(self) -> f64 {
        if self == BuffKind::Chain {
            CHAIN_CLICK_SECS
        } else {
            0.0
        }
    }

    fn cost_mult(self) -> f64 {
        if self == BuffKind::Sale {
            SALE_MULT
        } else {
            1.0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Buff {
    pub kind: BuffKind,
    pub remaining: f64,
    pub duration: f64,
}

/// Was ein eingesammeltes oder verpasstes Event bewirkt hat.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventOutcome {
    Buff {
        kind: BuffKind,
        seconds: f64,
    },
    /// Datenfund oder abgewehrter DDoS.
    Bytes(f64),
}

/// Alle laufenden Wirkungen und was sie zusammen bewirken.
#[derive(Debug, Clone, Default)]
pub struct Buffs(Vec<Buff>);

impl Buffs {
    /// Gleiche Wirkung nochmal = Zeit wieder voll.
    fn add(&mut self, kind: BuffKind, seconds: f64) -> EventOutcome {
        match self.0.iter_mut().find(|b| b.kind == kind) {
            Some(b) => {
                b.remaining = seconds;
                b.duration = seconds;
            }
            None => self.0.push(Buff {
                kind,
                remaining: seconds,
                duration: seconds,
            }),
        }
        EventOutcome::Buff { kind, seconds }
    }

    pub fn list(&self) -> &[Buff] {
        &self.0
    }

    pub(super) fn passive_mult(&self) -> f64 {
        self.0.iter().map(|b| b.kind.passive_mult()).product()
    }

    pub(super) fn click_mult(&self) -> f64 {
        self.0.iter().map(|b| b.kind.click_mult()).product()
    }

    pub(super) fn click_passive_secs(&self) -> f64 {
        self.0.iter().map(|b| b.kind.click_passive_secs()).sum()
    }

    pub(super) fn cost_mult(&self) -> f64 {
        self.0.iter().map(|b| b.kind.cost_mult()).product()
    }

    pub(super) fn tick(&mut self, dt: f64, events: &mut Vec<GameEvent>) {
        for buff in &mut self.0 {
            buff.remaining -= dt;
        }
        self.0.retain(|b| {
            let alive = b.remaining > 0.0;
            if !alive {
                events.push(GameEvent::BuffEnded(b.kind));
            }
            alive
        });
    }
}

impl Game {
    fn trigger_event(&mut self, kind: EventKind, events: &mut Vec<GameEvent>) -> EventOutcome {
        let longer = self.event_duration_multiplier();
        match kind.action() {
            Action::Buff(buff) => self.buffs.add(buff, buff.base_secs() * longer),
            Action::Windfall => {
                let amount =
                    (self.bytes * WINDFALL_SHARE).min(self.per_second() * WINDFALL_CAP_SECS);
                let amount = amount.floor() + REWARD_BASE;
                self.earn(amount, events);
                EventOutcome::Bytes(amount)
            }
            Action::Gamble if self.rng.chance(GLITCH_WIN) => self.buffs.add(
                BuffKind::Overdrive,
                BuffKind::Overdrive.base_secs() * longer,
            ),
            Action::Gamble => self.buffs.add(BuffKind::Lag, GLITCH_LAG_SECS),
            Action::Defend => {
                let reward = (self.per_second() * DDOS_REWARD_SECS).floor() + REWARD_BASE;
                self.earn(reward, events);
                EventOutcome::Bytes(reward)
            }
        }
    }

    /// Ein Event ist abgelaufen, ohne angeklickt zu werden.
    fn miss_event(&mut self, kind: EventKind) -> Option<EventOutcome> {
        kind.is_threat()
            .then(|| self.buffs.add(BuffKind::Lag, LAG_SECS))
    }

    pub fn buffs(&self) -> &[Buff] {
        self.buffs.list()
    }

    fn event_boost_levels(&self) -> i32 {
        self.effect_sum(|e| (e == Effect::EventBoost).then_some(1.0)) as i32
    }

    /// Faktor auf den Abstand zwischen zwei Events (kleiner = öfter).
    pub fn event_interval_multiplier(&self) -> f64 {
        EVENT_BOOST_GAP
            .powi(self.event_boost_levels())
            .max(EVENT_BOOST_GAP_FLOOR)
    }

    fn event_duration_multiplier(&self) -> f64 {
        1.0 + EVENT_BOOST_DURATION * self.event_boost_levels() as f64
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActiveEvent {
    /// Fortlaufend pro Sitzung – die Darstellung leitet daraus z. B. die Position ab.
    pub id: u64,
    pub kind: EventKind,
    spawned_at: f64,
}

impl ActiveEvent {
    pub fn age(&self, now: f64) -> f64 {
        (now - self.spawned_at).max(0.0)
    }

    pub fn remaining(&self, now: f64) -> f64 {
        (EVENT_LIFETIME - self.age(now)).max(0.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DirectorNews {
    Spawned(EventKind),
    Missed {
        kind: EventKind,
        outcome: Option<EventOutcome>,
    },
}

/// Taktgeber für Events. Zeit in Sekunden der Sitzung.
#[derive(Debug)]
pub struct EventDirector {
    next_at: Option<f64>,
    active: Option<ActiveEvent>,
    spawned: u64,
    rng: Rng,
}

impl EventDirector {
    pub fn new(seed: u64) -> Self {
        Self {
            next_at: None,
            active: None,
            spawned: 0,
            rng: Rng::new(seed),
        }
    }

    /// Startet den Takt; das erste Event kommt nach `FIRST_EVENT_GAP`.
    pub fn start(&mut self, now: f64, game: &Game) {
        self.schedule(now, FIRST_EVENT_GAP, game);
    }

    fn schedule(&mut self, now: f64, (lo, hi): (f64, f64), game: &Game) {
        let gap = lo + (hi - lo) * self.rng.unit();
        self.next_at = Some(now + gap * game.event_interval_multiplier());
    }

    /// Ein Event nach Gewicht auswürfeln.
    pub fn random_kind(&mut self) -> EventKind {
        let weighted: Vec<_> = EventKind::ALL.iter().map(|k| (*k, k.weight())).collect();
        self.rng.weighted(&weighted)
    }

    /// `can_spawn`: ob ein neues Event gerade gesehen werden kann.
    pub fn tick(&mut self, now: f64, can_spawn: bool, game: &mut Game) -> Option<DirectorNews> {
        if let Some(active) = self.active {
            if active.age(now) <= EVENT_LIFETIME {
                return None;
            }
            self.active = None;
            self.schedule(now, EVENT_GAP, game);
            return Some(DirectorNews::Missed {
                kind: active.kind,
                outcome: game.miss_event(active.kind),
            });
        }
        if can_spawn && self.next_at.is_some_and(|t| now >= t) {
            let kind = self.random_kind();
            self.spawn(kind, now);
            return Some(DirectorNews::Spawned(kind));
        }
        None
    }

    /// Ein Event sofort erscheinen lassen (ersetzt ein stehendes).
    pub fn spawn(&mut self, kind: EventKind, now: f64) {
        self.spawned += 1;
        self.active = Some(ActiveEvent {
            id: self.spawned,
            kind,
            spawned_at: now,
        });
    }

    pub fn collect(
        &mut self,
        now: f64,
        game: &mut Game,
        events: &mut Vec<GameEvent>,
    ) -> Option<(EventKind, EventOutcome)> {
        let active = self.active.take()?;
        self.schedule(now, EVENT_GAP, game);
        Some((active.kind, game.trigger_event(active.kind, events)))
    }

    pub fn active(&self) -> Option<ActiveEvent> {
        self.active
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::testkit::{idx, unlocked_rich};

    fn with_income() -> Game {
        let mut g = unlocked_rich();
        g.buy(idx("cron.d"), &mut vec![]).unwrap();
        g
    }

    fn buff_secs(outcome: EventOutcome) -> f64 {
        match outcome {
            EventOutcome::Buff { seconds, .. } => seconds,
            other => panic!("kein buff: {other:?}"),
        }
    }

    #[test]
    fn every_event_can_be_rolled() {
        let mut d = EventDirector::new(0);
        let rolled: Vec<_> = (0..2000).map(|_| d.random_kind()).collect();
        for kind in EventKind::ALL {
            assert!(rolled.contains(&kind), "{kind:?}");
        }
    }

    #[test]
    fn notify_d_makes_events_come_sooner_and_last_longer() {
        let mut plain = unlocked_rich();
        let mut boosted = unlocked_rich();
        boosted.buy(idx("notify.d"), &mut vec![]).unwrap();
        assert!(boosted.event_interval_multiplier() < plain.event_interval_multiplier());
        let short = buff_secs(plain.trigger_event(EventKind::Boost, &mut vec![]));
        let long = buff_secs(boosted.trigger_event(EventKind::Boost, &mut vec![]));
        assert!(long > short);
    }

    #[test]
    fn boost_raises_click_and_passive_until_it_runs_out() {
        let mut g = with_income();
        let mut ev = vec![];
        let (click, passive) = (g.per_click(), g.per_second());
        let secs = buff_secs(g.trigger_event(EventKind::Boost, &mut ev));
        assert!(g.per_click() > click && g.per_second() > passive);
        g.tick(secs + 0.1, 1.0, 0.0, &mut ev);
        assert!(ev.contains(&GameEvent::BuffEnded(BuffKind::Boost)));
        assert_eq!((g.per_click(), g.per_second()), (click, passive));
    }

    #[test]
    fn frenzy_only_raises_clicks_and_retrigger_refreshes() {
        let mut g = with_income();
        let mut ev = vec![];
        let (click, passive) = (g.per_click(), g.per_second());
        let secs = buff_secs(g.trigger_event(EventKind::Frenzy, &mut ev));
        assert!(g.per_click() > click);
        assert_eq!(g.per_second(), passive);
        g.tick(secs / 2.0, 1.0, 0.0, &mut ev);
        g.trigger_event(EventKind::Frenzy, &mut ev);
        assert_eq!(g.buffs().len(), 1);
        assert_eq!(g.buffs()[0].remaining, secs);
    }

    #[test]
    fn sale_makes_upgrades_cheaper_while_it_lasts() {
        let mut g = Game::new();
        let mut ev = vec![];
        let base = g.cost(0);
        let secs = buff_secs(g.trigger_event(EventKind::Sale, &mut ev));
        assert!(g.cost(0) < base);
        g.tick(secs + 0.1, 1.0, 0.0, &mut ev);
        assert_eq!(g.cost(0), base);
    }

    #[test]
    fn chain_turns_clicks_into_passive_income() {
        let mut g = with_income();
        let (click, passive) = (g.per_click(), g.per_second());
        g.trigger_event(EventKind::Chain, &mut vec![]);
        assert_eq!(g.per_second(), passive);
        assert_eq!(g.per_click(), click + CHAIN_CLICK_SECS * passive);
    }

    #[test]
    fn glitch_is_usually_good() {
        let wins = (1..200u64)
            .filter(|seed| {
                let mut g = Game::seeded(*seed);
                matches!(
                    g.trigger_event(EventKind::Glitch, &mut vec![]),
                    EventOutcome::Buff {
                        kind: BuffKind::Overdrive,
                        ..
                    }
                )
            })
            .count();
        assert!((90..170).contains(&wins), "{wins} gewonnen");
    }

    #[test]
    fn ddos_rewards_defense_and_punishes_ignoring() {
        let mut g = with_income();
        g.bytes = 1_000.0;
        let (bytes, passive) = (g.bytes, g.per_second());
        assert!(matches!(
            g.trigger_event(EventKind::Ddos, &mut vec![]),
            EventOutcome::Bytes(_)
        ));
        assert!(g.bytes > bytes);
        assert!(g.miss_event(EventKind::Boost).is_none());
        g.miss_event(EventKind::Ddos);
        assert!(g.per_second() < passive);
    }

    #[test]
    fn windfall_without_income_stays_small_even_when_rich() {
        let mut g = Game::new();
        g.bytes = 1_000_000.0;
        let EventOutcome::Bytes(found) = g.trigger_event(EventKind::Windfall, &mut vec![]) else {
            panic!("datenfund bringt bytes");
        };
        assert!(found > 0.0 && found < 1_000.0, "{found}");
    }

    #[test]
    fn director_waits_for_the_gap_and_a_visible_screen() {
        let mut g = Game::new();
        let mut d = EventDirector::new(0);
        d.start(0.0, &g);
        assert_eq!(d.tick(FIRST_EVENT_GAP.0 - 1.0, true, &mut g), None);
        assert_eq!(d.tick(FIRST_EVENT_GAP.1 + 1.0, false, &mut g), None);
        assert!(matches!(
            d.tick(FIRST_EVENT_GAP.1 + 1.0, true, &mut g),
            Some(DirectorNews::Spawned(_))
        ));
        assert!(d.active().is_some());
    }

    #[test]
    fn unclicked_ddos_is_missed_and_lags() {
        let mut g = Game::new();
        let mut d = EventDirector::new(0);
        d.spawn(EventKind::Ddos, 0.0);
        assert_eq!(d.tick(EVENT_LIFETIME, true, &mut g), None);
        let news = d.tick(EVENT_LIFETIME + 0.1, true, &mut g);
        assert!(matches!(
            news,
            Some(DirectorNews::Missed {
                kind: EventKind::Ddos,
                outcome: Some(EventOutcome::Buff {
                    kind: BuffKind::Lag,
                    ..
                })
            })
        ));
        assert!(d.active().is_none());
    }

    #[test]
    fn collecting_triggers_once_and_schedules_the_next() {
        let mut g = Game::new();
        let mut ev = vec![];
        let mut d = EventDirector::new(0);
        d.spawn(EventKind::Boost, 0.0);
        assert!(matches!(
            d.collect(1.0, &mut g, &mut ev),
            Some((EventKind::Boost, EventOutcome::Buff { .. }))
        ));
        assert!(d.collect(1.0, &mut g, &mut ev).is_none());
        assert!(d.tick(1.0 + EVENT_GAP.0 - 1.0, true, &mut g).is_none());
        assert!(d.tick(1.0 + EVENT_GAP.1 + 1.0, true, &mut g).is_some());
    }
}
