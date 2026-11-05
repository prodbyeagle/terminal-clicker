//! Admin-Panel: Bytes und Upgrades geben, Zeitraffer, Reset – plus Stats.

use crate::game::{Game, UPGRADES};
use crate::theme::Theme;
use crate::ui::main_view::ClickerApp;
use crate::ui::upgrades::trim_float;
use gpui::{Context, Div, MouseButton, div, prelude::*, px};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Item {
    Grant(u64),
    Times10,
    UnlockAll,
    LevelAll,
    SpawnEvent,
    TimeWarp,
    Reset,
}

pub const ITEMS: [Item; 10] = [
    Item::Grant(1_000),
    Item::Grant(100_000),
    Item::Grant(10_000_000),
    Item::Grant(1_000_000_000),
    Item::Times10,
    Item::UnlockAll,
    Item::LevelAll,
    Item::SpawnEvent,
    Item::TimeWarp,
    Item::Reset,
];

/// Zeitraffer-Stufen für passives Einkommen.
pub const TIME_SCALES: [u32; 4] = [1, 10, 100, 1000];

/// Was das Panel außer dem Spielstand noch anzeigt.
pub struct AdminInfo {
    pub clicks_per_second: f64,
    pub session_seconds: f64,
    pub packets: usize,
    pub time_scale: u32,
    /// Reset wurde einmal ausgelöst und wartet auf Bestätigung.
    pub reset_armed: bool,
}

