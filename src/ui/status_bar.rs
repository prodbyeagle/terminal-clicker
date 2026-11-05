use crate::theme::Theme;
use gpui::{Action, Div, MouseButton, div, prelude::*, px};

/// Ein Kürzel in der Statuszeile. Mit `action` ist es auch per Maus klickbar.
pub struct KeyHint {
    pub key: &'static str,
    pub label: &'static str,
    pub action: Option<Box<dyn Action>>,
}

impl KeyHint {
    pub fn new(key: &'static str, label: &'static str, action: impl Action) -> Self {
        Self {
            key,
            label,
            action: Some(Box::new(action)),
        }
    }

    /// Nur Hinweis, kein Klickziel (z. B. `↑↓ wählen`).
    pub fn info(key: &'static str, label: &'static str) -> Self {
        Self {
            key,
            label,
            action: None,
        }
    }
}

/// Statuszeile: links Werte, rechts alle Kürzel der aktuellen Ansicht.
pub fn render_status_bar(theme: &Theme, left: String, keys: Vec<KeyHint>) -> Div {
    let hover_bg = theme.line;
    div()
        .flex_none()
        .h(px(30.))
        .px(px(8.))
        .flex()
        .items_center()
        .justify_between()
        .gap(px(24.))
        .bg(theme.bg)
        .border_t_1()
        .border_color(theme.line)
        .child(
            div()
                .px(px(6.))
                .whitespace_nowrap()
                .overflow_hidden()
                .child(left),
        )
        .child(
            div()
                .flex()
                .flex_none()
                .gap(px(4.))
                .children(keys.into_iter().map(|hint| {
                    let item = div()
                        .flex()
                        .gap(px(6.))
                        .px(px(6.))
                        .whitespace_nowrap()
                        .child(hint.key)
                        .child(div().text_color(theme.dim).child(hint.label));
                    match hint.action {
                        Some(action) => item
                            .cursor_pointer()
                            .hover(move |s| s.bg(hover_bg))
                            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                cx.stop_propagation();
                                window.dispatch_action(action.boxed_clone(), cx);
                            }),
                        None => item,
                    }
                })),
        )
}
