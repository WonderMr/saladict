use std::fs;
use crate::config::get;
use crate::config::set;
use crate::StringWrapper;
use crate::APP;
use dirs::cache_dir;
use log::{info, warn};
use tauri::Emitter;
use tauri::Listener;
use tauri::Manager;
use tauri::Monitor;
use tauri::WebviewWindow;
use tauri::WebviewWindowBuilder;
use tauri::WebviewUrl;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use window_shadows::set_shadow;
use tauri::{LogicalPosition, PhysicalPosition};
#[cfg(target_os = "macos")]
use cocoa::appkit::NSWindow;
use mouse_position::mouse_position::Mouse;
use serde_json;

// Returns true only when the current process is actually talking to a Wayland
// compositor. Several Tauri builder / window operations (visible(false),
// skip_taskbar, .position(), set_skip_taskbar after build, center) trigger
// Wayland Protocol Error 71 or are no-ops on Wayland, but work fine on X11
// / XWayland / macOS / Windows. The dev shell pins GDK_BACKEND=x11 to force
// XWayland even when WAYLAND_DISPLAY is set, so the explicit backend
// override wins.
#[cfg(target_os = "linux")]
fn is_wayland_session() -> bool {
    let wayland_available = std::env::var("WAYLAND_DISPLAY")
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    let x11_available = std::env::var("DISPLAY")
        .map(|v| !v.is_empty())
        .unwrap_or(false);

    if let Ok(backend) = std::env::var("GDK_BACKEND") {
        // GDK_BACKEND is a comma-separated priority list (e.g. "wayland,x11"
        // means "prefer Wayland, fall back to X11"). GDK walks it and picks
        // the first entry whose display server is actually reachable, so we
        // need to do the same to match the real selection: pair each entry
        // with its corresponding display env var and return the first viable
        // match. This correctly handles "x11,wayland" with no DISPLAY set
        // (GTK falls back to Wayland) and "wayland,x11" with no
        // WAYLAND_DISPLAY set (GTK falls back to X11).
        for entry in backend.split(',').map(|s| s.trim().to_ascii_lowercase()) {
            match entry.as_str() {
                "x11" if x11_available => return false,
                "wayland" if wayland_available => return true,
                _ => {}
            }
        }
    }
    // No GDK_BACKEND override, or none of the listed backends could connect:
    // default to whichever display server is actually present.
    wayland_available
}

#[cfg(not(target_os = "linux"))]
fn is_wayland_session() -> bool {
    false
}

pub const THUMB_WIN_NAME: &str = "thumb";// Get daemon window instance
fn get_daemon_window() -> WebviewWindow {
    let app_handle = APP.get().unwrap();
    match app_handle.get_webview_window("daemon") {
        Some(v) => v,
        None => {
            warn!("Daemon window not found, create new daemon window!");
            let wayland = is_wayland_session();
            let mut builder = WebviewWindowBuilder::new(
                app_handle,
                "daemon",
                WebviewUrl::App("daemon.html".into()),
            )
            .title("Daemon");
            // visible(false) at build time trips Wayland Protocol Error 71;
            // on Wayland we build visible then hide immediately instead.
            if !wayland {
                builder = builder.visible(false);
            }
            // Build failure here only happens when the compositor / window
            // system rejects the creation outright — there's no meaningful
            // recovery path for a monitor-probe helper, so the descriptive
            // expect gives a clear message if it ever panics.
            let window = builder
                .build()
                .expect("get_daemon_window: failed to build daemon window");
            if wayland {
                if let Err(e) = window.hide() {
                    warn!("get_daemon_window: hide() failed: {:?}", e);
                }
            }
            window
        }
    }
}

