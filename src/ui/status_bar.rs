use crate::theme::Theme;
use crate::ui::keys;
use crate::ui::msg::{Emit, Msg, Screen};
use gpui::{Div, MouseButton, div, prelude::*, px};

const STATUS_BAR_H: f32 = 30.;

pub struct Hint {
    keys: String,
    label: &'static str,
    msg: Option<Msg>,
}

impl Hint {
    pub fn new(msg: Msg, label: &'static str) -> Self {
        Self {
            keys: keys::label(&[msg]),
            label,
            msg: Some(msg),
        }
    }

    /// Wechsel zu einer Ansicht, beschriftet mit ihrem Namen.
    pub fn open(screen: Screen) -> Self {
        Self::new(Msg::Open(screen), screen.name())
    }

    pub fn info(msgs: &[Msg], label: &'static str) -> Self {
        Self {
            keys: keys::label(msgs),
            label,
            msg: None,
        }
    }
}

pub fn render_status_bar(theme: &Theme, left: String, hints: Vec<Hint>, emit: &Emit) -> Div {
    let hover_bg = theme.line;
    div()
        .flex_none()
        .h(px(STATUS_BAR_H))
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
                .children(hints.into_iter().map(|hint| {
                    let item = div()
                        .flex()
                        .gap(px(6.))
                        .px(px(6.))
                        .whitespace_nowrap()
                        .child(hint.keys)
                        .child(div().text_color(theme.dim).child(hint.label));
                    match hint.msg {
                        Some(msg) => item
                            .cursor_pointer()
                            .hover(move |s| s.bg(hover_bg))
                            .on_mouse_down(MouseButton::Left, emit.on_press(msg)),
                        None => item,
                    }
                })),
        )
}
