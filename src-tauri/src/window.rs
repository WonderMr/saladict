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
    if let Ok(backend) = std::env::var("GDK_BACKEND") {
        // GDK_BACKEND is a comma-separated priority list (e.g. "wayland,x11"
        // means "prefer Wayland, fall back to X11"). GDK picks the first entry
        // that's available, so only the primary selector should classify the
        // session — substring matching would misread "wayland,x11" as X11.
        let primary = backend
            .split(',')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        match primary.as_str() {
            "x11" => return false,
            "wayland" => return true,
            _ => {}
        }
    }
    std::env::var("WAYLAND_DISPLAY")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
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
            let window = builder.build().unwrap();
            if wayland {
                let _ = window.hide();
            }
            window
        }
    }
}

// Get monitor where the mouse is currently located
fn get_current_monitor(x: i32, y: i32) -> Option<Monitor> {
    info!("Mouse position: {}, {}", x, y);
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
            info!("Current Monitor: {:?}", m);
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
            info!("Window existence: {}", label);
            // show() is intentionally not called here — callers may re-center
            // or re-size before the window becomes visible. set_focus stays
            // because it doesn't affect position and brings the window to front.
            let _ = v.set_focus();
            (v, true)
        }
        None => {
            info!("Window not existence, Creating new window: {}", label);
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
            let window = builder.build().unwrap();

            if label != "screenshot" {
                #[cfg(any(target_os = "macos", target_os = "windows"))]
                set_shadow(&window, true).unwrap_or_default();
            }
            // show() is intentionally not called here — callers apply sizing,
            // positioning, and centering, and then show the window themselves
            // so it doesn't flash at (0, 0) with default size. On Wayland we
            // couldn't use visible(false) at builder time, so hide now to
            // restore the same "invisible until caller shows" contract.
            if wayland {
                let _ = window.hide();
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
    // Wayland Protocol Error 71: skip_taskbar is not supported there.
    // Applied once on window creation only.
    if !exists && !is_wayland_session() {
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
            let position_x = match get("translate_window_position_x") {
                Some(v) => v.as_i64().unwrap(),
                None => 0,
            };
            let position_y = match get("translate_window_position_y") {
                Some(v) => v.as_i64().unwrap(),
                None => 0,
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
    use selection::get_text;
    // Get Selected Text
    let text = get_text();
    if !text.trim().is_empty() {
        let app_handle = APP.get().unwrap();
        // Write into State
        let state: tauri::State<StringWrapper> = app_handle.state();
        state.0.lock().unwrap().replace_range(.., &text);
    }

    let window = translate_window();
    if let Err(e) = window.show() {
        warn!("selection_translate: show() failed: {:?}", e);
    }
    window.emit("new_text", text).unwrap();
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
    if let Err(e) = window.show() {
        warn!("input_translate: show() failed: {:?}", e);
    }

    window.emit("new_text", "[INPUT_TRANSLATE]").unwrap();
}

pub fn text_translate(text: String) {
    let app_handle = APP.get().unwrap();
    // Clear State
    let state: tauri::State<StringWrapper> = app_handle.state();
    state.0.lock().unwrap().replace_range(.., &text);
    let window = translate_window();
    if let Err(e) = window.show() {
        warn!("text_translate: show() failed: {:?}", e);
    }
    window.emit("new_text", text).unwrap();
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
    if let Err(e) = window.show() {
        warn!("image_translate: show() failed: {:?}", e);
    }
    window.emit("new_text", "[IMAGE_TRANSLATE]").unwrap();
}

pub fn recognize_window() {
    let (window, exists) = build_window("recognize", "Recognize");
    if exists {
        if let Err(e) = window.show() {
            warn!("recognize_window: show() failed: {:?}", e);
        }
        window.emit("new_image", "").unwrap();
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
    window.emit("new_image", "").unwrap();
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
    #[cfg(target_os = "macos")]
    {
        let monitor = window.current_monitor().unwrap().unwrap();
        let size = monitor.size();
        window.set_decorations(false).unwrap();
        window.set_size(*size).unwrap();
    }

    #[cfg(not(target_os = "macos"))]
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
                if let Err(e) = window.set_position(LogicalPosition::new(-100.0, -100.0)) {
                    warn!("close_thumb: set_position failed: {:?}", e);
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
                let window = builder.build().unwrap();
                if wayland {
                    let _ = window.hide();
                }
                window
            };

            #[cfg(target_os = "windows")]
            let window = {
                let window = build_window(THUMB_WIN_NAME, THUMB_WIN_NAME).0;
                set_shadow(&window, false).unwrap_or_default();
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

    // Save content to file
    let app_dir = app_handle.path().app_config_dir().unwrap();
    let notify_file = app_dir.join("notify_content.json");
    let content_json = serde_json::json!({
        "content": content
    });
    std::fs::write(&notify_file, content_json.to_string()).unwrap();

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