/// 1234567 → "1.234.567"
pub fn fmt_num(v: u64) -> String {
    let digits = v.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

/// 3725 s → "1h 02m 05s"
pub fn fmt_duration(seconds: f64) -> String {
    let s = seconds.max(0.) as u64;
    let (h, m, s) = (s / 3600, (s / 60) % 60, s % 60);
    if h > 0 {
        format!("{h}h {m:02}m {s:02}s")
    } else {
        format!("{m}m {s:02}s")
    }
}

fn label(item: Item, info: &AdminInfo) -> String {
    match item {
        Item::Grant(n) => format!("+{} B", fmt_num(n)),
        Item::Times10 => "bytes ×10".into(),
        Item::UnlockAll => "alle upgrades freischalten".into(),
        Item::LevelAll => "+1 level auf alle upgrades".into(),
        Item::SpawnEvent => "event auslösen (zufällig)".into(),
        Item::TimeWarp => format!("zeitraffer  <×{}>", info.time_scale),
        Item::Reset if info.reset_armed => "wirklich zurücksetzen? nochmal enter".into(),
        Item::Reset => "spielstand zurücksetzen".into(),
    }
}

fn panel(theme: &Theme, title: &'static str) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(2.))
        .w(px(420.))
        .flex_shrink()
        .min_w(px(0.))
        .p(px(16.))
        .bg(theme.bg)
        .border_1()
        .border_color(theme.line)
        .child(div().pb(px(10.)).child(title))
        .child(div().border_b_1().border_color(theme.line).mb(px(6.)))
}

fn stat(theme: &Theme, name: &str, value: String) -> Div {
    div()
        .flex()
        .justify_between()
        .gap(px(12.))
        .px(px(10.))
        .child(div().text_color(theme.dim).child(name.to_string()))
        .child(div().whitespace_nowrap().child(value))
}

/// Anteil als Balken aus Blockzeichen, z. B. `██████░░░░░░`.
pub fn bar(share: f64, width: usize) -> String {
    let filled = ((share.clamp(0., 1.) * width as f64).round() as usize).min(width);
    format!("{}{}", "█".repeat(filled), "░".repeat(width - filled))
}

pub fn render_admin(
    game: &Game,
    info: &AdminInfo,
    selected: usize,
    theme: &Theme,
    cx: &mut Context<ClickerApp>,
) -> Div {
    let actions: Vec<_> = ITEMS
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_sel = i == selected;
            let danger = matches!(item, Item::Reset);
            let (fg, bg) = if is_sel { (theme.bg, theme.fg) } else { (theme.fg, theme.bg) };
            div()
                .id(("admin", i))
                .flex()
                .px(px(10.))
                .py(px(5.))
                .bg(bg)
                .text_color(fg)
                .cursor_pointer()
                .when(danger && !is_sel, |d| d.text_color(theme.dim))
                .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                    if *hovered {
                        this.hover_admin(i, cx);
                    }
                }))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| this.click_admin(i, 1, cx)),
                )
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, _, _, cx| this.click_admin(i, -1, cx)),
                )
                .child(div().w(px(24.)).flex_none().child(if is_sel { ">" } else { "" }))
                .child(label(*item, info))
        })
        .collect();

    let total_rate = game.per_second();
    let mut sources: Vec<(usize, f64)> = (0..UPGRADES.len())
        .map(|i| (i, game.source_rate(i)))
        .filter(|(_, r)| *r > 0.)
        .collect();
    sources.sort_by(|a, b| b.1.total_cmp(&a.1));

    let stats = panel(theme, "STATS")
        .child(stat(theme, "bytes", fmt_num(game.whole_bytes())))
        .child(stat(theme, "verdient gesamt", fmt_num(game.total_earned as u64)))
        .child(stat(theme, "klicks gesamt", fmt_num(game.total_clicks)))
        .child(stat(theme, "B/klick", fmt_num(game.per_click())))
        .child(stat(theme, "klicks/s", format!("{:.1}", info.clicks_per_second)))
        .child(stat(theme, "B/s", trim_float(total_rate)))
        .child(stat(
            theme,
            "passiv-multiplikator",
            format!("×{}", trim_float(game.passive_multiplier())),
        ))
        .child(stat(theme, "upgrades gekauft", game.upgrades_bought().to_string()))
        .child(stat(
            theme,
            "freigeschaltet",
            format!("{}/{}", game.unlocked_count(), UPGRADES.len()),
        ))
        .child(stat(theme, "spielzeit", fmt_duration(game.played_seconds)))
        .child(stat(theme, "sitzung", fmt_duration(info.session_seconds)))
        .child(stat(theme, "pakete aktiv", info.packets.to_string()))
        .child(stat(theme, "zeitraffer", format!("×{}", info.time_scale)))
        .child(
            div()
                .pt(px(12.))
                .px(px(10.))
                .text_color(theme.dim)
                .child("einkommen nach quelle"),
        )
        .children(sources.into_iter().map(|(i, rate)| {
            div()
                .flex()
                .gap(px(10.))
                .px(px(10.))
                .child(div().w(px(110.)).flex_none().overflow_hidden().child(UPGRADES[i].id))
                .child(div().text_color(theme.dim).child(bar(rate / total_rate, 12)))
                .child(div().whitespace_nowrap().child(format!("{} B/s", trim_float(rate))))
        }));

    div()
        .flex()
        .gap(px(16.))
        .items_start()
        .max_w_full()
        .child(
            panel(theme, "ADMIN")
                .children(actions)
                .child(
                    div()
                        .pt(px(12.))
                        .text_color(theme.dim)
                        .child("# enter/klick = ausführen · ←→ = zeitraffer"),
                ),
        )
        .child(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_numbers_and_durations() {
        assert_eq!(fmt_num(0), "0");
        assert_eq!(fmt_num(999), "999");
        assert_eq!(fmt_num(1_000), "1.000");
        assert_eq!(fmt_num(1_234_567), "1.234.567");
        assert_eq!(fmt_duration(65.), "1m 05s");
        assert_eq!(fmt_duration(3725.), "1h 02m 05s");
    }

    #[test]
    fn bar_fills_by_share() {
        assert_eq!(bar(0.5, 4), "██░░");
        assert_eq!(bar(2.0, 3), "███");
    }
}