// Get monitor where the mouse is currently located
fn get_current_monitor(x: i32, y: i32) -> Option<Monitor> {
    log::debug!("Mouse position: {}, {}", x, y);
    let daemon_window = get_daemon_window();
    let monitors = match daemon_window.available_monitors() {
        Ok(m) => m,
        Err(e) => {
            warn!("available_monitors failed: {:?}", e);
            return daemon_window.primary_monitor().ok().flatten();
        }
    };

    for m in monitors {
        let size = m.size();
        let position = m.position();

        if x >= position.x
            && x <= (position.x + size.width as i32)
            && y >= position.y
            && y <= (position.y + size.height as i32)
        {
            log::debug!("Current Monitor: {:?}", m);
            return Some(m);
        }
    }
    warn!("Current Monitor not found, using primary monitor");
    match daemon_window.primary_monitor() {
        Ok(m) => m,
        Err(e) => {
            warn!("primary_monitor failed: {:?}", e);
            None
        }
    }
}

// Creating a window on the mouse monitor
fn build_window(label: &str, title: &str) -> (WebviewWindow, bool) {
    let app_handle = APP.get().unwrap();
    match app_handle.get_webview_window(label) {
        Some(v) => {
            log::debug!("Window existence: {}", label);
            // show() is intentionally not called here — callers may re-center
            // or re-size before the window becomes visible.
            // unminimize() first so set_focus() (and any subsequent show())
            // actually restores the window when the user had minimized it.
            // Without this, a minimized + skip_taskbar window is unreachable
            // (no taskbar entry to click, set_focus is a no-op).
            if let Err(e) = v.unminimize() {
                warn!("build_window: unminimize() failed for {}: {:?}", label, e);
            }
            let _ = v.set_focus();
            (v, true)
        }
        None => {
            log::debug!("Window not existence, Creating new window: {}", label);
            let mut builder = WebviewWindowBuilder::new(
                app_handle,
                label,
                WebviewUrl::App("index.html".into()),
            )
            .focused(true)
            .title(title);

            // Wayland rejects position/visible(false)/skip_taskbar at build
            // time (Protocol Error 71). X11/XWayland/macOS/Windows accept
            // them, so we gate at runtime rather than by compile-time cfg.
            let wayland = is_wayland_session();
            if !wayland {
                use mouse_position::mouse_position::{Mouse, Position};
                let mouse_position = match Mouse::get_mouse_position() {
                    Mouse::Position { x, y } => Position { x, y },
                    Mouse::Error => {
                        warn!("Mouse position not found, using (0, 0) as default");
                        Position { x: 0, y: 0 }
                    }
                };
                let hide_dock_icon = get("hide_dock_icon")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                builder = builder.visible(false).skip_taskbar(hide_dock_icon);
                if let Some(monitor) = get_current_monitor(mouse_position.x, mouse_position.y) {
                    let position = monitor.position();
                    builder = builder.position(position.x.into(), position.y.into());
                }
            }

            #[cfg(target_os = "macos")]
            {
                builder = builder
                    .title_bar_style(tauri::TitleBarStyle::Overlay)
                    .hidden_title(true);
            }
            #[cfg(not(target_os = "macos"))]
            {
                builder = builder.transparent(true).decorations(false);
            }
            // If the window system rejects window creation there's no
            // recoverable path for a feature window — the translate /
            // config / recognize / updater / notify / screenshot screens
            // all need a backing Tauri window to function. Panicking with
            // a clear message is more diagnostic than silently returning
            // a dummy handle that would fail on every subsequent call.
            let window = builder
                .build()
                .unwrap_or_else(|e| panic!("build_window: failed to build '{}': {:?}", label, e));

            if label != "screenshot" {
                #[cfg(any(target_os = "macos", target_os = "windows"))]
                if let Err(e) = set_shadow(&window, true) {
                    warn!("build_window: set_shadow failed for '{}': {:?}", label, e);
                }
            }
            // show() is intentionally not called here — callers apply sizing,
            // positioning, and centering, and then show the window themselves
            // so it doesn't flash at (0, 0) with default size. On Wayland we
            // couldn't use visible(false) at builder time, so hide now to
            // restore the same "invisible until caller shows" contract.
            if wayland {
                if let Err(e) = window.hide() {
                    warn!("build_window: post-build hide() failed for '{}': {:?}", label, e);
                }
            } else {
                let _ = window.current_monitor();
            }
            (window, false)
        }
    }
}

