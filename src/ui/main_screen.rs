//! Hauptansicht: Zähler, KLICK-Button, Datenpakete, laufende Boni, Event-Popup.

use crate::ascii::counter::{CounterRoll, RollDir};
use crate::ascii::glitch::{decode_text, scramble_frames};
use crate::format::{self, bar};
use crate::game::GameEvent;
use crate::game::events::ActiveEvent;
use crate::ui::fx::Fx;
use crate::ui::keys;
use crate::ui::layout::char_width;
use crate::ui::msg::{Msg, Screen};
use crate::ui::packets::{self, Packets};
use crate::ui::page::Page;
use crate::ui::session::Session;
use crate::ui::status_bar::Hint;
use crate::ui::view::View;
use gpui::{AnyElement, Div, FontWeight, MouseButton, div, prelude::*, px, relative};
use std::time::{Duration, Instant};

const COUNTER_SIZE: f32 = 120.;
const COUNTER_LINE_H: f32 = 132.;
const COUNTER_ROLL_MS: u64 = 140;
const BUTTON_W: f32 = 260.;
const EVENT_W: f32 = 240.;

pub struct MainScreen {
    counter: CounterRoll,
    packets: Packets,
}

impl MainScreen {
    pub fn new(s: &Session) -> Self {
        let mut counter = CounterRoll::new(COUNTER_ROLL_MS);
        let bytes = s.game.whole_bytes();
        counter.update(
            bytes,
            &format::counter(bytes, s.settings.number_format),
            false,
            Instant::now(),
        );
        Self {
            counter,
            packets: Packets::default(),
        }
    }

