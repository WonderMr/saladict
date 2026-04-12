use crate::window::text_translate;
use std::sync::Mutex;
use tauri::Manager;

pub struct ClipboardMonitorEnableWrapper(pub Mutex<String>);

pub fn start_clipboard_monitor(app_handle: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut pre_text = "".to_string();
        loop {
            let handle = app_handle.clone();
            let state = handle.state::<ClipboardMonitorEnableWrapper>();
            if let Ok(clipboard_monitor) = state.0.try_lock() {
                if clipboard_monitor.contains("true") {
                    use arboard::Clipboard;
                    if let Ok(mut clipboard) = Clipboard::new() {
                        if let Ok(text) = clipboard.get_text() {
                            if text != pre_text {
                                text_translate(text.clone());
                                pre_text = text;
                            }
                        }
                    }
                } else {
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    });
}