pub fn config_window() {
    let (window, _exists) = build_window("config", "Config");
    if let Err(e) = window.set_min_size(Some(tauri::LogicalSize::new(800, 400))) {
        warn!("config_window: set_min_size failed: {:?}", e);
    }
    if let Err(e) = window.set_size(tauri::LogicalSize::new(800, 600)) {
        warn!("config_window: set_size failed: {:?}", e);
    }
    if !is_wayland_session() {
        if let Err(e) = window.center() {
            warn!("config_window: center() failed: {:?}", e);
        }
    }
    if let Err(e) = window.show() {
        warn!("config_window: show() failed: {:?}", e);
    }
}

pub fn translate_window() -> WebviewWindow {
    use mouse_position::mouse_position::{Mouse, Position};
    // Mouse physical position
    let mut mouse_position = match Mouse::get_mouse_position() {
        Mouse::Position { x, y } => Position { x, y },
        Mouse::Error => {
            warn!("Mouse position not found, using (0, 0) as default");
            Position { x: 0, y: 0 }
        }
    };
    let (window, exists) = build_window("translate", "Translate");
    // Re-applying size/position to an already-visible existing window would
    // make it visibly jump to the current mouse cursor on every text
    // selection (or snap to a different size after a config edit). The
    // trade-off: a translate_window_width/height config change only takes
    // effect after the user closes and re-opens the window. This matches
    // the pre-PR behavior.
    if exists {
        return window;
    }
    // Wayland Protocol Error 71: skip_taskbar is not supported there.
    if !is_wayland_session() {
        if let Err(e) = window.set_skip_taskbar(true) {
            warn!("translate_window: set_skip_taskbar failed: {:?}", e);
        }
    }
    // Get Translate Window Size
    let width = match get("translate_window_width") {
        Some(v) => v.as_i64().unwrap(),
        None => {
            set("translate_window_width", 350);
            350
        }
    };
    let height = match get("translate_window_height") {
        Some(v) => v.as_i64().unwrap(),
        None => {
            set("translate_window_height", 420);
            420
        }
    };

    let monitor = match window.current_monitor() {
        Ok(m) => m,
        Err(e) => {
            warn!("translate_window: current_monitor failed: {:?}", e);
            None
        }
    };
    let dpi = monitor.as_ref().map(|m| m.scale_factor()).unwrap_or(1.0);

    if let Err(e) = window.set_size(tauri::PhysicalSize::new(
        (width as f64) * dpi,
        (height as f64) * dpi,
    )) {
        warn!("translate_window: set_size failed: {:?}", e);
    }

    let position_type = match get("translate_window_position") {
        Some(v) => v.as_str().unwrap().to_string(),
        None => "mouse".to_string(),
    };

    match position_type.as_str() {
        "mouse" => {
            // Clamp against monitor bounds when the monitor is known.
            if let Some(monitor) = monitor.as_ref() {
                let monitor_size = monitor.size();
                let monitor_size_width = monitor_size.width as f64;
                let monitor_size_height = monitor_size.height as f64;
                let monitor_position = monitor.position();
                let monitor_position_x = monitor_position.x as f64;
                let monitor_position_y = monitor_position.y as f64;

                if mouse_position.x as f64 + width as f64 * dpi
                    > monitor_position_x + monitor_size_width
                {
                    mouse_position.x -= (width as f64 * dpi) as i32;
                    if (mouse_position.x as f64) < monitor_position_x {
                        mouse_position.x = monitor_position_x as i32;
                    }
                }
                if mouse_position.y as f64 + height as f64 * dpi
                    > monitor_position_y + monitor_size_height
                {
                    mouse_position.y -= (height as f64 * dpi) as i32;
                    if (mouse_position.y as f64) < monitor_position_y {
                        mouse_position.y = monitor_position_y as i32;
                    }
                }
            }

            if !is_wayland_session() {
                if let Err(e) = window.set_position(tauri::PhysicalPosition::new(
                    mouse_position.x,
                    mouse_position.y,
                )) {
                    warn!("translate_window: set_position failed: {:?}", e);
                }
            }
        }
        _ => {
            // translate_window_position is "pre_state" (or any non-"mouse"
            // value). Use the saved x/y, but only if both are actually in
            // the config — otherwise the user just enabled pre_state and
            // hasn't moved the window yet, so fall back to mouse positioning
            // instead of pinning the window to (0, 0) of the primary monitor.
            let saved = get("translate_window_position_x")
                .and_then(|v| v.as_i64())
                .and_then(|x| {
                    get("translate_window_position_y")
                        .and_then(|v| v.as_i64())
                        .map(|y| (x, y))
                });
            let (position_x, position_y) = match saved {
                Some(xy) => xy,
                None => {
                    log::debug!(
                        "translate_window: no saved pre_state position yet; \
                         falling back to mouse cursor"
                    );
                    (mouse_position.x as i64, mouse_position.y as i64)
                }
            };
            if !is_wayland_session() {
                if let Err(e) = window.set_position(tauri::PhysicalPosition::new(
                    (position_x as f64) * dpi,
                    (position_y as f64) * dpi,
                )) {
                    warn!("translate_window: set_position failed: {:?}", e);
                }
            }
        }
    }

    // show() is intentionally not called here — callers (e.g. input_translate)
    // may re-center the window and must do so before it becomes visible to
    // avoid a flash from the mouse position to the screen center.
    window
}

