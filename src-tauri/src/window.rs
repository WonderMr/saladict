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
#[cfg(not(target_os = "linux"))]
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

pub const THUMB_WIN_NAME: &str = "thumb";// Get daemon window instance
#[cfg(not(target_os = "linux"))]
fn get_daemon_window() -> WebviewWindow {
    let app_handle = APP.get().unwrap();
    match app_handle.get_webview_window("daemon") {
        Some(v) => v,
        None => {
            warn!("Daemon window not found, create new daemon window!");
            WebviewWindowBuilder::new(
                app_handle,
                "daemon",
                WebviewUrl::App("daemon.html".into()),
            )
            .title("Daemon")
            .visible(false)
            .build()
            .unwrap()
        }
    }
}

// Get monitor where the mouse is currently located
#[cfg(not(target_os = "linux"))]
fn get_current_monitor(x: i32, y: i32) -> Monitor {
    info!("Mouse position: {}, {}", x, y);
    let daemon_window = get_daemon_window();
    let monitors = daemon_window.available_monitors().unwrap();

    for m in monitors {
        let size = m.size();
        let position = m.position();

        if x >= position.x
            && x <= (position.x + size.width as i32)
            && y >= position.y
            && y <= (position.y + size.height as i32)
        {
            info!("Current Monitor: {:?}", m);
            return m;
        }
    }
    warn!("Current Monitor not found, using primary monitor");
    daemon_window.primary_monitor().unwrap().unwrap()
}

