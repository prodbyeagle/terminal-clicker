//! Die App: hält Sitzung und Ansichten, verarbeitet jede `Msg` an einer Stelle
//! und zeichnet den Rahmen (Titelleiste, Inhalt, Log, Statuszeile).

use crate::format;
use crate::game::events::{DirectorNews, EventOutcome};
use crate::game::{Game, GameEvent, SaveData, UPGRADES};
use crate::save::{self, GAME_FILE, SETTINGS_FILE};
use crate::settings::Settings;
use crate::theme;
use crate::ui::admin::AdminScreen;
use crate::ui::backdrop;
use crate::ui::boot::Boot;
use crate::ui::help::HelpScreen;
use crate::ui::keys;
use crate::ui::layout::{FONT, FONT_SIZE, LINE_H};
use crate::ui::log::{Log, render_log};
use crate::ui::main_screen::MainScreen;
use crate::ui::msg::{Emit, Msg, Screen};
use crate::ui::page::Page;
use crate::ui::session::Session;
use crate::ui::settings::SettingsScreen;
use crate::ui::status_bar::{Hint, render_status_bar};
use crate::ui::title_bar;
use crate::ui::upgrades::UpgradesScreen;
use crate::ui::view::View;
use gpui::{
    Context, FocusHandle, Subscription, Task, Window, WindowAppearance, div, prelude::*, px,
};
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(100);
/// ≈ 18 fps reichen für das Wabern des Hintergrunds.
const BG_FRAME: Duration = Duration::from_millis(55);
const AUTOSAVE: Duration = Duration::from_secs(30);
const CURSOR_BLINK_MS: u128 = 530;

pub struct ClickerApp {
    focus: FocusHandle,
    emit: Emit,
    screen: Screen,
    session: Session,
    boot: Boot,
    main: MainScreen,
    upgrades: UpgradesScreen,
    settings: SettingsScreen,
    help: HelpScreen,
    admin: AdminScreen,
    appearance: WindowAppearance,
    window_active: bool,
    bg_time: f32,
    started: Instant,
    last_save: Instant,
    _subscriptions: Vec<Subscription>,
    _tasks: Vec<Task<()>>,
}

