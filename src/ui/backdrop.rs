//! ASCII-Hintergrund hinter allem: Wahl aus den Settings in Rauschen umsetzen.

use crate::ascii::background::render_rows;
use crate::settings::{Density, Settings, Speed};
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

/// Wie schnell die Zeit im Rauschen läuft.
pub fn speed(speed: Speed) -> f32 {
    match speed {
        Speed::Slow => 0.08,
        Speed::Mid => 0.18,
        Speed::Fast => 0.4,
    }
}

pub fn render(settings: &Settings, t: f32, v: &View) -> Div {
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
        .text_color(v.theme.dim.opacity(0.2))
        .children(
            render_rows(cols, rows, t, bias(settings.density))
                .into_iter()
                .map(|line| {
                    div()
                        .h(px(LINE_H))
                        .line_height(px(LINE_H))
                        .whitespace_nowrap()
                        .child(line)
                }),
        )
}
