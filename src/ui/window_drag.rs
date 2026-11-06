//! Fenster über die eigene Titelleiste verschieben.
//!
//! macOS würde ein Fenster mit transparenter Titelleiste im oberen Streifen
//! selbst verschieben – auch über unseren Reitern, sodass schon ein Klick mit
//! minimaler Mausbewegung das Fenster ein paar Pixel versetzt. Deshalb ist das
//! Fenster für AppKit grundsätzlich *nicht* verschiebbar (`isMovable = NO`),
//! gpuis View meldet `mouseDownCanMoveWindow = NO` (sonst zoomt macOS auch bei
//! einem Doppelklick auf einen Reiter), und nur unsere Titelleiste startet auf
//! ihren freien Flächen gezielt einen Drag: kurz `isMovable = YES`, dann
//! `-[NSWindow performWindowDragWithEvent:]` mit dem Mouse-Down-Event, danach
//! wieder `NO`. Den Doppelklick auf freien Flächen behandelt die Titelleiste selbst.
//!
//! `performWindowDragWithEvent:` läuft eine eigene Event-Schleife, bis die Maus
//! losgelassen wird. Damit gpui währenddessen nicht mitten in seiner
//! Event-Verarbeitung steckt (Fenster-Callbacks würden sonst auf einen belegten
//! App-Zustand treffen), wird der Aufruf auf den nächsten Main-Loop-Durchlauf
//! verschoben. gpui 0.2.2 bietet dafür auf macOS selbst nichts an.

use gpui::{App, Window};

#[cfg(target_os = "macos")]
mod mac {
    use gpui::Window;
    use objc::runtime::{NO, Object, YES};
    use objc::{msg_send, sel, sel_impl};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    pub fn ns_window(window: &Window) -> Option<*mut Object> {
        let handle = HasWindowHandle::window_handle(window).ok()?;
        let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
            return None;
        };
        let view = appkit.ns_view.as_ptr() as *mut Object;
        let ns_window: *mut Object = unsafe { msg_send![view, window] };
        (!ns_window.is_null()).then_some(ns_window)
    }

    /// `-[GPUIView mouseDownCanMoveWindow]` → NO: AppKit behandelt gpuis Fläche
    /// dann auch im Titelleisten-Streifen als „eigenen“ Inhalt (kein natives Ziehen,
    /// kein natives Doppelklick-Zoomen).
    pub fn view_cannot_move_window() {
        use objc::runtime::{BOOL, Class, Imp, Sel, class_addMethod};

        extern "C" fn cannot_move_window(_: &Object, _: Sel) -> BOOL {
            NO
        }

        let Some(class) = Class::get("GPUIView") else {
            return;
        };
        // Typ-Kodierung: BOOL, self (@), _cmd (:). Auf Apple Silicon ist BOOL ein `bool` (B).
        let types = if cfg!(target_arch = "aarch64") {
            c"B@:"
        } else {
            c"c@:"
        };
        unsafe {
            let imp: extern "C" fn(&Object, Sel) -> BOOL = cannot_move_window;
            class_addMethod(
                class as *const Class as *mut Class,
                sel!(mouseDownCanMoveWindow),
                std::mem::transmute::<extern "C" fn(&Object, Sel) -> BOOL, Imp>(imp),
                types.as_ptr(),
            );
        }
    }

    pub unsafe fn set_movable(ns_window: *mut Object, movable: bool) {
        let flag = if movable { YES } else { NO };
        let _: () = unsafe { msg_send![ns_window, setMovable: flag] };
    }
}

/// Natives Ziehen und Doppelklick-Zoomen von macOS abschalten.
/// Einmal nach dem Öffnen des Fensters aufrufen.
#[cfg(target_os = "macos")]
pub fn lock_native_drag(window: &Window) {
    mac::view_cannot_move_window();
    if let Some(ns_window) = mac::ns_window(window) {
        unsafe { mac::set_movable(ns_window, false) };
    }
}

/// Startet einen Fenster-Drag aus dem aktuellen Mouse-Down heraus.
#[cfg(target_os = "macos")]
pub fn begin_window_drag(window: &mut Window, cx: &mut App) {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};

    let Some(ns_window) = mac::ns_window(window) else {
        return;
    };

    // Fenster und aktuelles Event jetzt greifen und festhalten (retain),
    // dann im nächsten Main-Loop-Durchlauf ziehen und wieder freigeben.
    let (ns_window, event) = unsafe {
        let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
        let event: *mut Object = msg_send![app, currentEvent];
        if event.is_null() {
            return;
        }
        let _: *mut Object = msg_send![ns_window, retain];
        let _: *mut Object = msg_send![event, retain];
        (ns_window as usize, event as usize)
    };

    window
        .spawn(cx, async move |_| unsafe {
            let ns_window = ns_window as *mut Object;
            let event = event as *mut Object;
            // Nur ziehen, wenn die linke Maustaste noch gedrückt ist. Sonst würde
            // die Drag-Schleife auf den *nächsten* Klick warten und ihn schlucken
            // (ein schneller zweiter Klick würde dann z. B. das Fenster zoomen).
            let pressed: usize = msg_send![class!(NSEvent), pressedMouseButtons];
            if pressed & 1 == 1 {
                mac::set_movable(ns_window, true);
                let _: () = msg_send![ns_window, performWindowDragWithEvent: event];
                mac::set_movable(ns_window, false);
            }
            let _: () = msg_send![event, release];
            let _: () = msg_send![ns_window, release];
        })
        .detach();
}

#[cfg(not(target_os = "macos"))]
pub fn lock_native_drag(_window: &Window) {}

#[cfg(not(target_os = "macos"))]
pub fn begin_window_drag(window: &mut Window, _cx: &mut App) {
    window.start_window_move();
}