pub fn selection_translate() {
    let text = capture_selected_text();
    let trimmed = text.trim();

    if !trimmed.is_empty() {
        let app_handle = APP.get().unwrap();
        let state: tauri::State<StringWrapper> = app_handle.state();
        // Always persist / emit the trimmed text so a stray leading/trailing
        // newline from the clipboard doesn't surface as a literal "\n" in
        // the translate window.
        state
            .0
            .lock()
            .unwrap()
            .replace_range(.., trimmed);
    } else {
        warn!(
            "selection_translate: no text captured (XDG_SESSION_TYPE={:?}, \
             WAYLAND_DISPLAY={:?}, GDK_BACKEND={:?}). Highlight text, then \
             trigger the hotkey — on some Wayland compositors PRIMARY \
             selection is not mirrored to XWayland, so try Ctrl+C first to \
             fall back to the system clipboard.",
            std::env::var("XDG_SESSION_TYPE").ok(),
            std::env::var("WAYLAND_DISPLAY").ok(),
            std::env::var("GDK_BACKEND").ok(),
        );
    }

    let window = translate_window();
    // Visibility is handled by the JS handleNewText listener in
    // Translate/components/SourceArea — it reads translate_hide_window and
    // calls appWindow.show()/hide() + setFocus() itself.
    if let Err(e) = window.emit("new_text", trimmed) {
        warn!("selection_translate: emit(new_text) failed: {:?}", e);
    }
}

// Try to grab the user's currently-highlighted text. On KDE/GNOME Wayland
// running saladict under XWayland (GDK_BACKEND=x11) the `selection` crate's
// PRIMARY read can come back empty if the source app wrote to the Wayland
// primary buffer instead of the X11 one, so we fall back to the cached text
// captured by mouse_hook at the moment the user released the mouse.
//
// The system CLIPBOARD is intentionally NOT a fallback here — on sessions
// where primary selection doesn't bridge correctly it would cause every
// selection_translate invocation to silently paste whatever was last Ctrl+C'd,
// which is both surprising and wrong. Users who explicitly want "translate
// whatever is on my clipboard" can Ctrl+C and use a dedicated flow instead.
fn capture_selected_text() -> String {
    let primary = selection::get_text();
    if !primary.trim().is_empty() {
        return primary;
    }
    log::debug!("capture_selected_text: primary selection empty, trying mouse_hook cache");
    let cached = crate::mouse_hook::SELECTED_TEXT.lock().clone();
    if !cached.trim().is_empty() {
        return cached;
    }
    String::new()
}