impl ClickerApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, admin: bool) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus);

        let mut log = Log::default();
        log.event(format!("terminal-clicker v{}", env!("CARGO_PKG_VERSION")));
        if admin {
            log.warn("admin-modus aktiv (--admin)");
        }
        let settings: Settings = save::load(SETTINGS_FILE)
            .unwrap_or_else(|e| {
                log.warn(e.message);
                None
            })
            .unwrap_or_default();
        let mut save_blocked = false;
        let loaded = save::load::<SaveData>(GAME_FILE).unwrap_or_else(|e| {
            save_blocked = e.still_in_place;
            log.warn(e.message);
            None
        });
        let seed = save::now_unix();
        let (game, offline, happened) = match &loaded {
            Some(data) => Game::resume(data, save::now_unix(), seed),
            None => (Game::seeded(seed), 0., Vec::new()),
        };
        let fmt = settings.number_format;
        if loaded.is_some() {
            log.detail(format!(
                "spielstand geladen: {}",
                format::bytes_padded(game.whole_bytes(), fmt)
            ));
        }
        if offline > 0. {
            log.event(format!(
                "offline verdient: +{}",
                format::bytes(offline, fmt)
            ));
        }

        let boot = Boot::new(&game, loaded.is_some(), fmt);
        let mut session = Session::new(game, settings, log, admin, seed);
        session.happened = happened;
        session.save_blocked = save_blocked;
        let main = MainScreen::new(&session);
        let emit = {
            let app = cx.entity().downgrade();
            Emit::new(move |msg, cx| {
                app.update(cx, |app, cx| app.handle(msg, cx)).ok();
            })
        };

        let subscriptions = vec![
            cx.observe_window_appearance(window, |this, window, cx| {
                this.appearance = window.appearance();
                cx.notify();
            }),
            cx.observe_window_activation(window, |this, window, cx| {
                this.window_active = window.is_window_active();
                cx.notify();
            }),
            cx.on_app_quit(|this, _| {
                this.persist();
                async {}
            }),
        ];
        let tasks = vec![
            cx.spawn_in(window, async move |this, cx| {
                let mut last = Instant::now();
                loop {
                    cx.background_executor().timer(TICK).await;
                    let dt = last.elapsed().as_secs_f64();
                    last = Instant::now();
                    if this.update(cx, |this, cx| this.tick(dt, cx)).is_err() {
                        break;
                    }
                }
            }),
            cx.spawn_in(window, async move |this, cx| {
                let mut last = Instant::now();
                loop {
                    cx.background_executor().timer(BG_FRAME).await;
                    let dt = last.elapsed().as_secs_f32();
                    last = Instant::now();
                    if this
                        .update(cx, |this, cx| this.animate_background(dt, cx))
                        .is_err()
                    {
                        break;
                    }
                }
            }),
        ];

        let now = Instant::now();
        let mut app = Self {
            focus,
            emit,
            screen: Screen::Boot,
            session,
            boot,
            main,
            upgrades: UpgradesScreen::default(),
            settings: SettingsScreen::default(),
            help: HelpScreen,
            admin: AdminScreen::default(),
            appearance: window.appearance(),
            window_active: true,
            bg_time: 0.,
            started: now,
            last_save: now,
            _subscriptions: subscriptions,
            _tasks: tasks,
        };
        app.settle();
        app
    }

    fn page(&self, screen: Screen) -> &dyn Page {
        match screen {
            Screen::Boot => &self.boot,
            Screen::Main => &self.main,
            Screen::Upgrades => &self.upgrades,
            Screen::Settings => &self.settings,
            Screen::Help => &self.help,
            Screen::Admin => &self.admin,
        }
    }

    /// Die Ansicht samt Sitzung, damit beide zugleich veränderbar sind.
    fn page_mut(&mut self, screen: Screen) -> (&mut dyn Page, &mut Session) {
        let page: &mut dyn Page = match screen {
            Screen::Boot => &mut self.boot,
            Screen::Main => &mut self.main,
            Screen::Upgrades => &mut self.upgrades,
            Screen::Settings => &mut self.settings,
            Screen::Help => &mut self.help,
            Screen::Admin => &mut self.admin,
        };
        (page, &mut self.session)
    }

    /// Gibt zurück, ob wirklich gespeichert wurde.
    fn persist(&mut self) -> bool {
        self.last_save = Instant::now();
        if self.session.save_blocked {
            return false;
        }
        let data = self.session.game.to_save(save::now_unix());
        match save::store(GAME_FILE, &data) {
            Ok(()) => true,
            Err(e) => {
                self.session
                    .log
                    .warn(format!("speichern fehlgeschlagen: {e}"));
                false
            }
        }
    }

    fn persist_settings(&mut self) {
        if let Err(e) = save::store(SETTINGS_FILE, &self.session.settings) {
            self.session
                .log
                .warn(format!("settings nicht gespeichert: {e}"));
        }
    }

    // ── Eingaben ──────────────────────────────────────────────────────────

    fn handle(&mut self, msg: Msg, cx: &mut Context<Self>) {
        match msg {
            // Gespeichert wird beim Beenden über `on_app_quit`.
            Msg::Quit => return cx.quit(),
            Msg::Open(screen) if self.session.running && screen.available(self.session.admin) => {
                self.switch(screen)
            }
            Msg::Back if self.session.running && self.screen != Screen::Main => {
                self.switch(Screen::Main)
            }
            Msg::CollectEvent => self.collect_event(),
            msg => {
                let (page, session) = self.page_mut(self.screen);
                if let Some(next) = page.handle(msg, session) {
                    self.switch(next);
                }
            }
        }
        let (page, session) = self.page_mut(self.screen);
        page.refresh(0., session);
        self.settle();
        cx.notify();
    }

    fn switch(&mut self, screen: Screen) {
        if screen != self.screen {
            let (page, session) = self.page_mut(self.screen);
            page.leave(session);
            self.screen = screen;
        }
    }

    fn collect_event(&mut self) {
        let s = &mut self.session;
        let now = s.now();
        let fmt = s.settings.number_format;
        match s.events.collect(now, &mut s.game, &mut s.happened) {
            Some((_, EventOutcome::Buff { kind, seconds })) => {
                s.log
                    .event(format!("{} aktiv ({} s)", kind.name(), seconds as u32))
            }
            Some((event, EventOutcome::Bytes(n))) => {
                s.log
                    .event(format!("{}: +{}", event.name(), format::bytes(n, fmt)))
            }
            None => {}
        }
    }

    // ── Zeit ──────────────────────────────────────────────────────────────

    fn tick(&mut self, dt: f64, cx: &mut Context<Self>) {
        if self.session.running {
            let s = &mut self.session;
            let now = s.now();
            s.income = s.game.tick(dt, s.time_scale, now, &mut s.happened);
            // Neue Events nur, wo man sie auch sieht.
            let can_spawn = self.screen == Screen::Main && self.window_active;
            match s.events.tick(now, can_spawn, &mut s.game) {
                Some(DirectorNews::Spawned(kind)) => s.log.warn(format!("event: {}", kind.name())),
                Some(DirectorNews::Missed {
                    outcome: Some(EventOutcome::Buff { kind, seconds }),
                    ..
                }) => s.log.warn(format!(
                    "nicht abgewehrt: {} ({} s)",
                    kind.name(),
                    seconds as u32
                )),
                Some(DirectorNews::Missed { kind, .. }) => {
                    s.log.detail(format!("verpasst: {}", kind.name()))
                }
                None => {}
            }
            if self.last_save.elapsed() >= AUTOSAVE && self.persist() {
                self.session.log.detail("autosave ok");
            }
        }
        let (page, session) = self.page_mut(self.screen);
        page.refresh(dt, session);
        self.session.anim.prune(Instant::now());
        self.settle();
        cx.notify();
    }

    fn animate_background(&mut self, dt: f32, cx: &mut Context<Self>) {
        let settings = &self.session.settings;
        if settings.ascii_background && self.window_active {
            self.bg_time += dt * backdrop::speed(settings.speed);
            cx.notify();
        }
    }

    /// Was in der Sitzung passiert ist, protokollieren, den Ansichten zeigen und speichern.
    fn settle(&mut self) {
        let fmt = self.session.settings.number_format;
        for event in std::mem::take(&mut self.session.happened) {
            let log = &mut self.session.log;
            match &event {
                GameEvent::Milestone(m) => {
                    log.event(format!("meilenstein: {}", format::bytes(*m, fmt)))
                }
                GameEvent::Unlocked(id) => log.event(format!("neu verfügbar: {id}")),
                GameEvent::BuffEnded(kind) => log.detail(format!("{} vorbei", kind.name())),
                GameEvent::Bought { index, level } => {
                    log.event(format!("installiert {}", UPGRADES[*index].id));
                    log.detail(format!("level {level:03}"));
                }
                GameEvent::Reset => {}
            }
            for screen in Screen::ALL {
                let (page, session) = self.page_mut(screen);
                page.on_event(&event, session);
            }
        }
        if std::mem::take(&mut self.session.save_game) {
            self.persist();
        }
        if std::mem::take(&mut self.session.save_settings) {
            self.persist_settings();
        }
    }

    // ── Zeichnen ──────────────────────────────────────────────────────────

    fn stats(&self) -> String {
        if !self.session.running {
            return "boot".into();
        }
        let g = &self.session.game;
        let f = self.session.settings.number_format;
        format!(
            "{} · {}/klick · {:.1} klicks/s · +{}",
            format::bytes_padded(g.whole_bytes(), f),
            format::bytes(g.per_click(), f),
            g.clicks_per_second(self.session.now()),
            format::rate(g.per_second(), f),
        )
    }

    fn hints(&self) -> Vec<Hint> {
        let mut hints = self.page(self.screen).hints(&self.session);
        hints.push(Hint::new(Msg::Quit, "beenden"));
        hints
    }
}

