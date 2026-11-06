//! Bausteine, die mehrere Ansichten gleich aussehen lassen.

use crate::theme::Theme;
use crate::ui::msg::Msg;
use crate::ui::view::View;
use gpui::{Div, MouseButton, SharedString, Stateful, div, prelude::*, px};

/// Kasten mit Titel, wie ihn alle Listen-Ansichten haben.
pub fn panel(theme: &Theme, title: &'static str) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(4.))
        .p(px(16.))
        .bg(theme.bg)
        .border_1()
        .border_color(theme.line)
        .child(div().pb(px(10.)).child(title))
}

pub fn separator(theme: &Theme) -> Div {
    div().border_b_1().border_color(theme.line).mb(px(6.))
}

/// Listenzeile: Maus darüber wählt aus, Klick aktiviert, Rechtsklick stellt zurück.
/// `inverted` färbt sie (Auswahl, Blinken), `selected` setzt den `>`-Marker.
pub fn list_row(
    id: impl Into<SharedString>,
    index: usize,
    selected: bool,
    inverted: bool,
    v: &View,
) -> Stateful<Div> {
    let (fg, bg) = if inverted {
        (v.theme.bg, v.theme.fg)
    } else {
        (v.theme.fg, v.theme.bg)
    };
    div()
        .id((id.into(), index))
        .flex()
        .flex_none()
        .px(px(10.))
        .bg(bg)
        .text_color(fg)
        .cursor_pointer()
        .on_hover(v.emit.on_hover(Msg::Select(index)))
        .on_mouse_down(MouseButton::Left, v.emit.on_press(Msg::Activate(index)))
        .on_mouse_down(MouseButton::Right, v.emit.on_press(Msg::Adjust(index, -1)))
        .child(
            div()
                .w(px(24.))
                .flex_none()
                .child(if selected { ">" } else { "" }),
        )
}