// Creating a window on the mouse monitor
fn build_window(label: &str, title: &str) -> (WebviewWindow, bool) {
    let app_handle = APP.get().unwrap();
    match app_handle.get_webview_window(label) {
        Some(v) => {
            info!("Window existence: {}", label);
            let _ = v.show();
            let _ = v.set_focus();
            (v, true)
        }
        None => {
            info!("Window not existence, Creating new window: {}", label);
            #[allow(unused_mut)]
            let mut builder = WebviewWindowBuilder::new(
                app_handle,
                label,
                WebviewUrl::App("index.html".into()),
            )
            .focused(true)
            .title(title);

            // Wayland rejects position/visible(false)/skip_taskbar at build time,
            // so these are applied only on non-Linux targets.
            #[cfg(not(target_os = "linux"))]
            {
                use mouse_position::mouse_position::{Mouse, Position};
                let mouse_position = match Mouse::get_mouse_position() {
                    Mouse::Position { x, y } => Position { x, y },
                    Mouse::Error => {
                        warn!("Mouse position not found, using (0, 0) as default");
                        Position { x: 0, y: 0 }
                    }
                };
                let position = get_current_monitor(mouse_position.x, mouse_position.y).position();
                let hide_dock_icon = get("hide_dock_icon")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                builder = builder
                    .position(position.x.into(), position.y.into())
                    .visible(false)
                    .skip_taskbar(hide_dock_icon);
            }

            #[cfg(target_os = "macos")]
            {
                builder = builder
                    .title_bar_style(tauri::TitleBarStyle::Overlay)
                    .hidden_title(true);
            }
            #[cfg(target_os = "windows")]
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
            // so it doesn't flash at (0, 0) with default size on non-Linux.
            #[cfg(not(target_os = "linux"))]
            let _ = window.current_monitor();
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
    #[cfg(not(target_os = "linux"))]
    if let Err(e) = window.center() {
        warn!("config_window: center() failed: {:?}", e);
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
    if exists {
        return window;
    }
    // Wayland Protocol Error 71: skip_taskbar is not supported there.
    #[cfg(not(target_os = "linux"))]
    if let Err(e) = window.set_skip_taskbar(true) {
        warn!("translate_window: set_skip_taskbar failed: {:?}", e);
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

            if let Err(e) = window.set_position(tauri::PhysicalPosition::new(
                mouse_position.x,
                mouse_position.y,
            )) {
                warn!("translate_window: set_position failed: {:?}", e);
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
            if let Err(e) = window.set_position(tauri::PhysicalPosition::new(
                (position_x as f64) * dpi,
                (position_y as f64) * dpi,
            )) {
                warn!("translate_window: set_position failed: {:?}", e);
            }
        }
    }

    if let Err(e) = window.show() {
        warn!("translate_window: show() failed: {:?}", e);
    }

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
    #[cfg(not(target_os = "linux"))]
    {
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

    window.emit("new_text", "[INPUT_TRANSLATE]").unwrap();
}

pub fn text_translate(text: String) {
    let app_handle = APP.get().unwrap();
    // Clear State
    let state: tauri::State<StringWrapper> = app_handle.state();
    state.0.lock().unwrap().replace_range(.., &text);
    let window = translate_window();
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
    window.emit("new_text", "[IMAGE_TRANSLATE]").unwrap();
}

pub fn recognize_window() {
    let (window, exists) = build_window("recognize", "Recognize");
    if exists {
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
    #[cfg(not(target_os = "linux"))]
    if let Err(e) = window.center() {
        warn!("recognize_window: center() failed: {:?}", e);
    }
    if let Err(e) = window.show() {
        warn!("recognize_window: show() failed: {:?}", e);
    }
    window.emit("new_image", "").unwrap();
}

#[cfg(not(target_os = "macos"))]
fn screenshot_window() -> WebviewWindow {
    let (window, _exists) = build_window("screenshot", "Screenshot");

    // Wayland Protocol Error 71: skip_taskbar is not supported there.
    #[cfg(not(target_os = "linux"))]
    if let Err(e) = window.set_skip_taskbar(true) {
        warn!("screenshot_window: set_skip_taskbar failed: {:?}", e);
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
    if let Err(e) = window.show() {
        warn!("screenshot_window: show() failed: {:?}", e);
    }
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
    #[cfg(not(target_os = "linux"))]
    if let Err(e) = window.center() {
        warn!("updater_window: center() failed: {:?}", e);
    }
    if let Err(e) = window.show() {
        warn!("updater_window: show() failed: {:?}", e);
    }
}

pub fn delete_thumb() {
    match APP.get() {
        Some(handle) => match handle.get_webview_window(THUMB_WIN_NAME) {
            Some(window) => {
                window.close().unwrap();
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
                window
                    .set_position(LogicalPosition::new(-100.0, -100.0))
                    .unwrap();
                window.set_always_on_top(false).unwrap();
                window.hide().unwrap();
            }
            None => {}
        },
        None => {}
    }
}

pub fn show_thumb(x: i32, y: i32) {
    let window = get_thumb_window(x, y);
    window.show().unwrap();
}

pub fn get_thumb_window(x: i32, y: i32) -> WebviewWindow {
    let handle = APP.get().unwrap();
    let position_offset = 7.0 as f64;
    let window = match handle.get_webview_window(THUMB_WIN_NAME) {
        Some(window) => {
            info!("Thumb window already exists");
            window.unminimize().unwrap();
            window.set_always_on_top(true).unwrap();
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
                .visible(false)
                .resizable(false)
                .skip_taskbar(true)
                .closable(false)
                .decorations(false);

                #[cfg(not(feature = "app-store"))]
                {
                    builder = builder.transparent(true);
                }
                builder.build().unwrap()
            };

            #[cfg(target_os = "windows")]
            let window = {
                let window = build_window(THUMB_WIN_NAME, THUMB_WIN_NAME).0;
                set_shadow(&window, false).unwrap_or_default();
                window.set_resizable(false).unwrap();
                window.set_skip_taskbar(true).unwrap();
                window
                    .set_size(tauri::LogicalSize {
                        width: 20.0,
                        height: 20.0,
                    })
                    .unwrap();
                window
            };

            post_process_window(&window);
            window.unminimize().unwrap();
            window.set_always_on_top(true).unwrap();
            window
        }
    };

    if cfg!(target_os = "macos") {
        window
            .set_position(LogicalPosition::new(
                x as f64 + position_offset,
                y as f64 + position_offset,
            ))
            .unwrap();
    } else {
        window
            .set_position(PhysicalPosition::new(
                x as f64 + position_offset,
                y as f64 + position_offset,
            ))
            .unwrap();
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
            v.set_focus().unwrap();
            v
        }
        None => {
            info!("Creating new notification window");
            build_window("notify", "Notification").0
        }
    };

    window.set_size(tauri::LogicalSize::new(400, 400)).unwrap();
    #[cfg(not(target_os = "linux"))]
    if let Err(e) = window.center() {
        warn!("notify_window: center() failed: {:?}", e);
    }
    window.set_maximizable(false).unwrap();
    window.set_minimizable(false).unwrap();
    window.set_always_on_top(true).unwrap();
    window.show().unwrap();
}