impl Render for ClickerApp {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let now = Instant::now();
        let s = &self.session;
        let v = View {
            theme: theme::resolve(&s.settings.theme, self.appearance),
            now,
            emit: &self.emit,
            viewport: window.viewport_size(),
            appearance: self.appearance,
            cursor_on: (self.started.elapsed().as_millis() / CURSOR_BLINK_MS).is_multiple_of(2),
        };
        let page = self.page(self.screen);
        let animate = s.settings.animations;

        let content = page.render(s, &v);
        let body = if !s.running {
            div().flex_1().flex().child(content)
        } else {
            let content = if page.fills() {
                div().flex_1().flex().child(content)
            } else {
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .p(px(24.))
                    .child(content)
            };
            div()
                .flex_1()
                .min_h(px(0.))
                .flex()
                .child(content)
                .child(render_log(&s.log, &v.theme, v.cursor_on, now, animate))
        };

        if s.anim.is_active(now) || page.animating(s, now) || (animate && s.log.is_animating(now)) {
            window.request_animation_frame();
        }

        let root = div()
            .id("root")
            .key_context("Clicker")
            .track_focus(&self.focus);
        keys::attach(root, &self.emit)
            .size_full()
            .relative()
            .bg(v.theme.bg)
            .text_color(v.theme.fg)
            .font_family(FONT)
            .text_size(px(FONT_SIZE))
            .line_height(px(LINE_H))
            .when(s.settings.ascii_background, |d| {
                d.child(backdrop::render(&s.settings, self.bg_time, &v))
            })
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .flex()
                    .flex_col()
                    .child(title_bar::render(
                        self.screen,
                        s.admin,
                        s.running,
                        self.window_active,
                        window.is_fullscreen(),
                        &v,
                    ))
                    .child(body)
                    .child(render_status_bar(
                        &v.theme,
                        self.stats(),
                        self.hints(),
                        &self.emit,
                    )),
            )
    }
}
