//! Eigene Titelleiste im Terminal-Look. Die macOS-Ampel bleibt nativ und sitzt
//! links in der freigehaltenen Fläche; Ziehen und Doppelklick laufen über uns.

use crate::ui::layout::{FONT_SIZE, LINE_H, char_width};

const TITLE_BAR_H: f32 = 36.;
use crate::ui::msg::{Msg, Screen};
use crate::ui::view::View;
use crate::ui::window_drag;
use gpui::{Div, MouseButton, TitlebarOptions, div, point, prelude::*, px};

const TRAFFIC_LIGHTS_X: f32 = 14.;
/// Höhe eines Ampel-Knopfs (macOS-Vorgabe).
const TRAFFIC_LIGHT_H: f32 = 14.;
/// Sichtbare Breite der drei Ampel-Kreise.
const TRAFFIC_LIGHTS_SPAN: f32 = 60.;
const TRAFFIC_LIGHTS_W: f32 = TRAFFIC_LIGHTS_X + TRAFFIC_LIGHTS_SPAN + TRAFFIC_LIGHTS_X;

pub fn options() -> TitlebarOptions {
    TitlebarOptions {
        title: Some("Terminal Clicker".into()),
        appears_transparent: true,
        traffic_light_position: Some(point(
            px(TRAFFIC_LIGHTS_X),
            px((TITLE_BAR_H - TRAFFIC_LIGHT_H) / 2.),
        )),
    }
}

/// `running`: nach dem Boot – erst dann gibt es Reiter.
pub fn render(
    screen: Screen,
    admin: bool,
    running: bool,
    active: bool,
    fullscreen: bool,
    v: &View,
) -> Div {
    let theme = &v.theme;
    let text = if active { theme.fg } else { theme.dim };
    let hover_bg = theme.line;
    let cols = (f32::from(v.viewport.width) / char_width(FONT_SIZE)) as u32;
    let rows = (f32::from(v.viewport.height) / LINE_H) as u32;

    let tabs = Screen::ALL
        .into_iter()
        .filter(|s| *s != Screen::Boot && s.available(admin));
    let tab_bar = div().flex().h_full().children(tabs.map(|tab| {
        let selected = screen == tab;
        div()
            .px(px(12.))
            .h_full()
            .flex()
            .items_center()
            .cursor_pointer()
            .border_l_1()
            .border_color(theme.line)
            .when(selected, |d| d.bg(text).text_color(theme.bg))
            .when(!selected, |d| {
                d.text_color(theme.dim).hover(move |s| s.bg(hover_bg))
            })
            .on_mouse_down(MouseButton::Left, v.emit.on_press(Msg::Open(tab)))
            .child(tab.name())
    }));

    // Im Vollbild blendet macOS die Ampel aus – dann keinen Platz freihalten.
    let lights_space = if fullscreen { 0. } else { TRAFFIC_LIGHTS_W };

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
        // Doppelklick verhält sich wie in macOS eingestellt (zoomen/minimieren).
        .on_mouse_down(MouseButton::Left, |event, window, cx| {
            if event.click_count == 2 {
                window.titlebar_double_click();
            } else {
                window_drag::begin_window_drag(window, cx);
            }
        })
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
                        .child(format!("— {} — {cols}×{rows}", screen.name())),
                ),
        )
        .child(
            div()
                .h_full()
                .flex()
                .items_center()
                .child(div().w(px(lights_space)).flex_none())
                .when(running, |d| d.child(tab_bar))
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
