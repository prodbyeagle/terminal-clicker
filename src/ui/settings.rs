//! Settings-Ansicht: Liste der Einstellungen und Theme-Kacheln.

use crate::format::NumberFormat;
use crate::save::SETTINGS_FILE;
use crate::settings::{Density, Settings, Speed};
use crate::theme;
use crate::ui::list::{self, ListScreen, cycle};
use crate::ui::msg::{Msg, Screen};
use crate::ui::page::Page;
use crate::ui::session::Session;
use crate::ui::status_bar::Hint;
use crate::ui::view::View;
use crate::ui::widgets::{list_row, panel, separator};
use gpui::{AnyElement, Div, MouseButton, div, prelude::*, px};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Item {
    AsciiBackground,
    Density,
    Speed,
    Animations,
    Numbers,
    Theme,
}

const ITEMS: [Item; 6] = [
    Item::AsciiBackground,
    Item::Density,
    Item::Speed,
    Item::Animations,
    Item::Numbers,
    Item::Theme,
];

fn change(settings: &mut Settings, item: Item, delta: i32) {
    match item {
        Item::AsciiBackground => settings.ascii_background = !settings.ascii_background,
        Item::Animations => settings.animations = !settings.animations,
        Item::Density => settings.density = cycle(&Density::ALL, settings.density, delta),
        Item::Speed => settings.speed = cycle(&Speed::ALL, settings.speed, delta),
        Item::Numbers => {
            settings.number_format = cycle(&NumberFormat::ALL, settings.number_format, delta)
        }
        Item::Theme => {
            let all: Vec<&str> = theme::choices().collect();
            settings.theme = cycle(&all, settings.theme.as_str(), delta).to_string();
        }
    }
}

#[derive(Default)]
pub struct SettingsScreen {
    selected: usize,
}

impl ListScreen for SettingsScreen {
    fn len(&self) -> usize {
        ITEMS.len()
    }

    fn selected(&self) -> usize {
        self.selected
    }

    fn set_selected(&mut self, index: usize) {
        self.selected = index;
    }

    fn activate(&mut self, index: usize, s: &mut Session) {
        self.adjust(index, 1, s);
    }

    fn adjust(&mut self, index: usize, delta: i32, s: &mut Session) {
        change(&mut s.settings, ITEMS[index], delta);
        if !s.settings.animations {
            s.anim.clear();
        }
        s.save_settings = true;
    }
}

impl Page for SettingsScreen {
    fn hints(&self, _s: &Session) -> Vec<Hint> {
        vec![
            Hint::info(&[Msg::Move(-1), Msg::Move(1)], "wählen"),
            Hint::new(Msg::Confirm, "umschalten"),
            Hint::info(&[Msg::AdjustSelected(-1), Msg::AdjustSelected(1)], "ändern"),
            Hint::open(Screen::Upgrades),
            Hint::new(Msg::Back, "zurück"),
        ]
    }

    fn handle(&mut self, msg: Msg, s: &mut Session) -> Option<Screen> {
        match msg {
            Msg::PickTheme(id) => {
                s.settings.theme = id.to_string();
                s.save_settings = true;
            }
            msg => list::handle(self, msg, s),
        }
        None
    }

    fn render(&self, s: &Session, v: &View) -> AnyElement {
        let (theme, settings) = (&v.theme, &s.settings);
        let check = |on: bool| div().child(if on { "[x]" } else { "[ ]" });
        let stepper = |index: usize, value: &str| {
            let arrow = |label: &'static str, delta: i32| {
                div()
                    .px(px(6.))
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        v.emit.on_press(Msg::Adjust(index, delta)),
                    )
                    .child(label)
            };
            div()
                .flex()
                .child(arrow("<", -1))
                .child(value.to_string())
                .child(arrow(">", 1))
        };

        let rows = ITEMS.iter().enumerate().map(|(i, item)| {
            let (mark, label, needs_background) = match item {
                Item::AsciiBackground => {
                    (check(settings.ascii_background), "ascii-hintergrund", false)
                }
                Item::Density => (stepper(i, settings.density.label()), "  dichte", true),
                Item::Speed => (
                    stepper(i, settings.speed.label()),
                    "  geschwindigkeit",
                    true,
                ),
                Item::Animations => (check(settings.animations), "animationen", false),
                Item::Numbers => (stepper(i, settings.number_format.label()), "zahlen", false),
                Item::Theme => (stepper(i, theme::label(&settings.theme)), "theme", false),
            };
            let is_sel = i == self.selected;
            let greyed = needs_background && !settings.ascii_background;
            list_row("setting", i, is_sel, is_sel, v)
                .py(px(6.))
                .when(greyed && !is_sel, |d| d.text_color(theme.dim))
                .child(div().w(px(140.)).flex_none().child(mark))
                .child(div().flex_1().child(label))
        });

        panel(theme, "SETTINGS")
            .w(px(600.))
            .max_w_full()
            .child(separator(theme))
            .children(rows)
            .child(theme_picker(settings, v))
            .child(
                div()
                    .pt(px(12.))
                    .text_color(theme.dim)
                    .child("# klick = umschalten · rechtsklick = zurück"),
            )
            .child(
                div()
                    .text_color(theme.dim)
                    .child(format!("# gespeichert in {SETTINGS_FILE}")),
            )
            .into_any_element()
    }
}

/// Themes werden automatisch anhand ihrer tatsächlichen Breite umgebrochen.
fn theme_picker(settings: &Settings, v: &View) -> Div {
    let chip = |id: &'static str| {
        let preview = theme::resolve(id, v.appearance);
        let active = settings.theme == id;

        div()
            .px(px(10.))
            .py(px(4.))
            .flex()
            .gap(px(8.))
            .flex_none()
            .cursor_pointer()
            .bg(preview.bg)
            .text_color(preview.fg)
            .map(|d| {
                if active {
                    d.border_2().border_color(v.theme.fg)
                } else {
                    d.border_1().border_color(v.theme.line).m(px(1.))
                }
            })
            .on_mouse_down(MouseButton::Left, v.emit.on_press(Msg::PickTheme(id)))
            .child(
                div()
                    .text_color(preview.dim)
                    .child(if active { ">" } else { "·" }),
            )
            .child(theme::label(id))
    };

    let ids: Vec<&'static str> = theme::choices().collect();

    div()
        .flex()
        .flex_col()
        .w_full()
        .max_w_full()
        .gap(px(6.))
        .pt(px(6.))
        .pl(px(34.))
        .child(
            div()
                .flex()
                .flex_wrap()
                .w_full()
                .max_w_full()
                .gap(px(6.))
                .children(ids.into_iter().map(chip)),
        )
}
