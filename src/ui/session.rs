//! Der Zustand einer Spielsitzung, den alle Ansichten lesen und ändern.

use crate::game::events::EventDirector;
use crate::game::{Game, GameEvent};
use crate::rng::Rng;
use crate::settings::Settings;
use crate::ui::fx::Animator;
use crate::ui::log::Log;
use std::time::Instant;

pub struct Session {
    pub game: Game,
    pub settings: Settings,
    pub log: Log,
    pub events: EventDirector,
    pub anim: Animator,
    /// Zufall für Deko (Positionen, Scramble) – getrennt vom Spiel-Zufall.
    pub rng: Rng,
    /// Mit `--admin` gestartet.
    pub admin: bool,
    /// Nach dem Boot: Zeit, Events und Navigation laufen.
    pub running: bool,
    /// Faktor auf das passive Einkommen (Admin-Zeitraffer).
    pub time_scale: f64,
    /// Was im Spiel passiert ist und noch dargestellt werden muss.
    pub happened: Vec<GameEvent>,
    /// Einkommen je Quelle aus dem letzten Tick (für die Datenpakete).
    pub income: Vec<(usize, f64)>,
    pub save_game: bool,
    pub save_settings: bool,
    /// Ein unlesbarer Spielstand ließ sich nicht beiseitelegen: nicht überschreiben.
    pub save_blocked: bool,
    started: Instant,
}

impl Session {
    pub fn new(game: Game, settings: Settings, log: Log, admin: bool, seed: u64) -> Self {
        Self {
            game,
            settings,
            log,
            events: EventDirector::new(seed ^ 0xe7e7),
            anim: Animator::default(),
            rng: Rng::new(seed ^ 0xdec0),
            admin,
            running: false,
            time_scale: 1.0,
            happened: Vec::new(),
            income: Vec::new(),
            save_game: false,
            save_settings: false,
            save_blocked: false,
            started: Instant::now(),
        }
    }

    /// Sekunden seit Sitzungsbeginn – die Uhr für Klickrate und Events.
    pub fn now(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }

    /// Das Spiel beginnt: Zeit und Events laufen ab jetzt.
    pub fn start(&mut self) {
        self.running = true;
        self.log.event("boot ok. viel spaß.");
        let now = self.now();
        self.events.start(now, &self.game);
    }
}