    fn render_counter(&self, s: &Session, v: &View) -> Div {
        // Meilenstein-Scramble hat Vorrang vor dem Rollen.
        let scramble = s
            .anim
            .find(Fx::Scramble, v.now)
            .and_then(|a| a.frame(v.now))
            .map(str::to_string);
        let number = match scramble {
            Some(text) => div().child(text),
            None => div().flex().children(self.counter.digits().iter().map(|d| {
                let cell = div()
                    .relative()
                    .w(px(char_width(COUNTER_SIZE)))
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
                match self.counter.progress(d, v.now) {
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
            .child(div().text_color(v.theme.dim).child("BYTES"))
            .child(
                number
                    .text_size(px(COUNTER_SIZE))
                    .line_height(px(COUNTER_LINE_H))
                    .whitespace_nowrap(),
            )
            .child(div().text_color(v.theme.dim).child(format!(
                "{}/klick",
                format::bytes(s.game.per_click(), s.settings.number_format)
            )))
    }
}

impl Page for MainScreen {
    fn hints(&self, s: &Session) -> Vec<Hint> {
        let mut hints = vec![
            Hint::new(Msg::Click, "klick"),
            Hint::open(Screen::Upgrades),
            Hint::open(Screen::Settings),
            Hint::open(Screen::Help),
        ];
        if s.events.active().is_some() {
            hints.push(Hint::new(Msg::CollectEvent, "event!"));
        }
        if Screen::Admin.available(s.admin) {
            hints.push(Hint::open(Screen::Admin));
        }
        hints
    }

    fn render(&self, s: &Session, v: &View) -> AnyElement {
        div()
            .flex_1()
            .relative()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            // Pakete zuerst, damit sie hinter Zähler und Button liegen.
            .children(packets::render(s, v))
            .child(self.render_counter(s, v))
            .child(render_button(s, v))
            .child(
                div()
                    .mt(px(14.))
                    .text_color(v.theme.dim)
                    .child(format!("{} oder klick", keys::label(&[Msg::Click]))),
            )
            .child(render_buffs(s, v))
            // Event zuletzt, damit es über allem liegt und anklickbar ist.
            .children(s.events.active().map(|e| render_event(e, s, v)))
            .into_any_element()
    }

    fn handle(&mut self, msg: Msg, s: &mut Session) -> Option<Screen> {
        if msg == Msg::Click {
            click(s, Instant::now());
        }
        None
    }

    fn refresh(&mut self, dt: f64, s: &mut Session) {
        self.packets.tick(dt, s);
        let bytes = s.game.whole_bytes();
        let text = format::counter(bytes, s.settings.number_format);
        self.counter
            .update(bytes, &text, s.settings.animations, Instant::now());
    }

    fn on_event(&mut self, event: &GameEvent, s: &mut Session) {
        match event {
            GameEvent::Milestone(_) if s.settings.animations => {
                let width = format::counter(s.game.whole_bytes(), s.settings.number_format)
                    .chars()
                    .count();
                let frames = scramble_frames(&mut s.rng, width, 6);
                s.anim.play(Fx::Scramble, frames, 50, Instant::now());
            }
            GameEvent::Reset => {
                self.packets.reset();
                s.anim.clear();
            }
            _ => {}
        }
    }

    fn animating(&self, _s: &Session, now: Instant) -> bool {
        self.counter.is_active(now)
    }

    fn fills(&self) -> bool {
        true
    }
}

fn click(s: &mut Session, now: Instant) {
    let result = s.game.click(s.now(), &mut s.happened);
    // Gedrückt-Zustand ca. 80 ms invertiert – auch ohne „animationen“.
    s.anim
        .play(Fx::ButtonPress, vec![String::new(); 2], 40, now);
    if s.settings.animations {
        let x = s.rng.range(-60., 60.);
        let amount = format::bytes(result.gain, s.settings.number_format);
        let label = if result.crit {
            format!("KRIT +{amount}")
        } else {
            format!("+{amount}")
        };
        s.anim.play(
            Fx::Float {
                x,
                crit: result.crit,
            },
            vec![label.clone(), label, "·".into()],
            130,
            now,
        );
        s.anim
            .play(Fx::ButtonPulse, vec!["[[ KLICK ]]".into(); 2], 60, now);
    }
}

fn render_button(s: &Session, v: &View) -> Div {
    let pressed = s.anim.find(Fx::ButtonPress, v.now).is_some();
    let label = s
        .anim
        .find(Fx::ButtonPulse, v.now)
        .and_then(|a| a.frame(v.now))
        .unwrap_or("[ KLICK ]");
    let (fg, bg) = if pressed {
        (v.theme.bg, v.theme.fg)
    } else {
        (v.theme.fg, v.theme.bg)
    };

    let floats = s.anim.iter(v.now).filter_map(|a| {
        let Fx::Float { x, crit } = a.kind else {
            return None;
        };
        let p = a.progress(v.now);
        Some(
            div()
                .absolute()
                .top(px(-22. - p * 46.))
                .left(px(BUTTON_W / 2. - 100. + x))
                .w(px(200.))
                .flex()
                .justify_center()
                .whitespace_nowrap()
                .opacity(1. - p * 0.7)
                .when(crit, |d| d.text_size(px(18.)).font_weight(FontWeight::BOLD))
                .child(a.frame(v.now)?.to_string()),
        )
    });

    div()
        .relative()
        .mt(px(48.))
        .child(
            div()
                .id("klick")
                .w(px(BUTTON_W))
                .h(px(56.))
                .flex()
                .items_center()
                .justify_center()
                .border_2()
                .border_color(v.theme.fg)
                .bg(bg)
                .text_color(fg)
                .text_size(px(18.))
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, v.emit.on_press(Msg::Click))
                .child(label.to_string()),
        )
        .children(floats)
}

fn render_buffs(s: &Session, v: &View) -> Div {
    div()
        .mt(px(18.))
        .min_h(px(44.))
        .flex()
        .flex_col()
        .items_center()
        .gap(px(2.))
        .children(s.game.buffs().iter().map(|b| {
            let bad = b.kind.is_bad();
            div()
                .flex()
                .gap(px(10.))
                .whitespace_nowrap()
                .when(bad, |d| d.text_color(v.theme.dim))
                .child(format!("{}{}", if bad { "! " } else { "" }, b.kind.title()))
                .child(
                    div()
                        .text_color(v.theme.dim)
                        .child(bar(b.remaining / b.duration, 16)),
                )
                .child(format!("{:>2} s", b.remaining.ceil() as u32))
                .child(div().text_color(v.theme.dim).child(b.kind.effect_hint()))
        }))
}

/// Eine der vier Ecken, abhängig vom Event – nie über Zähler oder Button.
fn corner(id: u64) -> (f32, f32) {
    let jitter = (id.wrapping_mul(0x9e37_79b9) % 1000) as f32 / 1000.;
    let x = if id.is_multiple_of(2) {
        0.03 + 0.09 * jitter
    } else {
        0.6 + 0.08 * jitter
    };
    let y = if (id / 2).is_multiple_of(2) {
        0.04 + 0.1 * jitter
    } else {
        0.78 + 0.06 * jitter
    };
    (x, y)
}

fn render_event(event: ActiveEvent, s: &Session, v: &View) -> Div {
    let (x, y) = corner(event.id);
    let age = Duration::from_secs_f64(event.age(s.now()));
    let animate = s.settings.animations;
    let blink = animate && (age.as_millis() / 450).is_multiple_of(2);
    let decode = if animate {
        age.as_secs_f32() / 0.35
    } else {
        1.
    };
    let threat = event.kind.is_threat();
    let prefix = if threat { "!!" } else { "!" };
    let title = decode_text(&format!("{prefix} {}", event.kind.title()), decode, age);
    let (head_fg, head_bg) = if blink {
        (v.theme.bg, v.theme.fg)
    } else {
        (v.theme.fg, v.theme.bg)
    };

    div()
        .absolute()
        .left(relative(x))
        .top(relative(y))
        .w(px(EVENT_W))
        .flex()
        .flex_col()
        .bg(v.theme.bg)
        .border_2()
        .border_color(v.theme.fg)
        // Bedrohungen gestrichelt – die will man wegklicken.
        .when(threat, |d| d.border_dashed())
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, v.emit.on_press(Msg::CollectEvent))
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
                .child(event.kind.description()),
        )
        .child(
            div()
                .px(px(10.))
                .pb(px(6.))
                .text_size(px(12.))
                .text_color(v.theme.dim)
                .child(format!(
                    "klick oder [{}] · noch {} s",
                    keys::label(&[Msg::CollectEvent]),
                    event.remaining(s.now()).ceil() as u32
                )),
        )
}
