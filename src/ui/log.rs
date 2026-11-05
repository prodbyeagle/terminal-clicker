use crate::ascii::anim::decode_text;
use crate::theme::Theme;
use gpui::{Div, div, prelude::*, px};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub const LOG_CAPACITY: usize = 50;
pub const LOG_LINE_H: f32 = 20.;
pub const LOG_PANEL_W: f32 = 380.;
/// So lange schiebt sich eine neue Zeile von unten herein.
const SLIDE: Duration = Duration::from_millis(160);
/// Rauschen wechselt alle 40 ms.
const GLITCH_TICK_MS: u128 = 40;

/// Wie lange eine Zeile braucht, um sich aus dem Rauschen zu entschlüsseln.
fn decode_duration(line: &str) -> Duration {
    Duration::from_millis((260 + line.chars().count() as u64 * 8).min(560))
}

struct Line {
    text: String,
    added: Instant,
}

/// Ringpuffer der letzten Log-Zeilen.
#[derive(Default)]
pub struct Log {
    lines: VecDeque<Line>,
    pushed: u64,
}

impl Log {
    pub fn push(&mut self, line: impl Into<String>) {
        if self.lines.len() == LOG_CAPACITY {
            self.lines.pop_front();
        }
        self.pushed += 1;
        self.lines.push_back(Line {
            text: line.into(),
            added: Instant::now(),
        });
    }

    /// Anzahl aller jemals geschriebenen Zeilen (wächst auch bei vollem Puffer).
    pub fn pushed(&self) -> u64 {
        self.pushed
    }

    /// Läuft gerade noch eine Einblend-Animation?
    pub fn is_animating(&self, now: Instant) -> bool {
        self.lines.back().is_some_and(|l| {
            now.saturating_duration_since(l.added) < decode_duration(&l.text).max(SLIDE)
        })
    }
}

/// Log-Panel rechts: solide Fläche, unten angeheftet, blinkender Cursor.
/// Neue Zeilen schieben sich weich herein und entschlüsseln sich aus ASCII-Rauschen.
pub fn render_log(log: &Log, theme: &Theme, cursor_on: bool, now: Instant, animate: bool) -> Div {
    let lines = log.lines.iter().map(|line| {
        let age = now.saturating_duration_since(line.added);
        let dim = line.text.starts_with("  ");
        let (height, text, opacity) = if animate {
            let slide = (age.as_secs_f32() / SLIDE.as_secs_f32()).min(1.);
            let eased = 1. - (1. - slide).powi(3);
            let decode = age.as_secs_f32() / decode_duration(&line.text).as_secs_f32();
            let tick = (age.as_millis() / GLITCH_TICK_MS) as u64;
            (
                LOG_LINE_H * eased,
                decode_text(&line.text, decode, tick),
                0.35 + 0.65 * eased,
            )
        } else {
            (LOG_LINE_H, line.text.clone(), 1.)
        };
        div()
            .flex_none()
            .h(px(height))
            .overflow_hidden()
            .whitespace_nowrap()
            .opacity(opacity)
            .when(dim, |d| d.text_color(theme.dim))
            .child(text)
    });

    div()
        .w(px(LOG_PANEL_W))
        .h_full()
        .flex()
        .flex_col()
        .flex_none()
        .bg(theme.bg)
        .border_l_1()
        .border_color(theme.line)
        .child(
            div()
                .px(px(14.))
                .py(px(8.))
                .border_b_1()
                .border_color(theme.line)
                .text_color(theme.dim)
                .child("LOG"),
        )
        .child(
            div()
                .flex_1()
                .min_h(px(0.))
                .overflow_hidden()
                .flex()
                .flex_col()
                .justify_end()
                .px(px(14.))
                .py(px(10.))
                .children(lines)
                .child(
                    div()
                        .flex()
                        .child("> ")
                        .child(if cursor_on { "_" } else { " " }),
                ),
        )
}
