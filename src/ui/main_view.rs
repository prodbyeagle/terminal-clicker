use crate::ascii::anim::{AnimKind, Animator, CounterRoll, RollDir, decode_text, scramble_frames};
use crate::ascii::background::{SPEEDS, render_rows};
use crate::format::{self, NumberFormat};
use crate::game::{BuyError, EventKind, EventOutcome, Game, GameEvent, UPGRADES};
use crate::save::{self, Settings};
use crate::theme::{self, Theme};
use crate::ui::admin::{self as admin_ui, AdminInfo, TIME_SCALES, bar, fmt_num, render_admin};
use crate::ui::help::render_help;
use crate::ui::log::{LOG_PANEL_W, Log, render_log};
use crate::ui::settings::{self as settings_ui, render_settings};
use crate::ui::status_bar::{KeyHint, render_status_bar};
use crate::ui::upgrades::render_upgrades;
use crate::ui::window_drag::begin_window_drag;
use crate::{
    Back, Click, CollectEvent, Confirm, Down, Left, OpenAdmin, OpenHelp, OpenSettings, OpenUpgrades,
    Quit, Right, Up,
};
use gpui::{
    Context, Div, ScrollHandle, FocusHandle, MouseButton, Subscription, Task, Window, WindowAppearance,
    div, prelude::*, px,
};
use std::time::{Duration, Instant};

pub const FONT: &str = "JetBrains Mono";
pub const FONT_SIZE: f32 = 14.;
/// JetBrains Mono: Vorschub 600/1000 em.
const CELL_W: f32 = FONT_SIZE * 0.6;
const BG_LINE_H: f32 = 18.;
const LINE_H: f32 = 20.;
pub const TITLE_BAR_H: f32 = 36.;
/// Abstand der Ampel vom linken Fensterrand (an `TitlebarOptions` übergeben).
pub const TRAFFIC_LIGHTS_X: f32 = 14.;
/// Sichtbare Breite der drei Ampel-Kreise (linker Rand rot bis rechter Rand grün).
const TRAFFIC_LIGHTS_SPAN: f32 = 60.;
/// Links freigehaltene Fläche: Rand + Ampel + gleich viel Rand rechts davon.
const TRAFFIC_LIGHTS_W: f32 = TRAFFIC_LIGHTS_X + TRAFFIC_LIGHTS_SPAN + TRAFFIC_LIGHTS_X;

const TICK: Duration = Duration::from_millis(100);
const BG_FRAME: Duration = Duration::from_millis(55); // ≈ 18 fps
const AUTOSAVE: Duration = Duration::from_secs(30);
const PRESS: Duration = Duration::from_millis(80);
const CURSOR_BLINK_MS: u128 = 530;
/// Dauer, die eine Ziffer des Zählers zum Durchrollen braucht.
const COUNTER_ROLL_MS: u64 = 140;
/// Lebensdauer eines Datenpakets (passives Einkommen) auf dem Bildschirm.
const PACKET_MS: u64 = 1800;
/// Höchstens so viele Pakete gleichzeitig – was darüber hinausgeht, wird ins
/// nächste Paket desselben Upgrades zusammengefasst.
const MAX_PACKETS: usize = 12;
const STATUS_BAR_H: f32 = 30.;
/// So lange bleibt ein Event-Popup stehen, bevor es verschwindet.
const EVENT_LIFETIME: Duration = Duration::from_secs(12);
/// Abstand zwischen zwei Events (zufällig dazwischen), in Sekunden.
const EVENT_GAP: (f32, f32) = (45., 110.);
/// Das erste Event kommt etwas früher.
const FIRST_EVENT_GAP: (f32, f32) = (20., 40.);
const EVENT_W: f32 = 240.;
/// So lange wartet „spielstand zurücksetzen“ auf die Bestätigung.
const RESET_CONFIRM: Duration = Duration::from_secs(4);
const COUNTER_SIZE: f32 = 120.;
const COUNTER_LINE_H: f32 = 132.;
/// Breite einer Ziffer: JetBrains Mono, 600/1000 em.
const COUNTER_DIGIT_W: f32 = COUNTER_SIZE * 0.6;
const BOOT_CHARS_PER_SEC: f64 = 420.;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Boot,
    Main,
    Upgrades,
    Settings,
    Admin,
    Help,
}

/// Ein aufgeploptes Event, das auf einen Klick wartet.
#[derive(Debug, Clone, Copy)]
struct EventPopup {
    kind: EventKind,
    /// Position als Anteil (0…1) der Hauptfläche.
    x: f32,
    y: f32,
    spawned: Instant,
}

pub struct ClickerApp {
    focus: FocusHandle,
    /// Admin-Panel verfügbar? Nur per Startparameter, sonst komplett unsichtbar.
    admin: bool,
    /// Hilfe-Seite wird gerade als Intro (erster Start) gezeigt.
    intro: bool,
    screen: Screen,
    game: Game,
    settings: Settings,
    appearance: WindowAppearance,
    window_active: bool,
    event_popup: Option<EventPopup>,
    next_event_at: Instant,
    log: Log,
    anim: Animator,
    counter: CounterRoll,
    epoch: Instant,
    boot_start: Instant,
    boot_lines: Vec<String>,
    pressed_until: Option<Instant>,
    upgrade_sel: usize,
    /// Pro Upgrade: seit dem letzten Paket verdiente Bytes und vergangene Zeit.
    packet_acc: Vec<f64>,
    packet_timer: Vec<f64>,
    upgrade_scroll: ScrollHandle,
    settings_sel: usize,
    admin_sel: usize,
    /// Index in `TIME_SCALES`.
    time_warp: usize,
    /// Reset einmal ausgelöst, wartet bis dahin auf Bestätigung.
    reset_armed: Option<Instant>,
    bg_time: f32,
    last_save: Instant,
    last_sig: (u64, u64, bool, u64),
    _subscriptions: Vec<Subscription>,
    _tasks: Vec<Task<()>>,
}

