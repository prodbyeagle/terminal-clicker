mod ascii;
mod format;
mod game;
mod save;
mod theme;
mod ui;

use gpui::{
    App, Application, Bounds, KeyBinding, TitlebarOptions, WindowBounds, WindowOptions, actions,
    point, prelude::*, px, size,
};
use std::borrow::Cow;
use ui::main_view::ClickerApp;

actions!(
    clicker,
    [
        Click,
        Quit,
        OpenUpgrades,
        OpenSettings,
        OpenAdmin,
        OpenHelp,
        CollectEvent,
        Up,
        Down,
        Left,
        Right,
        Confirm,
        Back
    ]
);

fn main() {
    // Admin-Panel nur mit `--admin` (oder `--debug`) – z. B. `cargo run -- --admin`
    // oder `open "Terminal Clicker.app" --args --admin`.
    let admin = std::env::args().skip(1).any(|a| a == "--admin" || a == "--debug");

    Application::new().run(move |cx: &mut App| {
        cx.text_system()
            .add_fonts(vec![
                Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf").as_slice()),
                Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf").as_slice()),
            ])
            .expect("Schrift konnte nicht geladen werden");

        cx.bind_keys([
            KeyBinding::new("space", Click, None),
            KeyBinding::new("q", Quit, None),
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("u", OpenUpgrades, None),
            KeyBinding::new("s", OpenSettings, None),
            KeyBinding::new("a", OpenAdmin, None),
            KeyBinding::new("h", OpenHelp, None),
            KeyBinding::new("e", CollectEvent, None),
            KeyBinding::new("up", Up, None),
            KeyBinding::new("down", Down, None),
            KeyBinding::new("left", Left, None),
            KeyBinding::new("right", Right, None),
            KeyBinding::new("enter", Confirm, None),
            KeyBinding::new("escape", Back, None),
        ]);

        let bounds = Bounds::centered(None, size(px(1280.), px(800.)), cx);
        let window = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(800.), px(500.))),
                // Eigene Titelleiste: System-Leiste unsichtbar, Ampel bleibt nativ
                // und wird vertikal in unserer Leiste zentriert.
                titlebar: Some(TitlebarOptions {
                    title: Some("Terminal Clicker".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(
                        px(ui::main_view::TRAFFIC_LIGHTS_X),
                        px((ui::main_view::TITLE_BAR_H - 14.) / 2.),
                    )),
                }),
                ..Default::default()
            },
            move |window, cx| cx.new(|cx| ClickerApp::new(window, cx, admin)),
        )
        .expect("Fenster konnte nicht geöffnet werden");
        // Ziehen nur über die freien Flächen unserer Titelleiste, nicht über die Reiter.
        window
            .update(cx, |_, window, _| ui::window_drag::lock_native_drag(window))
            .ok();

        // Fenster zu = App zu (speichert über on_app_quit).
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.activate(true);
    });
}