pub fn input_translate() {
    let app_handle = APP.get().unwrap();
    // Clear State
    let state: tauri::State<StringWrapper> = app_handle.state();
    state
        .0
        .lock()
        .unwrap()
        .replace_range(.., "[INPUT_TRANSLATE]");
    let window = translate_window();
    if !is_wayland_session() {
        let position_type = match get("translate_window_position") {
            Some(v) => v.as_str().unwrap().to_string(),
            None => "mouse".to_string(),
        };
        if position_type == "mouse" {
            if let Err(e) = window.center() {
                warn!("input_translate: center() failed: {:?}", e);
            }
        }
    }
    // Visibility is handled by the JS handleNewText listener — it always
    // show()s on [INPUT_TRANSLATE] regardless of translate_hide_window.
    if let Err(e) = window.emit("new_text", "[INPUT_TRANSLATE]") {
        warn!("input_translate: emit(new_text) failed: {:?}", e);
    }
}

pub fn text_translate(text: String) {
    // Strip leading/trailing whitespace so a lone "\n" captured by the
    // mouse_hook thumb path doesn't become the translate-window content.
    let trimmed = text.trim();
    let app_handle = APP.get().unwrap();
    let state: tauri::State<StringWrapper> = app_handle.state();
    state.0.lock().unwrap().replace_range(.., trimmed);
    let window = translate_window();
    // Visibility is handled by the JS handleNewText listener (see
    // selection_translate for details) — avoid a flash when the user has
    // translate_hide_window enabled.
    if let Err(e) = window.emit("new_text", trimmed) {
        warn!("text_translate: emit(new_text) failed: {:?}", e);
    }
}

pub fn image_translate() {
    let app_handle = APP.get().unwrap();
    let state: tauri::State<StringWrapper> = app_handle.state();
    state
        .0
        .lock()
        .unwrap()
        .replace_range(.., "[IMAGE_TRANSLATE]");
    let window = translate_window();
    // Visibility is handled by the JS handleNewText listener (see
    // selection_translate for details).
    if let Err(e) = window.emit("new_text", "[IMAGE_TRANSLATE]") {
        warn!("image_translate: emit(new_text) failed: {:?}", e);
    }
}

pub fn recognize_window() {
    let (window, exists) = build_window("recognize", "Recognize");
    if exists {
        if let Err(e) = window.show() {
            warn!("recognize_window: show() failed: {:?}", e);
        }
        if let Err(e) = window.emit("new_image", "") {
            warn!("recognize_window: emit(new_image) failed: {:?}", e);
        }
        return;
    }
    let width = match get("recognize_window_width") {
        Some(v) => v.as_i64().unwrap(),
        None => {
            set("recognize_window_width", 800);
            800
        }
    };
    let height = match get("recognize_window_height") {
        Some(v) => v.as_i64().unwrap(),
        None => {
            set("recognize_window_height", 400);
            400
        }
    };
    let dpi = match window.current_monitor() {
        Ok(Some(m)) => m.scale_factor(),
        Ok(None) => 1.0,
        Err(e) => {
            warn!("recognize_window: current_monitor failed: {:?}", e);
            1.0
        }
    };
    if let Err(e) = window.set_size(tauri::PhysicalSize::new(
        (width as f64) * dpi,
        (height as f64) * dpi,
    )) {
        warn!("recognize_window: set_size failed: {:?}", e);
    }
    if !is_wayland_session() {
        if let Err(e) = window.center() {
            warn!("recognize_window: center() failed: {:?}", e);
        }
    }
    if let Err(e) = window.show() {
        warn!("recognize_window: show() failed: {:?}", e);
    }
    if let Err(e) = window.emit("new_image", "") {
        warn!("recognize_window: emit(new_image) failed: {:?}", e);
    }
}

