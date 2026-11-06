//! ASCII-Hintergrund hinter allem: Wahl aus den Settings in Zeichen umsetzen.

use crate::ascii::background::{Style, render_rows};
use crate::settings::{Density, Scale, Settings, Speed, Strength};
use crate::ui::layout::{FONT_SIZE, char_width};
use crate::ui::view::View;
use gpui::{Div, div, prelude::*, px};

const LINE_H: f32 = 18.;

/// Verschiebung der Zeichenrampe: mehr Dichte = mehr Zeichen.
fn bias(density: Density) -> f32 {
    match density {
        Density::Low => -0.18,
        Density::Mid => 0.0,
        Density::High => 0.16,
    }
}

fn scale(scale: Scale) -> f32 {
    match scale {
        Scale::Fine => 0.6,
        Scale::Mid => 1.0,
        Scale::Coarse => 1.7,
    }
}

/// Deckkraft der Zeichen.
fn opacity(strength: Strength) -> f32 {
    match strength {
        Strength::Faint => 0.1,
        Strength::Low => 0.2,
        Strength::Mid => 0.34,
        Strength::High => 0.55,
    }
}

/// Wie schnell die Zeit im Rauschen läuft.
pub fn speed(speed: Speed) -> f32 {
    match speed {
        Speed::Slow => 0.08,
        Speed::Mid => 0.18,
        Speed::Fast => 0.4,
    }
}

pub fn render(settings: &Settings, t: f32, v: &View) -> Div {
    let style = Style {
        pattern: settings.pattern,
        charset: settings.charset,
        bias: bias(settings.density),
        scale: scale(settings.scale),
    };
    let cols = (f32::from(v.viewport.width) / char_width(FONT_SIZE)).ceil() as usize + 1;
    let rows = (f32::from(v.viewport.height) / LINE_H).ceil() as usize + 1;
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .overflow_hidden()
        .flex()
        .flex_col()
        .text_color(v.theme.dim.opacity(opacity(settings.strength)))
        .children(render_rows(cols, rows, t, &style).into_iter().map(|line| {
            div()
                .h(px(LINE_H))
                .line_height(px(LINE_H))
                .whitespace_nowrap()
                .child(line)
        }))
}
