use crate::format::NumberFormat;
use crate::save::{LEVEL_LABELS, SPEED_LABELS, Settings};
use crate::theme::{self, Theme};
use crate::ui::main_view::ClickerApp;
use gpui::{Context, Div, MouseButton, WindowAppearance, div, prelude::*, px};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Item {
    AsciiBackground,
    Density,
    Speed,
    Animations,
    Numbers,
    Theme,
}

pub const ITEMS: [Item; 6] = [
    Item::AsciiBackground,
    Item::Density,
    Item::Speed,
    Item::Animations,
    Item::Numbers,
    Item::Theme,
];

fn step(value: usize, delta: i32, len: usize) -> usize {
    (value as i32 + delta).rem_euclid(len as i32) as usize
}

/// Wert eines Eintrags um `delta` verändern (enter/klick = +1, ←/→ bzw. Rechtsklick = -1).
pub fn change(settings: &mut Settings, item: Item, delta: i32) {
    match item {
        Item::AsciiBackground => settings.ascii_background = !settings.ascii_background,
        Item::Animations => settings.animations = !settings.animations,
        Item::Density => {
            settings.density = step(settings.density as usize, delta, LEVEL_LABELS.len()) as u8
        }
        Item::Speed => {
            settings.speed = step(settings.speed as usize, delta, SPEED_LABELS.len()) as u8
        }
        Item::Numbers => {
            let all = NumberFormat::ALL;
            let i = all.iter().position(|f| *f == settings.number_format).unwrap_or(0);
            settings.number_format = all[step(i, delta, all.len())];
        }
        Item::Theme => {
            let all: Vec<&str> = theme::choices().collect();
            let i = all.iter().position(|c| *c == settings.theme).unwrap_or(0);
            settings.theme = all[step(i, delta, all.len())].to_string();
        }
    }
}

fn check(on: bool) -> String {
    if on { "[x]".into() } else { "[ ]".into() }
}

/// `< wert >` mit klickbaren Pfeilen links/rechts.
fn stepper(index: usize, value: &str, cx: &mut Context<ClickerApp>) -> Div {
    let arrow = |label: &'static str, delta: i32, cx: &mut Context<ClickerApp>| {
        div()
            .px(px(6.))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.click_setting(index, delta, cx);
                }),
            )
            .child(label)
    };
    div()
        .flex()
        .child(arrow("<", -1, cx))
        .child(value.to_string())
        .child(arrow(">", 1, cx))
}

/// Theme-Auswahl als Kacheln, jede in ihren eigenen Farben – ein Klick wählt sie.
fn theme_picker(
    settings: &Settings,
    theme: &Theme,
    appearance: WindowAppearance,
    cx: &mut Context<ClickerApp>,
) -> Div {
    let chip = |id: &'static str, cx: &mut Context<ClickerApp>| {
        let preview = theme::resolve(id, appearance);
        let active = settings.theme == id;
        div()
            .px(px(10.))
            .py(px(4.))
            .flex()
            .gap(px(8.))
            .cursor_pointer()
            .bg(preview.bg)
            .text_color(preview.fg)
            .map(|d| {
                if active {
                    d.border_2().border_color(theme.fg)
                } else {
                    d.border_1().border_color(theme.line).m(px(1.))
                }
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| this.pick_theme(id, cx)),
            )
            .child(div().text_color(preview.dim).child(if active { ">" } else { "·" }))
            .child(theme::label(id))
    };

    // Feste Zeilen statt flex-wrap: das Layout misst Umbrüche sonst zu hoch.
    let ids: Vec<&'static str> = theme::choices().collect();
    div()
        .flex()
        .flex_col()
        .gap(px(6.))
        .pt(px(6.))
        .pl(px(34.))
        .children(ids.chunks(CHIPS_PER_ROW).map(|row| {
            div()
                .flex()
                .gap(px(6.))
                .children(row.iter().map(|id| chip(id, cx)))
        }))
}

const CHIPS_PER_ROW: usize = 5;

/// Settings-Liste. Maus: überfahren = auswählen, klicken = umschalten/weiter,
/// Rechtsklick = zurück, `<`/`>` = Wert ändern.
pub fn render_settings(
    settings: &Settings,
    selected: usize,
    theme: &Theme,
    appearance: WindowAppearance,
    cx: &mut Context<ClickerApp>,
) -> Div {
    let rows: Vec<_> = ITEMS
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let (mark, label, dependent) = match item {
                Item::AsciiBackground => (
                    div().child(check(settings.ascii_background)),
                    "ascii-hintergrund",
                    false,
                ),
                Item::Density => (
                    stepper(i, LEVEL_LABELS[settings.density as usize % 3], cx),
                    "  dichte",
                    true,
                ),
                Item::Speed => (
                    stepper(i, SPEED_LABELS[settings.speed as usize % 3], cx),
                    "  geschwindigkeit",
                    true,
                ),
                Item::Animations => (div().child(check(settings.animations)), "animationen", false),
                Item::Numbers => (stepper(i, settings.number_format.label(), cx), "zahlen", false),
                Item::Theme => (stepper(i, theme::label(&settings.theme), cx), "theme", false),
            };
            let is_sel = i == selected;
            let greyed = dependent && !settings.ascii_background;
            let (fg, bg) = if is_sel { (theme.bg, theme.fg) } else { (theme.fg, theme.bg) };

            div()
                .id(("setting", i))
                .flex()
                .px(px(10.))
                .py(px(6.))
                .bg(bg)
                .text_color(fg)
                .cursor_pointer()
                .when(greyed && !is_sel, |d| d.text_color(theme.dim))
                .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                    if *hovered {
                        this.hover_setting(i, cx);
                    }
                }))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| this.click_setting(i, 1, cx)),
                )
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, _, _, cx| this.click_setting(i, -1, cx)),
                )
                .child(div().w(px(24.)).flex_none().child(if is_sel { ">" } else { "" }))
                .child(div().w(px(140.)).flex_none().child(mark))
                .child(div().flex_1().child(label))
        })
        .collect();

    div()
        .flex()
        .flex_col()
        .gap(px(4.))
        .w(px(600.))
        .max_w_full()
        .p(px(16.))
        .bg(theme.bg)
        .border_1()
        .border_color(theme.line)
        .child(div().pb(px(10.)).child("SETTINGS"))
        .child(div().border_b_1().border_color(theme.line).mb(px(6.)))
        .children(rows)
        .child(theme_picker(settings, theme, appearance, cx))
        .child(
            div()
                .pt(px(12.))
                .text_color(theme.dim)
                .child("# klick = umschalten · rechtsklick = zurück"),
        )
        .child(div().text_color(theme.dim).child("# gespeichert in settings.json"))
}
