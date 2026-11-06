use crate::ascii::glitch::decode_text;
use crate::theme::Theme;
use crate::ui::layout::LINE_H;
use gpui::{Div, div, prelude::*, px};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

const LOG_CAPACITY: usize = 50;
const PANEL_W: f32 = 380.;
/// So lange schiebt sich eine neue Zeile von unten herein.
const SLIDE: Duration = Duration::from_millis(160);

fn decode_duration(line: &str) -> Duration {
    Duration::from_millis((260 + line.chars().count() as u64 * 8).min(560))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogKind {
    Event,
    Detail,
    Warn,
}

impl LogKind {
    fn prefix(self) -> &'static str {
        match self {
            LogKind::Event => "> ",
            LogKind::Detail => "  ",
            LogKind::Warn => "! ",
        }
    }
}

struct Line {
    kind: LogKind,
    text: String,
    added: Instant,
}

#[derive(Default)]
pub struct Log {
    lines: VecDeque<Line>,
}

impl Log {
    fn push(&mut self, kind: LogKind, text: impl Into<String>) {
        if self.lines.len() == LOG_CAPACITY {
            self.lines.pop_front();
        }
        let text = format!("{}{}", kind.prefix(), text.into());
        self.lines.push_back(Line {
            kind,
            text,
            added: Instant::now(),
        });
    }

    pub fn event(&mut self, text: impl Into<String>) {
        self.push(LogKind::Event, text);
    }

    pub fn detail(&mut self, text: impl Into<String>) {
        self.push(LogKind::Detail, text);
    }

    pub fn warn(&mut self, text: impl Into<String>) {
        self.push(LogKind::Warn, text);
    }

    pub fn is_animating(&self, now: Instant) -> bool {
        self.lines.back().is_some_and(|l| {
            now.saturating_duration_since(l.added) < decode_duration(&l.text).max(SLIDE)
        })
    }
}

pub fn render_log(log: &Log, theme: &Theme, cursor_on: bool, now: Instant, animate: bool) -> Div {
    let lines = log.lines.iter().map(|line| {
        let age = now.saturating_duration_since(line.added);
        let dim = line.kind == LogKind::Detail;
        let (height, text, opacity) = if animate {
            let slide = (age.as_secs_f32() / SLIDE.as_secs_f32()).min(1.);
            let eased = 1. - (1. - slide).powi(3);
            let decode = age.as_secs_f32() / decode_duration(&line.text).as_secs_f32();
            (
                LINE_H * eased,
                decode_text(&line.text, decode, age),
                0.35 + 0.65 * eased,
            )
        } else {
            (LINE_H, line.text.clone(), 1.)
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
        .w(px(PANEL_W))
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
