use crate::window::text_translate;
use std::sync::Mutex;
use tauri::Manager;

pub struct ClipboardMonitorEnableWrapper(pub Mutex<String>);

pub fn start_clipboard_monitor(app_handle: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        use arboard::Clipboard;
        let mut pre_text = "".to_string();
        let mut clipboard = Clipboard::new().ok();
        loop {
            let handle = app_handle.clone();
            let state = handle.state::<ClipboardMonitorEnableWrapper>();
            if let Ok(clipboard_monitor) = state.0.try_lock() {
                if clipboard_monitor.contains("true") {
                    // Recreate clipboard if previous attempt failed
                    if clipboard.is_none() {
                        clipboard = Clipboard::new().ok();
                    }
                    if let Some(ref mut cb) = clipboard {
                        match cb.get_text() {
                            Ok(text) => {
                                if text != pre_text {
                                    text_translate(text.clone());
                                    pre_text = text;
                                }
                            }
                            Err(_) => {
                                clipboard = None;
                            }
                        }
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    });
}