#[cfg(not(target_os = "macos"))]
fn screenshot_window() -> WebviewWindow {
    let (window, _exists) = build_window("screenshot", "Screenshot");

    // The window is already hidden at this point: on X11/Windows/macOS because
    // build_window applied visible(false); on Wayland because build_window
    // explicitly called hide() after the build step. This keeps the subsequent
    // screen.capture() from capturing the blank fullscreen screenshot window
    // itself. The React Screenshot component calls appWindow.show() in its
    // img onLoad handler once the captured image is ready.

    // Wayland Protocol Error 71: skip_taskbar is not supported there.
    if !is_wayland_session() {
        if let Err(e) = window.set_skip_taskbar(true) {
            warn!("screenshot_window: set_skip_taskbar failed: {:?}", e);
        }
    }
    // macOS uses its own /usr/sbin/screencapture pipeline in ocr_recognize /
    // ocr_translate instead of this screenshot window, so the function itself
    // is cfg-gated to non-macOS targets; no macOS-specific branch needed here.
    if let Err(e) = window.set_fullscreen(true) {
        warn!("screenshot_window: set_fullscreen failed: {:?}", e);
    }

    if let Err(e) = window.set_always_on_top(true) {
        warn!("screenshot_window: set_always_on_top failed: {:?}", e);
    }
    // show() is intentionally not called here — the React Screenshot
    // component calls appWindow.show() in its img onLoad handler once
    // pot_screenshot.png is loaded, to avoid a blank/white flash.
    window
}