impl ClickerApp {
    /// `admin`: Admin-Panel freigeschaltet (nur mit `--admin` bzw. `--debug` gestartet).
    pub fn new(window: &mut Window, cx: &mut Context<Self>, admin: bool) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus);

        let loaded = save::load_game();
        let game = loaded.as_ref().map(Game::from_save).unwrap_or_default();
        let settings = save::load_settings();
        let boot_lines = boot_lines(&game, loaded.is_some(), settings.number_format);

        let subscriptions = vec![
            // Theme folgt dem System ohne Neustart.
            cx.observe_window_appearance(window, |this, window, cx| {
                this.appearance = window.appearance();
                cx.notify();
            }),
            // Hintergrund pausiert, wenn das Fenster nicht aktiv ist.
            cx.observe_window_activation(window, |this, window, cx| {
                this.window_active = window.is_window_active();
                cx.notify();
            }),
            cx.on_app_quit(|this, _| {
                this.persist();
                async {}
            }),
        ];

        let tick = cx.spawn_in(window, async move |this, cx| {
            let mut last = Instant::now();
            loop {
                cx.background_executor().timer(TICK).await;
                let now = Instant::now();
                let dt = (now - last).as_secs_f64();
                last = now;
                if this.update(cx, |this, cx| this.on_tick(dt, cx)).is_err() {
                    break;
                }
            }
        });

        let bg = cx.spawn_in(window, async move |this, cx| {
            let mut last = Instant::now();
            loop {
                cx.background_executor().timer(BG_FRAME).await;
                let now = Instant::now();
                let dt = (now - last).as_secs_f32();
                last = now;
                let alive = this.update(cx, |this, cx| {
                    if this.settings.ascii_background && this.window_active {
                        this.bg_time += dt * SPEEDS[(this.settings.speed as usize).min(2)];
                        cx.notify();
                    }
                });
                if alive.is_err() {
                    break;
                }
            }
        });

        let mut log = Log::default();
        log.push("> terminal-clicker v0.1.0");
        if admin {
            log.push("! admin-modus aktiv (--admin)");
        }
        if loaded.is_some() {
            log.push(format!("  spielstand geladen: {}", format::bytes_padded(game.whole_bytes(), settings.number_format)));
        }

        let now = Instant::now();
        let mut counter = CounterRoll::new(COUNTER_ROLL_MS);
        let start_bytes = game.whole_bytes();
        counter.update(start_bytes, &format::counter(start_bytes, settings.number_format), false, now);
        Self {
            focus,
            admin,
            intro: false,
            screen: Screen::Boot,
            game,
            settings,
            appearance: window.appearance(),
            window_active: true,
            event_popup: None,
            next_event_at: now + Duration::from_secs(30),
            log,
            anim: Animator::new(),
            counter,
            epoch: now,
            boot_start: now,
            boot_lines,
            pressed_until: None,
            upgrade_sel: 0,
            packet_acc: vec![0.; UPGRADES.len()],
            packet_timer: vec![0.; UPGRADES.len()],
            upgrade_scroll: ScrollHandle::new(),
            settings_sel: 0,
            admin_sel: 0,
            time_warp: 0,
            reset_armed: None,
            bg_time: 0.,
            last_save: now,
            last_sig: (0, 0, false, 0),
            _subscriptions: subscriptions,
            _tasks: vec![tick, bg],
        }
    }

    fn theme(&self) -> Theme {
        theme::resolve(&self.settings.theme, self.appearance)
    }

    fn game_time(&self) -> f64 {
        self.epoch.elapsed().as_secs_f64()
    }

    fn cursor_on(&self) -> bool {
        (self.epoch.elapsed().as_millis() / CURSOR_BLINK_MS).is_multiple_of(2)
    }

    fn persist(&mut self) {
        if let Err(e) = save::save_game(&self.game.to_save(save::now_unix())) {
            self.log.push(format!("! speichern fehlgeschlagen: {e}"));
        }
        self.last_save = Instant::now();
    }

    fn persist_settings(&mut self) {
        if let Err(e) = save::save_settings(&self.settings) {
            self.log.push(format!("! settings nicht gespeichert: {e}"));
        }
    }

    // ── Loop ──────────────────────────────────────────────────────────────

    fn on_tick(&mut self, dt: f64, cx: &mut Context<Self>) {
        if self.screen != Screen::Boot {
            let mut events = Vec::new();
            let now = self.game_time();
            // Zeitraffer (Admin) beschleunigt nur das passive Einkommen.
            let scaled = dt * TIME_SCALES[self.time_warp] as f64;
            self.game.played_seconds += dt;
            self.game.tick(scaled, now, &mut events);
            self.handle_events(events);
            if self.emit_packets(scaled) {
                cx.notify();
            }
            // Popup-Countdown und laufende Boni brauchen regelmäßiges Neuzeichnen.
            if self.update_events() || !self.game.buffs().is_empty() {
                cx.notify();
            }
            if self.reset_armed.is_some_and(|t| t.elapsed() > RESET_CONFIRM) {
                self.reset_armed = None;
                cx.notify();
            }

            if self.last_save.elapsed() >= AUTOSAVE {
                self.persist();
                self.log.push("  autosave ok");
            }
        }

        // Nur neu zeichnen, wenn sich sichtbar etwas geändert hat.
        let sig = (
            self.game.whole_bytes(),
            (self.game.clicks_per_second(self.game_time()) * 10.) as u64,
            self.cursor_on(),
            self.log.pushed(),
        );
        if sig != self.last_sig {
            self.last_sig = sig;
            cx.notify();
        }
    }

    /// Macht passives Einkommen sichtbar: jedes aktive Upgrade schickt in
    /// seinem eigenen Takt ein Datenpaket mit dem seitdem verdienten Betrag.
    /// Mehr Level = schnellerer Takt, aber nie mehr als `MAX_PACKETS` zugleich –
    /// so bleibt es auch bei sehr hohem Einkommen übersichtlich.
    fn emit_packets(&mut self, dt: f64) -> bool {
        let visible = self.settings.animations && self.screen == Screen::Main;
        let now = Instant::now();
        let mut spawned = false;
        for i in 0..UPGRADES.len() {
            let rate = self.game.source_rate(i);
            if rate <= 0. {
                continue;
            }
            self.packet_acc[i] += rate * dt;
            self.packet_timer[i] += dt;
            let level = self.game.level(i) as f64;
            let interval = (2.4 / (1. + level).sqrt()).max(0.6);
            if self.packet_timer[i] < interval {
                continue;
            }
            if !visible {
                // Nicht sichtbar: nichts aufstauen, sonst käme später ein Riesenpaket.
                self.packet_acc[i] = 0.;
                self.packet_timer[i] = 0.;
                continue;
            }
            let active = self
                .anim
                .iter()
                .filter(|a| matches!(a.kind, AnimKind::Packet { .. }))
                .count();
            if active >= MAX_PACKETS || self.packet_acc[i] < 1. {
                continue; // weiter sammeln, kommt im nächsten Paket mit
            }
            let value = self.packet_acc[i].floor() as u64;
            self.packet_acc[i] -= value as f64;
            self.packet_timer[i] = 0.;

            // Links oder rechts vom Zähler auftauchen, nie mitten drauf.
            let rng = &mut self.anim.rng;
            let x = if rng.range(0., 1.) < 0.5 {
                rng.range(0.03, 0.18)
            } else {
                rng.range(0.72, 0.86)
            };
            let y = rng.range(0.12, 0.9);
            let id = self.anim.next_id();
            self.anim.play(
                AnimKind::Packet { id, source: i, x, y },
                vec![format!("+{}", format::bytes(value, self.settings.number_format))],
                PACKET_MS,
                now,
            );
            spawned = true;
        }
        spawned
    }

    // ── Events ────────────────────────────────────────────────────────────

    fn schedule_next_event(&mut self, gap: (f32, f32)) {
        let secs = self.anim.rng.range(gap.0, gap.1);
        self.next_event_at = Instant::now() + Duration::from_secs_f32(secs);
    }

    /// Lässt Events auftauchen und wieder verschwinden. `true` = neu zeichnen.
    fn update_events(&mut self) -> bool {
        let now = Instant::now();
        if let Some(popup) = self.event_popup {
            if now.saturating_duration_since(popup.spawned) > EVENT_LIFETIME {
                self.event_popup = None;
                self.log.push(format!("  verpasst: {}", popup.kind.title().to_lowercase()));
                self.schedule_next_event(EVENT_GAP);
            }
            return true;
        }
        // Nur in der Hauptansicht und bei aktivem Fenster – sonst verpasst man es eh.
        if self.screen == Screen::Main && self.window_active && now >= self.next_event_at {
            self.spawn_event(None);
            return true;
        }
        false
    }

    /// Ein Event aufploppen lassen (`None` = zufällige Art).
    fn spawn_event(&mut self, kind: Option<EventKind>) {
        let rng = &mut self.anim.rng;
        let kind = kind.unwrap_or_else(|| match rng.range(0., 1.) {
            r if r < 0.5 => EventKind::Boost,
            r if r < 0.75 => EventKind::Frenzy,
            _ => EventKind::Windfall,
        });
        // In einer der vier Ecken, damit es nie über Zähler oder Button liegt.
        let x = if rng.range(0., 1.) < 0.5 { rng.range(0.03, 0.12) } else { rng.range(0.6, 0.68) };
        let y = if rng.range(0., 1.) < 0.5 { rng.range(0.04, 0.14) } else { rng.range(0.78, 0.84) };
        self.event_popup = Some(EventPopup { kind, x, y, spawned: Instant::now() });
        self.log.push(format!("! event: {}", kind.title().to_lowercase()));
    }

    pub(crate) fn collect_event(&mut self, cx: &mut Context<Self>) {
        let Some(popup) = self.event_popup.take() else {
            return;
        };
        let mut events = Vec::new();
        match self.game.trigger_event(popup.kind, &mut events) {
            EventOutcome::Buff { kind, seconds } => self.log.push(format!(
                "> {} aktiv ({} s)",
                kind.title().to_lowercase(),
                seconds as u32
            )),
            EventOutcome::Bytes(n) => self.log.push(format!(
                "> datenfund: +{}",
                format::bytes(n, self.settings.number_format)
            )),
        }
        self.handle_events(events);
        self.schedule_next_event(EVENT_GAP);
        cx.notify();
    }

    fn on_collect_event(&mut self, _: &CollectEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.collect_event(cx);
    }

    fn handle_events(&mut self, events: Vec<GameEvent>) {
        let now = Instant::now();
        for event in events {
            match event {
                GameEvent::Milestone(m) => {
                    self.log.push(format!("> meilenstein: {}", format::bytes(m, self.settings.number_format)));
                    if self.settings.animations {
                        let width = format::counter(self.game.whole_bytes(), self.settings.number_format).chars().count();
                        let frames = scramble_frames(&mut self.anim.rng, width, 6);
                        self.anim.play(AnimKind::Scramble, frames, 50, now);
                    }
                }
                GameEvent::Unlocked(id) => {
                    self.log.push(format!("> neu verfügbar: {id}"));
                }
                GameEvent::BuffEnded(kind) => {
                    self.log.push(format!("  {} vorbei", kind.title().to_lowercase()));
                }
                GameEvent::Bought { id, level } => {
                    self.log.push(format!("> installiert {id}"));
                    self.log.push(format!("  level {level:03}"));
                    if self.settings.animations
                        && let Some(row) = UPGRADES.iter().position(|d| d.id == id)
                    {
                        let frames = ["on", "", "on", ""].map(String::from).to_vec();
                        self.anim.play(AnimKind::RowBlink(row), frames, 70, now);
                    }
                }
            }
        }
    }

    // ── Aktionen ──────────────────────────────────────────────────────────

    fn click(&mut self, cx: &mut Context<Self>) {
        let now = Instant::now();
        let mut events = Vec::new();
        let gain = self.game.click(self.game_time(), &mut events);

        // Gedrückt-Zustand ca. 80 ms invertiert, danach neu zeichnen.
        self.pressed_until = Some(now + PRESS);
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(PRESS).await;
            this.update(cx, |_, cx| cx.notify()).ok();
        })
        .detach();

        if self.settings.animations {
            let id = self.anim.next_id();
            let x = self.anim.rng.range(-60., 60.);
            let label = format!("+{gain}");
            self.anim.play(
                AnimKind::Float { id, x },
                vec![label.clone(), label, "·".into()],
                130,
                now,
            );
            self.anim.play(
                AnimKind::ButtonPulse,
                vec!["[[ KLICK ]]".into(), "[[ KLICK ]]".into()],
                60,
                now,
            );
        }

        self.handle_events(events);
        cx.notify();
    }

    fn on_click(&mut self, _: &Click, _: &mut Window, cx: &mut Context<Self>) {
        match self.screen {
            Screen::Main => self.click(cx),
            Screen::Settings => self.change_setting(1, cx),
            Screen::Upgrades => self.buy_selected(cx),
            Screen::Admin => self.run_admin(1, cx),
            Screen::Help | Screen::Boot => {}
        }
    }

    fn on_quit(&mut self, _: &Quit, _: &mut Window, cx: &mut Context<Self>) {
        self.persist();
        cx.quit();
    }

    fn set_screen(&mut self, screen: Screen, cx: &mut Context<Self>) {
        if screen == Screen::Admin && !self.admin {
            return;
        }
        // Wer die Hilfe einmal verlassen hat, bekommt kein Intro mehr.
        if self.screen == Screen::Help && screen != Screen::Help && !self.settings.intro_seen {
            self.settings.intro_seen = true;
            self.persist_settings();
        }
        if screen != Screen::Help {
            self.intro = false;
        }
        if self.screen != Screen::Boot || screen == Screen::Main {
            self.screen = screen;
            cx.notify();
        }
    }

    fn on_upgrades(&mut self, _: &OpenUpgrades, _: &mut Window, cx: &mut Context<Self>) {
        self.set_screen(Screen::Upgrades, cx);
    }

    fn on_settings(&mut self, _: &OpenSettings, _: &mut Window, cx: &mut Context<Self>) {
        self.set_screen(Screen::Settings, cx);
    }

    fn on_admin(&mut self, _: &OpenAdmin, _: &mut Window, cx: &mut Context<Self>) {
        self.set_screen(Screen::Admin, cx);
    }

    fn on_help(&mut self, _: &OpenHelp, _: &mut Window, cx: &mut Context<Self>) {
        self.set_screen(Screen::Help, cx);
    }

    fn on_back(&mut self, _: &Back, _: &mut Window, cx: &mut Context<Self>) {
        if matches!(
            self.screen,
            Screen::Upgrades | Screen::Settings | Screen::Admin | Screen::Help
        ) {
            self.set_screen(Screen::Main, cx);
        }
    }

    fn move_sel(&mut self, delta: i32, cx: &mut Context<Self>) {
        let (sel, len) = match self.screen {
            Screen::Upgrades => (&mut self.upgrade_sel, UPGRADES.len()),
            Screen::Settings => (&mut self.settings_sel, settings_ui::ITEMS.len()),
            Screen::Admin => (&mut self.admin_sel, admin_ui::ITEMS.len()),
            _ => return,
        };
        self.reset_armed = None;
        *sel = (*sel as i32 + delta).rem_euclid(len as i32) as usize;
        if self.screen == Screen::Upgrades {
            self.upgrade_scroll.scroll_to_item(self.upgrade_sel);
        }
        cx.notify();
    }

    fn on_up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        self.move_sel(-1, cx);
    }

    fn on_down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        self.move_sel(1, cx);
    }

    fn on_left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        match self.screen {
            Screen::Settings => self.change_setting(-1, cx),
            Screen::Admin if admin_ui::ITEMS[self.admin_sel] == admin_ui::Item::TimeWarp => {
                self.run_admin(-1, cx)
            }
            _ => {}
        }
    }

    fn on_right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        match self.screen {
            Screen::Settings => self.change_setting(1, cx),
            Screen::Admin if admin_ui::ITEMS[self.admin_sel] == admin_ui::Item::TimeWarp => {
                self.run_admin(1, cx)
            }
            _ => {}
        }
    }

    fn on_confirm(&mut self, _: &Confirm, _: &mut Window, cx: &mut Context<Self>) {
        match self.screen {
            Screen::Boot => self.start(cx),
            Screen::Upgrades => self.buy_selected(cx),
            Screen::Settings => self.change_setting(1, cx),
            Screen::Admin => self.run_admin(1, cx),
            Screen::Help => self.set_screen(Screen::Main, cx),
            Screen::Main => {}
        }
    }

    /// Führt den gewählten Admin-Eintrag aus. `delta` < 0 = Rechtsklick/←.
    fn run_admin(&mut self, delta: i32, cx: &mut Context<Self>) {
        use admin_ui::Item;
        let item = admin_ui::ITEMS[self.admin_sel];
        if item != Item::Reset {
            self.reset_armed = None;
        }
        let mut events = Vec::new();
        match item {
            Item::Grant(n) if delta > 0 => {
                self.game.admin_grant(n as f64, &mut events);
                self.log.push(format!("> admin: +{} B", fmt_num(n)));
            }
            Item::Times10 if delta > 0 => {
                self.game.admin_grant(self.game.bytes * 9., &mut events);
                self.log.push("> admin: bytes ×10");
            }
            Item::UnlockAll if delta > 0 => {
                self.game.admin_unlock_all(&mut events);
                self.log.push("> admin: alles freigeschaltet");
            }
            Item::LevelAll if delta > 0 => {
                self.game.admin_level_all();
                self.log.push("> admin: +1 level auf alles");
            }
            Item::SpawnEvent if delta > 0 => {
                self.log.push("> admin: event ausgelöst (in main)");
                self.spawn_event(None);
            }
            Item::TimeWarp => {
                let len = TIME_SCALES.len() as i32;
                self.time_warp = (self.time_warp as i32 + delta.signum()).rem_euclid(len) as usize;
                self.log.push(format!("> admin: zeitraffer ×{}", TIME_SCALES[self.time_warp]));
            }
            Item::Reset if delta > 0 => {
                if self.reset_armed.take().is_some() {
                    self.game.admin_reset();
                    self.packet_acc.iter_mut().for_each(|v| *v = 0.);
                    self.packet_timer.iter_mut().for_each(|v| *v = 0.);
                    self.anim.clear();
                    self.log.push("> admin: spielstand zurückgesetzt");
                } else {
                    self.reset_armed = Some(Instant::now());
                    self.log.push("! nochmal enter zum zurücksetzen");
                }
            }
            _ => {}
        }
        self.handle_events(events);
        if item != Item::TimeWarp {
            self.persist();
        }
        cx.notify();
    }

    pub(crate) fn hover_admin(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.admin_sel != index {
            self.admin_sel = index;
            self.reset_armed = None;
            cx.notify();
        }
    }

    pub(crate) fn click_admin(&mut self, index: usize, delta: i32, cx: &mut Context<Self>) {
        self.admin_sel = index;
        self.run_admin(delta, cx);
    }

    fn start(&mut self, cx: &mut Context<Self>) {
        if self.screen == Screen::Boot {
            self.log.push("> boot ok. viel spaß.");
            self.set_screen(Screen::Main, cx);
            self.schedule_next_event(FIRST_EVENT_GAP);
            // Allererster Start: erst die Hilfe als kleines Intro.
            if !self.settings.intro_seen {
                self.set_screen(Screen::Help, cx);
                self.intro = true;
            }
        }
    }

    // ── Maus ──────────────────────────────────────────────────────────────
    // Maus und Tastatur teilen sich dieselbe Auswahl und dieselben Funktionen.

    pub(crate) fn hover_upgrade(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.upgrade_sel != index {
            self.upgrade_sel = index;
            cx.notify();
        }
    }

    pub(crate) fn click_upgrade(&mut self, index: usize, cx: &mut Context<Self>) {
        self.upgrade_sel = index;
        self.buy_selected(cx);
    }

    pub(crate) fn hover_setting(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.settings_sel != index {
            self.settings_sel = index;
            cx.notify();
        }
    }

    pub(crate) fn click_setting(&mut self, index: usize, delta: i32, cx: &mut Context<Self>) {
        self.settings_sel = index;
        self.change_setting(delta, cx);
    }

    pub(crate) fn pick_theme(&mut self, id: &str, cx: &mut Context<Self>) {
        self.settings.theme = id.to_string();
        self.persist_settings();
        cx.notify();
    }

    fn buy_selected(&mut self, cx: &mut Context<Self>) {
        let mut events = Vec::new();
        match self.game.buy(self.upgrade_sel, &mut events) {
            Ok(_) => self.persist(),
            Err(BuyError::Locked) => self.log.push("! gesperrt"),
            Err(BuyError::TooExpensive { cost }) => {
                self.log.push(format!("! zu teuer: {} nötig", format::bytes_padded(cost, self.settings.number_format)))
            }
            Err(BuyError::Unknown) => {}
        }
        self.handle_events(events);
        cx.notify();
    }

    fn change_setting(&mut self, delta: i32, cx: &mut Context<Self>) {
        let item = settings_ui::ITEMS[self.settings_sel];
        settings_ui::change(&mut self.settings, item, delta);
        if !self.settings.animations {
            self.anim.clear();
        }
        self.persist_settings();
        cx.notify();
    }

    // ── Rendering ─────────────────────────────────────────────────────────

    /// Eigene Titelleiste. Die macOS-Ampel bleibt nativ (siehe `TitlebarOptions`
    /// in main.rs) und sitzt links in der dafür freigehaltenen Fläche.
    fn render_title_bar(&self, window: &Window, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let view = match self.screen {
            Screen::Boot => "boot",
            Screen::Main => "main",
            Screen::Upgrades => "upgrades",
            Screen::Settings => "settings",
            Screen::Admin => "admin",
            Screen::Help => "hilfe",
        };
        // Inaktives Fenster: Titelleiste gedimmt, wie bei macOS üblich.
        let active = self.window_active;
        let text = if active { theme.fg } else { theme.dim };
        let hover_bg = theme.line;

        // Fenstergröße in Zeichenzellen, wie im Terminal-Titel (z. B. 152×40).
        let size = window.viewport_size();
        let cols = (f32::from(size.width) / CELL_W) as u32;
        let rows = (f32::from(size.height) / LINE_H) as u32;

        // Reiter zum Navigieren per Maus; auf dem Boot-Screen ausgeblendet.
        let tabs = [
            ("main", Screen::Main),
            ("upgrades", Screen::Upgrades),
            ("settings", Screen::Settings),
            ("hilfe", Screen::Help),
            ("admin", Screen::Admin),
        ];
        let admin = self.admin;
        let tabs = tabs.into_iter().filter(move |(_, s)| admin || *s != Screen::Admin);
        let tab_bar = div().flex().h_full().children(tabs.map(|(name, screen)| {
            let selected = self.screen == screen;
            div()
                .px(px(12.))
                .h_full()
                .flex()
                .items_center()
                .cursor_pointer()
                .border_l_1()
                .border_color(theme.line)
                .when(selected, |d| d.bg(text).text_color(theme.bg))
                .when(!selected, |d| d.text_color(theme.dim).hover(move |s| s.bg(hover_bg)))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        // Kein Doppelklick-Zoom, wenn man schnell zwischen Reitern wechselt.
                        cx.stop_propagation();
                        this.set_screen(screen, cx);
                    }),
                )
                .child(name)
        }));

        // Im Vollbild blendet macOS die Ampel aus – dann keinen Platz freihalten.
        let lights_space = if window.is_fullscreen() { 0. } else { TRAFFIC_LIGHTS_W };

        div()
            .flex_none()
            .h(px(TITLE_BAR_H))
            .relative()
            .flex()
            .items_center()
            .justify_between()
            .bg(theme.bg)
            .text_color(text)
            .border_b_1()
            .border_color(theme.line)
            // Ziehen verschiebt das Fenster; Doppelklick verhält sich wie in
            // macOS eingestellt (zoomen/minimieren).
            .on_mouse_down(MouseButton::Left, |event, window, cx| {
                if event.click_count == 2 {
                    window.titlebar_double_click();
                } else {
                    begin_window_drag(window, cx);
                }
            })
            // Titel mittig über der ganzen Breite, wie bei Terminal.app.
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.dim)
                    .child(
                        div()
                            .flex()
                            .gap(px(8.))
                            .child(div().text_color(text).child("terminal-clicker"))
                            .child(format!("— {view} — {cols}×{rows}")),
                    ),
            )
            .child(
                div()
                    .h_full()
                    .flex()
                    .items_center()
                    .child(div().w(px(lights_space)).flex_none())
                    .when(self.screen != Screen::Boot, |d| d.child(tab_bar))
                    .child(div().h_full().border_l_1().border_color(theme.line)),
            )
            .child(
                div()
                    .h_full()
                    .px(px(14.))
                    .flex()
                    .items_center()
                    .text_color(theme.dim)
                    .child("tty0"),
            )
    }

    fn render_counter(&self, now: Instant, theme: &Theme) -> Div {
        // Meilenstein-Scramble hat Vorrang vor dem Rollen.
        let scramble = self
            .anim
            .find(AnimKind::Scramble)
            .and_then(|a| a.frame(now))
            .map(str::to_string);

        let number = match scramble {
            Some(text) => div().child(text),
            None => div().flex().children(self.counter.digits().iter().map(|d| {
                let cell = div()
                    .relative()
                    .w(px(COUNTER_DIGIT_W))
                    .h(px(COUNTER_LINE_H))
                    .overflow_hidden();
                let glyph = |c: char, top: f32| {
                    div()
                        .absolute()
                        .left_0()
                        .top(px(top))
                        .w_full()
                        .child(c.to_string())
                };
                match self.counter.progress(d, now) {
                    // Alte Ziffer rollt raus, neue rein – wie ein Zählwerk.
                    Some(p) => {
                        let h = COUNTER_LINE_H;
                        let (old_top, new_top) = match d.dir {
                            RollDir::Up => (-h * p, h * (1. - p)),
                            RollDir::Down => (h * p, -h * (1. - p)),
                        };
                        cell.child(glyph(d.prev, old_top).opacity(1. - p))
                            .child(glyph(d.cur, new_top))
                    }
                    None => cell.child(glyph(d.cur, 0.)),
                }
            })),
        };

        div()
            .flex()
            .flex_col()
            .items_center()
            .child(div().text_color(theme.dim).child("BYTES"))
            .child(
                number
                    .text_size(px(COUNTER_SIZE))
                    .line_height(px(COUNTER_LINE_H))
                    .whitespace_nowrap(),
            )
            .child(
                div()
                    .text_color(theme.dim)
                    .child(format!("{}/klick", format::bytes(self.game.per_click(), self.settings.number_format))),
            )
    }

    fn render_button(&self, now: Instant, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let pressed = self.pressed_until.is_some_and(|t| now < t);
        let label = self
            .anim
            .find(AnimKind::ButtonPulse)
            .and_then(|a| a.frame(now))
            .unwrap_or("[ KLICK ]")
            .to_string();
        let (fg, bg) = if pressed { (theme.bg, theme.fg) } else { (theme.fg, theme.bg) };

        let floats = self.anim.iter().filter_map(|a| match a.kind {
            AnimKind::Float { x, .. } => {
                let frame = a.frame(now)?.to_string();
                let p = a.progress(now);
                Some(
                    div()
                        .absolute()
                        .top(px(-22. - p * 46.))
                        .left(px(108. + x))
                        .w(px(44.))
                        .flex()
                        .justify_center()
                        .opacity(1. - p * 0.7)
                        .child(frame),
                )
            }
            _ => None,
        });

        div()
            .relative()
            .mt(px(48.))
            .child(
                div()
                    .id("klick")
                    .w(px(260.))
                    .h(px(56.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_2()
                    .border_color(theme.fg)
                    .bg(bg)
                    .text_color(fg)
                    .text_size(px(18.))
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            if this.screen == Screen::Main {
                                this.click(cx);
                            }
                        }),
                    )
                    .child(label),
            )
            .children(floats)
    }

    /// Datenpakete der passiven Upgrades: tauchen aus ASCII-Rauschen auf,
    /// steigen langsam und verblassen.
    fn render_packets(&self, window: &Window, now: Instant, theme: &Theme) -> Vec<Div> {
        let size = window.viewport_size();
        let w = f32::from(size.width) - LOG_PANEL_W;
        let h = f32::from(size.height) - TITLE_BAR_H - STATUS_BAR_H;
        self.anim
            .iter()
            .filter_map(|a| {
                let AnimKind::Packet { source, x, y, .. } = a.kind else {
                    return None;
                };
                let p = a.progress(now);
                let decode = (p / 0.22).min(1.);
                let tick = (now.saturating_duration_since(a.start).as_millis() / 40) as u64;
                let opacity = if p < 0.6 { 1. } else { (1. - p) / 0.4 };
                let value = decode_text(a.frames.first()?, decode, tick);
                let name = decode_text(UPGRADES[source].id, decode, tick + 7);
                Some(
                    div()
                        .absolute()
                        .left(px(x * w))
                        .top(px(y * h - p * 40.))
                        .flex()
                        .gap(px(6.))
                        .text_size(px(12.))
                        .whitespace_nowrap()
                        .opacity(opacity)
                        .child(div().text_color(theme.fg).child(value))
                        .child(div().text_color(theme.dim).child(name)),
                )
            })
            .collect()
    }

    fn render_main(&self, window: &Window, now: Instant, theme: &Theme, cx: &mut Context<Self>) -> Div {
        div()
            .flex_1()
            .relative()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            // Pakete zuerst, damit sie hinter Zähler und Button liegen.
            .children(self.render_packets(window, now, theme))
            .child(self.render_counter(now, theme))
            .child(self.render_button(now, theme, cx))
            .child(
                div()
                    .mt(px(14.))
                    .text_color(theme.dim)
                    .child("space oder klick"),
            )
            .child(self.render_buffs(theme))
            // Event zuletzt, damit es über allem liegt und anklickbar ist.
            .children(self.render_event(window, now, theme, cx))
    }

    /// Laufende Boni unter dem Button: Name, Restzeit-Balken, Sekunden.
    fn render_buffs(&self, theme: &Theme) -> Div {
        div()
            .mt(px(18.))
            .h(px(44.))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(2.))
            .children(self.game.buffs().iter().map(|b| {
                div()
                    .flex()
                    .gap(px(10.))
                    .child(b.kind.title())
                    .child(
                        div()
                            .text_color(theme.dim)
                            .child(bar(b.remaining / b.duration, 16)),
                    )
                    .child(format!("{:>2} s", b.remaining.ceil() as u32))
            }))
    }

    /// Das Event-Popup: Kasten mit blinkender Kopfzeile, Klick oder `e` sammelt ein.
    fn render_event(
        &self,
        window: &Window,
        now: Instant,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Option<Div> {
        let popup = self.event_popup?;
        let size = window.viewport_size();
        let w = f32::from(size.width) - LOG_PANEL_W;
        let h = f32::from(size.height) - TITLE_BAR_H - STATUS_BAR_H;
        let age = now.saturating_duration_since(popup.spawned);
        let left = EVENT_LIFETIME.saturating_sub(age).as_secs_f32().ceil() as u32;
        let animate = self.settings.animations;
        let blink = animate && (age.as_millis() / 450).is_multiple_of(2);
        let tick = (age.as_millis() / 40) as u64;
        let decode = if animate { age.as_secs_f32() / 0.35 } else { 1. };
        let title = decode_text(&format!("! {}", popup.kind.title()), decode, tick);
        let (head_fg, head_bg) = if blink { (theme.bg, theme.fg) } else { (theme.fg, theme.bg) };

        Some(
            div()
                .absolute()
                .left(px(popup.x * w))
                .top(px(popup.y * h))
                .w(px(EVENT_W))
                .flex()
                .flex_col()
                .bg(theme.bg)
                .border_2()
                .border_color(theme.fg)
                .cursor_pointer()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        cx.stop_propagation();
                        this.collect_event(cx);
                    }),
                )
                .child(
                    div()
                        .px(px(10.))
                        .py(px(4.))
                        .bg(head_bg)
                        .text_color(head_fg)
                        .whitespace_nowrap()
                        .child(title),
                )
                .child(
                    div()
                        .px(px(10.))
                        .pt(px(6.))
                        .text_size(px(12.))
                        .child(popup.kind.description()),
                )
                .child(
                    div()
                        .px(px(10.))
                        .pb(px(6.))
                        .text_size(px(12.))
                        .text_color(theme.dim)
                        .child(format!("klick oder [e] · noch {left} s")),
                ),
        )
    }

    fn render_boot(&self, theme: &Theme, cx: &mut Context<Self>) -> (Div, bool) {
        let total: usize = self.boot_lines.iter().map(|l| l.chars().count()).sum();
        let budget = (self.boot_start.elapsed().as_secs_f64() * BOOT_CHARS_PER_SEC) as usize;
        let revealed = if self.settings.animations { budget.min(total) } else { total };
        let done = revealed >= total;

        // Zeilen tippen sich Zeichen für Zeichen ein.
        let mut left = revealed;
        let mut lines = Vec::new();
        for line in &self.boot_lines {
            let n = line.chars().count();
            let take = n.min(left);
            lines.push(line.chars().take(take).collect::<String>());
            left -= take;
            if take < n {
                break;
            }
        }
        let cursor = if !done || self.cursor_on() { "_" } else { " " };
        let line_count = lines.len();

        let body = div()
            .flex_1()
            .p(px(28.))
            .flex()
            .flex_col()
            .bg(theme.bg)
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| this.start(cx)))
            .children(lines.into_iter().enumerate().map(|(i, l)| {
                let last = i + 1 == line_count;
                let dim = l.starts_with('[');
                div()
                    .flex()
                    .whitespace_nowrap()
                    .when(dim, |d| d.text_color(theme.dim))
                    .child(l)
                    .when(last, |d| d.child(div().text_color(theme.fg).child(cursor)))
            }));
        (body, !done)
    }

    fn render_background(&self, window: &Window, theme: &Theme) -> Div {
        let size = window.viewport_size();
        let cols = (f32::from(size.width) / CELL_W).ceil() as usize + 1;
        let rows = (f32::from(size.height) / BG_LINE_H).ceil() as usize + 1;
        let lines = render_rows(cols, rows, self.bg_time, self.settings.density);

        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .overflow_hidden()
            .flex()
            .flex_col()
            .text_color(theme.dim.opacity(0.2))
            .children(lines.into_iter().map(|line| {
                div()
                    .h(px(BG_LINE_H))
                    .line_height(px(BG_LINE_H))
                    .whitespace_nowrap()
                    .child(line)
            }))
    }

    fn status(&self) -> (String, Vec<KeyHint>) {
        let g = &self.game;
        let f = self.settings.number_format;
        let stats = format!(
            "{} · {}/klick · {:.1} klicks/s · +{}",
            format::bytes_padded(g.whole_bytes(), f),
            format::bytes(g.per_click(), f),
            g.clicks_per_second(self.game_time()),
            format::rate(g.per_second(), f),
        );
        let quit = || KeyHint::new("q", "beenden", Quit);
        match self.screen {
            Screen::Boot => (
                "boot".into(),
                vec![KeyHint::new("enter", "starten", Confirm), quit()],
            ),
            Screen::Main => (
                stats,
                vec![
                    KeyHint::new("space", "klick", Click),
                    KeyHint::new("u", "upgrades", OpenUpgrades),
                    KeyHint::new("s", "settings", OpenSettings),
                    KeyHint::new("h", "hilfe", OpenHelp),
                ]
                .into_iter()
                .chain(self.event_popup.is_some().then(|| KeyHint::new("e", "event!", CollectEvent)))
                .chain(self.admin.then(|| KeyHint::new("a", "admin", OpenAdmin)))
                .chain([quit()])
                .collect(),
            ),
            Screen::Upgrades => (
                stats,
                vec![
                    KeyHint::info("↑↓", "wählen"),
                    KeyHint::new("enter", "kaufen", Confirm),
                    KeyHint::new("s", "settings", OpenSettings),
                    KeyHint::new("esc", "zurück", Back),
                    quit(),
                ],
            ),
            Screen::Settings => (
                stats,
                vec![
                    KeyHint::info("↑↓", "wählen"),
                    KeyHint::new("enter", "umschalten", Confirm),
                    KeyHint::info("←→", "ändern"),
                    KeyHint::new("u", "upgrades", OpenUpgrades),
                    KeyHint::new("esc", "zurück", Back),
                    quit(),
                ],
            ),
            Screen::Help => (
                stats,
                vec![
                    if self.intro {
                        KeyHint::new("enter", "los geht's", Confirm)
                    } else {
                        KeyHint::new("esc", "zurück", Back)
                    },
                    quit(),
                ],
            ),
            Screen::Admin => (
                stats,
                vec![
                    KeyHint::info("↑↓", "wählen"),
                    KeyHint::new("enter", "ausführen", Confirm),
                    KeyHint::info("←→", "zeitraffer"),
                    KeyHint::new("esc", "zurück", Back),
                    quit(),
                ],
            ),
        }
    }
}

