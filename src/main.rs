mod ascii;
mod format;
mod game;
mod rng;
mod save;
mod settings;
mod theme;
mod ui;

use gpui::{App, Application, Bounds, WindowBounds, WindowOptions, prelude::*, px, size};
use std::borrow::Cow;
use ui::app::ClickerApp;
use ui::layout::{FONT_SIZE, LINE_H, char_width};

fn main() {
    // Admin-Panel nur mit `--admin` (oder `--debug`), z. B. `cargo run -- --admin`
    // oder `open "Terminal Clicker.app" --args --admin`.
    let admin = std::env::args()
        .skip(1)
        .any(|a| a == "--admin" || a == "--debug");

    Application::new().run(move |cx: &mut App| {
        cx.text_system()
            .add_fonts(vec![
                Cow::Borrowed(
                    include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf").as_slice(),
                ),
                Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf").as_slice()),
            ])
            .expect("Schrift konnte nicht geladen werden");
        ui::keys::bind(cx);

        let min_width = char_width(FONT_SIZE) * 200.;
        let min_height = LINE_H * 60.;
        let min_size = size(px(min_width), px(min_height));

        let bounds = Bounds::centered(None, min_size, cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(char_width(FONT_SIZE) * 160.), px(LINE_H * 45.))),
                    titlebar: Some(ui::title_bar::options()),
                    ..Default::default()
                },
                move |window, cx| cx.new(|cx| ClickerApp::new(window, cx, admin)),
            )
            .expect("Fenster konnte nicht geöffnet werden");
        // Ziehen und Doppelklick-Zoom macht unsere Titelleiste, nicht macOS.
        window
            .update(cx, |_, window, _| ui::window_drag::lock_native_drag(window))
            .ok();

        // Fenster zu = App zu (gespeichert wird über on_app_quit).
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.activate(true);
    });
}