pub fn ocr_recognize() {
    #[cfg(target_os = "macos")]
    {
        use dirs::cache_dir;

        let app_handle = APP.get().unwrap();
        let mut app_cache_dir_path = cache_dir().expect("Get Cache Dir Failed");
        app_cache_dir_path.push(&app_handle.config().identifier);
        if !app_cache_dir_path.exists() {
            fs::create_dir_all(&app_cache_dir_path).expect("Create Cache Dir Failed");
        }
        app_cache_dir_path.push("pot_screenshot_cut.png");

        let path = app_cache_dir_path.to_string_lossy().replace("\\\\?\\", "");
        println!("Screenshot path: {}", path);
        if let Ok(_output) = std::process::Command::new("/usr/sbin/screencapture")
            .arg("-i")
            .arg("-r")
            .arg(path)
            .output()
        {
            recognize_window();
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let window = screenshot_window();
        let window_ = window.clone();
        window.listen("success", move |event| {
            recognize_window();
            window_.unlisten(event.id())
        });
    }
}
pub fn ocr_translate() {
    #[cfg(target_os = "macos")]
    {
        let app_handle = APP.get().unwrap();
        let mut app_cache_dir_path = cache_dir().expect("Get Cache Dir Failed");
        app_cache_dir_path.push(&app_handle.config().identifier);
        if !app_cache_dir_path.exists() {
            fs::create_dir_all(&app_cache_dir_path).expect("Create Cache Dir Failed");
        }
        app_cache_dir_path.push("pot_screenshot_cut.png");

        let path = app_cache_dir_path.to_string_lossy().replace("\\\\?\\", "");
        println!("Screenshot path: {}", path);
        if let Ok(_output) = std::process::Command::new("/usr/sbin/screencapture")
            .arg("-i")
            .arg("-r")
            .arg(path)
            .output()
        {
            image_translate();
            ();
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let window = screenshot_window();
        let window_ = window.clone();
        window.listen("success", move |event| {
            image_translate();
            window_.unlisten(event.id())
        });
    }
}

#[tauri::command(async)]
pub fn updater_window() {
    let (window, _exists) = build_window("updater", "Updater");
    if let Err(e) = window.set_min_size(Some(tauri::LogicalSize::new(600, 400))) {
        warn!("updater_window: set_min_size failed: {:?}", e);
    }
    if let Err(e) = window.set_size(tauri::LogicalSize::new(600, 400)) {
        warn!("updater_window: set_size failed: {:?}", e);
    }
    if !is_wayland_session() {
        if let Err(e) = window.center() {
            warn!("updater_window: center() failed: {:?}", e);
        }
    }
    if let Err(e) = window.show() {
        warn!("updater_window: show() failed: {:?}", e);
    }
}

pub fn delete_thumb() {
    match APP.get() {
        Some(handle) => match handle.get_webview_window(THUMB_WIN_NAME) {
            Some(window) => {
                if let Err(e) = window.close() {
                    warn!("delete_thumb: close() failed: {:?}", e);
                }
            }
            None => {}
        },
        None => {}
    }
}

pub fn close_thumb() {
    match APP.get() {
        Some(handle) => match handle.get_webview_window(THUMB_WIN_NAME) {
            Some(window) => {
                // set_position is rejected on Wayland; rely on hide() alone
                // there and skip moving the window off-screen.
                if !is_wayland_session() {
                    if let Err(e) = window.set_position(LogicalPosition::new(-100.0, -100.0)) {
                        warn!("close_thumb: set_position failed: {:?}", e);
                    }
                }
                if let Err(e) = window.set_always_on_top(false) {
                    warn!("close_thumb: set_always_on_top failed: {:?}", e);
                }
                if let Err(e) = window.hide() {
                    warn!("close_thumb: hide() failed: {:?}", e);
                }
            }
            None => {}
        },
        None => {}
    }
}

pub fn show_thumb(x: i32, y: i32) {
    let window = get_thumb_window(x, y);
    if let Err(e) = window.show() {
        warn!("show_thumb: show() failed: {:?}", e);
    }
}

pub fn get_thumb_window(x: i32, y: i32) -> WebviewWindow {
    let handle = APP.get().unwrap();
    let position_offset = 7.0 as f64;
    let window = match handle.get_webview_window(THUMB_WIN_NAME) {
        Some(window) => {
            info!("Thumb window already exists");
            if let Err(e) = window.unminimize() {
                warn!("get_thumb_window: unminimize failed: {:?}", e);
            }
            if let Err(e) = window.set_always_on_top(true) {
                warn!("get_thumb_window: set_always_on_top failed: {:?}", e);
            }
            window
        }
        None => {
            info!("Thumb window does not exist");

            #[cfg(any(target_os = "macos", target_os = "linux"))]
            let window = {
                let mut builder = WebviewWindowBuilder::new(
                    handle,
                    THUMB_WIN_NAME,
                    WebviewUrl::App("index.html".into()),
                )
                .fullscreen(false)
                .focused(false)
                .inner_size(20.0, 20.0)
                .min_inner_size(20.0, 20.0)
                .max_inner_size(20.0, 20.0)
                .resizable(false)
                .closable(false)
                .decorations(false);

                // Wayland rejects visible(false)/skip_taskbar at build time
                // (Protocol Error 71). Apply them on X11/macOS/Windows; on
                // Wayland we hide the window post-build instead.
                let wayland = is_wayland_session();
                if !wayland {
                    builder = builder.visible(false).skip_taskbar(true);
                }

                #[cfg(not(feature = "app-store"))]
                {
                    builder = builder.transparent(true);
                }
                // The thumb window is a tiny 20x20 companion popup; if the
                // compositor denies creation there's nothing meaningful to
                // fall back to for show_thumb / close_thumb callers. A
                // descriptive expect is more diagnostic than silently
                // returning a dummy handle downstream callers would deref.
                let window = builder
                    .build()
                    .expect("get_thumb_window: failed to build thumb window");
                if wayland {
                    if let Err(e) = window.hide() {
                        warn!("get_thumb_window: post-build hide() failed: {:?}", e);
                    }
                }
                window
            };

            #[cfg(target_os = "windows")]
            let window = {
                let window = build_window(THUMB_WIN_NAME, THUMB_WIN_NAME).0;
                if let Err(e) = set_shadow(&window, false) {
                    warn!("get_thumb_window: set_shadow failed: {:?}", e);
                }
                if let Err(e) = window.set_resizable(false) {
                    warn!("get_thumb_window: set_resizable failed: {:?}", e);
                }
                if let Err(e) = window.set_skip_taskbar(true) {
                    warn!("get_thumb_window: set_skip_taskbar failed: {:?}", e);
                }
                if let Err(e) = window.set_size(tauri::LogicalSize {
                    width: 20.0,
                    height: 20.0,
                }) {
                    warn!("get_thumb_window: set_size failed: {:?}", e);
                }
                window
            };

            post_process_window(&window);
            if let Err(e) = window.unminimize() {
                warn!("get_thumb_window: unminimize failed: {:?}", e);
            }
            if let Err(e) = window.set_always_on_top(true) {
                warn!("get_thumb_window: set_always_on_top failed: {:?}", e);
            }
            window
        }
    };

    // set_position is Wayland-problematic; on Wayland the compositor picks
    // placement and this call would just log noise on every selection.
    if !is_wayland_session() {
        let set_position_result = if cfg!(target_os = "macos") {
            window.set_position(LogicalPosition::new(
                x as f64 + position_offset,
                y as f64 + position_offset,
            ))
        } else {
            window.set_position(PhysicalPosition::new(
                x as f64 + position_offset,
                y as f64 + position_offset,
            ))
        };
        if let Err(e) = set_position_result {
            warn!("get_thumb_window: set_position failed: {:?}", e);
        }
    }

    window
}

pub fn post_process_window(window: &WebviewWindow) {
    let _ = window.current_monitor();

    #[cfg(target_os = "macos")]
    {
        use cocoa::appkit::NSWindowCollectionBehavior;
        use cocoa::base::id;

        let ns_win = window.ns_window().unwrap() as id;

        unsafe {
            // Disable the automatic creation of "Show Tab Bar" etc menu items on macOS
            NSWindow::setAllowsAutomaticWindowTabbing_(ns_win, cocoa::base::NO);

            let mut collection_behavior = ns_win.collectionBehavior();
            collection_behavior |=
                NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces;

            ns_win.setCollectionBehavior_(collection_behavior);
        }
    }
}

pub fn get_mouse_location() -> Result<(i32, i32), String> {
    let position = Mouse::get_mouse_position();
    match position {
        Mouse::Position { x, y } => Ok((x, y)),
        Mouse::Error => Err("Error getting mouse position".to_string()),
    }
}

pub fn notify_window(content: &str) {
    let app_handle = APP.get().unwrap();

    // Persist the content so the notify window can pick it up on load.
    // I/O failures (missing config dir, read-only filesystem, permission
    // errors) are logged but not fatal — we still want to show the window.
    match app_handle.path().app_config_dir() {
        Ok(app_dir) => {
            // app_config_dir can return a path that doesn't exist yet on a
            // fresh install; create it before writing so we don't fail with
            // NotFound and render an empty notification.
            if let Err(e) = std::fs::create_dir_all(&app_dir) {
                warn!(
                    "notify_window: failed to create {}: {:?}",
                    app_dir.display(),
                    e
                );
            } else {
                let notify_file = app_dir.join("notify_content.json");
                let content_json = serde_json::json!({ "content": content });
                if let Err(e) = std::fs::write(&notify_file, content_json.to_string()) {
                    warn!(
                        "notify_window: failed to write {}: {:?}",
                        notify_file.display(),
                        e
                    );
                }
            }
        }
        Err(e) => {
            warn!(
                "notify_window: app_config_dir unavailable, skipping content persistence: {:?}",
                e
            );
        }
    }

    let window: WebviewWindow = match app_handle.get_webview_window("notify") {
        Some(v) => {
            info!("Notification window exists");
            if let Err(e) = v.set_focus() {
                warn!("notify_window: set_focus failed: {:?}", e);
            }
            v
        }
        None => {
            info!("Creating new notification window");
            build_window("notify", "Notification").0
        }
    };

    if let Err(e) = window.set_size(tauri::LogicalSize::new(400, 400)) {
        warn!("notify_window: set_size failed: {:?}", e);
    }
    if !is_wayland_session() {
        if let Err(e) = window.center() {
            warn!("notify_window: center() failed: {:?}", e);
        }
    }
    if let Err(e) = window.set_maximizable(false) {
        warn!("notify_window: set_maximizable failed: {:?}", e);
    }
    if let Err(e) = window.set_minimizable(false) {
        warn!("notify_window: set_minimizable failed: {:?}", e);
    }
    if let Err(e) = window.set_always_on_top(true) {
        warn!("notify_window: set_always_on_top failed: {:?}", e);
    }
    if let Err(e) = window.show() {
        warn!("notify_window: show() failed: {:?}", e);
    }
}