impl Render for ClickerApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = Instant::now();
        let theme = self.theme();
        self.anim.prune(now);
        if self.pressed_until.is_some_and(|t| now >= t) {
            self.pressed_until = None;
        }

        let value = self.game.whole_bytes();
        self.counter
            .update(value, &format::counter(value, self.settings.number_format), self.settings.animations, now);

        let mut keep_animating = self.anim.is_active()
            || self.counter.is_active(now)
            || (self.settings.animations && self.log.is_animating(now));

        let body = match self.screen {
            Screen::Boot => {
                let (boot, typing) = self.render_boot(&theme, cx);
                keep_animating |= typing;
                boot
            }
            screen => {
                let content = match screen {
                    Screen::Upgrades => {
                        let blink = self.anim.iter().find_map(|a| match a.kind {
                            AnimKind::RowBlink(row) => {
                                Some((row, a.frame(now).is_some_and(|f| !f.is_empty())))
                            }
                            _ => None,
                        });
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .p(px(24.))
                            .child(render_upgrades(&self.game, self.upgrade_sel, blink, &theme, &self.upgrade_scroll, self.settings.number_format, cx))
                    }
                    Screen::Settings => div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .p(px(24.))
                        .child(render_settings(&self.settings, self.settings_sel, &theme, self.appearance, cx)),
                    Screen::Admin => {
                        let info = AdminInfo {
                            clicks_per_second: self.game.clicks_per_second(self.game_time()),
                            session_seconds: self.epoch.elapsed().as_secs_f64(),
                            packets: self
                                .anim
                                .iter()
                                .filter(|a| matches!(a.kind, AnimKind::Packet { .. }))
                                .count(),
                            time_scale: TIME_SCALES[self.time_warp],
                            reset_armed: self.reset_armed.is_some(),
                        };
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .p(px(24.))
                            .child(render_admin(&self.game, &info, self.admin_sel, &theme, cx))
                    }
                    Screen::Help => div()
                        .flex_1()
                        .min_w(px(0.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .p(px(24.))
                        .child(render_help(&theme, self.intro, self.admin)),
                    _ => self.render_main(window, now, &theme, cx),
                };
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .flex()
                    .child(content)
                    .child(render_log(&self.log, &theme, self.cursor_on(), now, self.settings.animations))
            }
        };

        // Nur solange etwas läuft, wird der nächste Frame angefordert.
        if keep_animating {
            window.request_animation_frame();
        }

        let (stats, keys) = self.status();

        div()
            .id("root")
            .key_context("Clicker")
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::on_click))
            .on_action(cx.listener(Self::on_quit))
            .on_action(cx.listener(Self::on_upgrades))
            .on_action(cx.listener(Self::on_settings))
            .on_action(cx.listener(Self::on_admin))
            .on_action(cx.listener(Self::on_help))
            .on_action(cx.listener(Self::on_collect_event))
            .on_action(cx.listener(Self::on_back))
            .on_action(cx.listener(Self::on_up))
            .on_action(cx.listener(Self::on_down))
            .on_action(cx.listener(Self::on_left))
            .on_action(cx.listener(Self::on_right))
            .on_action(cx.listener(Self::on_confirm))
            .size_full()
            .relative()
            .bg(theme.bg)
            .text_color(theme.fg)
            .font_family(FONT)
            .text_size(px(FONT_SIZE))
            .line_height(px(LINE_H))
            .when(self.settings.ascii_background, |d| {
                d.child(self.render_background(window, &theme))
            })
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .flex()
                    .flex_col()
                    .child(self.render_title_bar(window, &theme, cx))
                    .child(body)
                    .child(render_status_bar(&theme, stats, keys)),
            )
    }
}

fn boot_lines(game: &Game, loaded: bool, fmt: NumberFormat) -> Vec<String> {
    let save = if loaded {
        format!("ok ({})", format::bytes_padded(game.whole_bytes(), fmt))
    } else {
        "neu".to_string()
    };
    let installed: u32 = (0..UPGRADES.len()).map(|i| game.level(i)).sum();
    vec![
        "terminal-clicker v0.1.0 (tty0)".into(),
        String::new(),
        "[ 0.000] bios: 1 finger erkannt".into(),
        "[ 0.012] mem: 640 KB sollten reichen".into(),
        "[ 0.031] mount /dev/klick ............ ok".into(),
        format!("[ 0.047] lade spielstand ............. {save}"),
        "[ 0.063] starte byte-daemon .......... ok".into(),
        format!("[ 0.071] upgrades installiert ........ {installed:03}"),
        String::new(),
        "bereit. [enter] oder klick zum starten ".into(),
    ]
}
