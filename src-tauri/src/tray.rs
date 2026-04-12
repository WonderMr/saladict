use crate::clipboard::*;
use crate::config::{get, set};
use crate::window::config_window;
use crate::window::input_translate;
use crate::window::ocr_recognize;
use crate::window::ocr_translate;
use crate::window::updater_window;
use log::info;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Manager};
use tauri::tray::{TrayIcon, TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState};
use crate::cmd::is_app_store_version;
use crate::APP;

pub fn build_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let menu = build_tray_menu(app.handle(), "", "")?;
    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .icon_as_template(true)
        .menu(&menu)
        .on_menu_event(|app, event| {
            match event.id().as_ref() {
                "input_translate" => on_input_translate_click(),
                "copy_source" => on_auto_copy_click(app, "source"),
                "clipboard_monitor" => on_clipboard_monitor_click(app),
                "copy_target" => on_auto_copy_click(app, "target"),
                "copy_source_target" => on_auto_copy_click(app, "source_target"),
                "copy_disable" => on_auto_copy_click(app, "disable"),
                "ocr_recognize" => on_ocr_recognize_click(),
                "ocr_translate" => on_ocr_translate_click(),
                "config" => on_config_click(),
                "check_update" => on_check_update_click(),
                "view_log" => on_view_log_click(app),
                "restart" => on_restart_click(app),
                "quit" => on_quit_click(app),
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                on_tray_click();
            }
        })
        .build(app)?;
    Ok(())
}

fn build_tray_menu(app_handle: &AppHandle, language: &str, copy_mode: &str) -> Result<Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    let lang = if language.is_empty() {
        match get("app_language") {
            Some(v) => v.as_str().unwrap().to_string(),
            None => "en".to_string(),
        }
    } else {
        language.to_string()
    };

    let mode = if copy_mode.is_empty() {
        match get("translate_auto_copy") {
            Some(v) => v.as_str().unwrap().to_string(),
            None => {
                set("translate_auto_copy", "disable");
                "disable".to_string()
            }
        }
    } else {
        copy_mode.to_string()
    };

    let enable_clipboard_monitor = match get("clipboard_monitor") {
        Some(v) => v.as_bool().unwrap(),
        None => {
            set("clipboard_monitor", false);
            false
        }
    };

    let labels = get_labels(&lang);

    let menu = Menu::new(app_handle)?;

    let input_translate_item = MenuItem::with_id(app_handle, "input_translate", labels.input_translate, true, None::<&str>)?;
    let clipboard_monitor_item = CheckMenuItem::with_id(app_handle, "clipboard_monitor", labels.clipboard_monitor, true, enable_clipboard_monitor, None::<&str>)?;

    let copy_source = CheckMenuItem::with_id(app_handle, "copy_source", labels.copy_source, true, mode == "source", None::<&str>)?;
    let copy_target = CheckMenuItem::with_id(app_handle, "copy_target", labels.copy_target, true, mode == "target", None::<&str>)?;
    let copy_source_target = CheckMenuItem::with_id(app_handle, "copy_source_target", labels.copy_source_target, true, mode == "source_target", None::<&str>)?;
    let copy_disable = CheckMenuItem::with_id(app_handle, "copy_disable", labels.copy_disable, true, mode == "disable", None::<&str>)?;

    let auto_copy_submenu = Submenu::with_id(app_handle, "auto_copy", labels.auto_copy, true)?;
    auto_copy_submenu.append(&copy_source)?;
    auto_copy_submenu.append(&copy_target)?;
    auto_copy_submenu.append(&copy_source_target)?;
    auto_copy_submenu.append(&PredefinedMenuItem::separator(app_handle)?)?;
    auto_copy_submenu.append(&copy_disable)?;

    let ocr_recognize_item = MenuItem::with_id(app_handle, "ocr_recognize", labels.ocr_recognize, true, None::<&str>)?;
    let ocr_translate_item = MenuItem::with_id(app_handle, "ocr_translate", labels.ocr_translate, true, None::<&str>)?;
    let config_item = MenuItem::with_id(app_handle, "config", labels.config, true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app_handle, "quit", labels.quit, true, None::<&str>)?;

    menu.append(&input_translate_item)?;
    menu.append(&clipboard_monitor_item)?;
    menu.append(&auto_copy_submenu)?;
    menu.append(&PredefinedMenuItem::separator(app_handle)?)?;
    menu.append(&ocr_recognize_item)?;
    menu.append(&ocr_translate_item)?;
    menu.append(&PredefinedMenuItem::separator(app_handle)?)?;
    menu.append(&config_item)?;

    if !is_app_store_version() {
        let check_update_item = MenuItem::with_id(app_handle, "check_update", labels.check_update, true, None::<&str>)?;
        menu.append(&check_update_item)?;
    }

    if let Some(dev_mode) = get("dev_mode") {
        if dev_mode.as_bool().unwrap_or(false) {
            let view_log_item = MenuItem::with_id(app_handle, "view_log", labels.view_log, true, None::<&str>)?;
            menu.append(&view_log_item)?;
        }
    }

    menu.append(&PredefinedMenuItem::separator(app_handle)?)?;

    if !is_app_store_version() {
        let restart_item = MenuItem::with_id(app_handle, "restart", labels.restart, true, None::<&str>)?;
        menu.append(&restart_item)?;
    }

    menu.append(&quit_item)?;

    Ok(menu)
}

#[tauri::command]
pub fn update_tray(app_handle: tauri::AppHandle, mut language: String, mut copy_mode: String) {
    if language.is_empty() {
        language = match get("app_language") {
            Some(v) => v.as_str().unwrap().to_string(),
            None => "en".to_string(),
        };
    }
    if copy_mode.is_empty() {
        copy_mode = match get("translate_auto_copy") {
            Some(v) => v.as_str().unwrap().to_string(),
            None => {
                set("translate_auto_copy", "disable");
                "disable".to_string()
            }
        };
    }

    info!(
        "Update tray with language: {}, copy mode: {}",
        language, copy_mode
    );

    if let Ok(menu) = build_tray_menu(&app_handle, &language, &copy_mode) {
        if let Some(tray) = app_handle.tray_by_id("main") {
            let _ = tray.set_menu(Some(menu));
        } else {
            // Try the default tray (first one)
            // In v2 we need to iterate or use a known id
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        if let Some(tray) = app_handle.tray_by_id("main") {
            let _ = tray.set_tooltip(Some(&format!(
                "{} {}",
                app_handle.package_info().name,
                app_handle.package_info().version
            )));
        }
    }
}

fn on_tray_click() {
    let event = match get("tray_click_event") {
        Some(v) => v.as_str().unwrap().to_string(),
        None => {
            set("tray_click_event", "config");
            "config".to_string()
        }
    };
    match event.as_str() {
        "config" => config_window(),
        "translate" => input_translate(),
        "ocr_recognize" => ocr_recognize(),
        "ocr_translate" => ocr_translate(),
        "disable" => {}
        _ => config_window(),
    }
}
fn on_input_translate_click() {
    input_translate();
}
fn on_clipboard_monitor_click(app: &AppHandle) {
    let enable_clipboard_monitor = match get("clipboard_monitor") {
        Some(v) => v.as_bool().unwrap(),
        None => {
            set("clipboard_monitor", false);
            false
        }
    };
    let current = !enable_clipboard_monitor;
    // Update Config File
    set("clipboard_monitor", current);
    // Update State and Start Monitor
    let state = app.state::<ClipboardMonitorEnableWrapper>();
    state
        .0
        .lock()
        .unwrap()
        .replace_range(.., &current.to_string());
    if current {
        start_clipboard_monitor(app.clone());
    }
    // Rebuild tray to reflect new state
    update_tray(app.clone(), "".to_string(), "".to_string());
}
fn on_auto_copy_click(app: &AppHandle, mode: &str) {
    info!("Set copy mode to: {}", mode);
    set("translate_auto_copy", mode);
    app.emit("translate_auto_copy_changed", mode).unwrap();
    update_tray(app.clone(), "".to_string(), mode.to_string());
}
fn on_ocr_recognize_click() {
    ocr_recognize();
}
fn on_ocr_translate_click() {
    ocr_translate();
}

fn on_config_click() {
    config_window();
}

fn on_check_update_click() {
    updater_window();
}
fn on_view_log_click(app: &AppHandle) {
    let log_path = app.path().app_log_dir().unwrap();
    let _ = tauri_plugin_opener::open_path(log_path.to_str().unwrap(), None::<&str>);
}
fn on_restart_click(app: &AppHandle) {
    info!("============== Restart App ==============");
    app.restart();
}
fn on_quit_click(app: &AppHandle) {
    info!("============== Quit App ==============");
    app.exit(0);
}

struct TrayLabels {
    input_translate: &'static str,
    clipboard_monitor: &'static str,
    auto_copy: &'static str,
    copy_source: &'static str,
    copy_target: &'static str,
    copy_source_target: &'static str,
    copy_disable: &'static str,
    ocr_recognize: &'static str,
    ocr_translate: &'static str,
    config: &'static str,
    check_update: &'static str,
    view_log: &'static str,
    restart: &'static str,
    quit: &'static str,
}

fn get_labels(lang: &str) -> TrayLabels {
    match lang {
        "zh_cn" => TrayLabels {
            input_translate: "输入翻译",
            clipboard_monitor: "监听剪切板",
            auto_copy: "自动复制",
            copy_source: "原文",
            copy_target: "译文",
            copy_source_target: "原文+译文",
            copy_disable: "关闭",
            ocr_recognize: "文字识别",
            ocr_translate: "截图翻译",
            config: "偏好设置",
            check_update: "检查更新",
            view_log: "查看日志",
            restart: "重启应用",
            quit: "退出",
        },
        "zh_tw" => TrayLabels {
            input_translate: "輸入翻譯",
            clipboard_monitor: "偵聽剪貼簿",
            auto_copy: "自動複製",
            copy_source: "原文",
            copy_target: "譯文",
            copy_source_target: "原文+譯文",
            copy_disable: "關閉",
            ocr_recognize: "文字識別",
            ocr_translate: "截圖翻譯",
            config: "偏好設定",
            check_update: "檢查更新",
            view_log: "查看日誌",
            restart: "重啓程式",
            quit: "退出",
        },
        "ja" => TrayLabels {
            input_translate: "翻訳を入力",
            clipboard_monitor: "クリップボードを監視する",
            auto_copy: "自動コピー",
            copy_source: "原文",
            copy_target: "訳文",
            copy_source_target: "原文+訳文",
            copy_disable: "閉じる",
            ocr_recognize: "テキスト認識",
            ocr_translate: "スクリーンショットの翻訳",
            config: "プリファレンス設定",
            check_update: "更新を確認する",
            view_log: "ログを見る",
            restart: "アプリの再起動",
            quit: "退出する",
        },
        "ko" => TrayLabels {
            input_translate: "입력 번역",
            clipboard_monitor: "감청 전단판",
            auto_copy: "자동 복사",
            copy_source: "원문",
            copy_target: "번역문",
            copy_source_target: "원문+번역문",
            copy_disable: "닫기",
            ocr_recognize: "문자인식",
            ocr_translate: "스크린샷 번역",
            config: "기본 설정",
            check_update: "업데이트 확인",
            view_log: "로그 보기",
            restart: "응용 프로그램 다시 시작",
            quit: "퇴출",
        },
        "fr" => TrayLabels {
            input_translate: "Traduction d'entrée",
            clipboard_monitor: "Surveiller le presse-papiers",
            auto_copy: "Copier automatiquement",
            copy_source: "Source",
            copy_target: "Cible",
            copy_source_target: "Source+Cible",
            copy_disable: "Désactiver",
            ocr_recognize: "Reconnaissance de texte",
            ocr_translate: "Traduction d'image",
            config: "Paramètres",
            check_update: "Vérifier les mises à jour",
            view_log: "Voir le journal",
            restart: "Redémarrer l'application",
            quit: "Quitter",
        },
        "de" => TrayLabels {
            input_translate: "Eingabeübersetzung",
            clipboard_monitor: "Zwischenablage überwachen",
            auto_copy: "Automatisch kopieren",
            copy_source: "Quelle",
            copy_target: "Ziel",
            copy_source_target: "Quelle+Ziel",
            copy_disable: "Deaktivieren",
            ocr_recognize: "Texterkennung",
            ocr_translate: "Bildübersetzung",
            config: "Einstellungen",
            check_update: "Auf Updates prüfen",
            view_log: "Protokoll anzeigen",
            restart: "Anwendung neu starten",
            quit: "Beenden",
        },
        "ru" => TrayLabels {
            input_translate: "Ввод перевода",
            clipboard_monitor: "Следить за буфером обмена",
            auto_copy: "Автоматическое копирование",
            copy_source: "Источник",
            copy_target: "Цель",
            copy_source_target: "Источник+Цель",
            copy_disable: "Отключить",
            ocr_recognize: "Распознавание текста",
            ocr_translate: "Перевод изображения",
            config: "Настройки",
            check_update: "Проверить обновления",
            view_log: "Просмотр журнала",
            restart: "Перезапустить приложение",
            quit: "Выход",
        },
        "fa" => TrayLabels {
            input_translate: "متن",
            clipboard_monitor: "گوش دادن به تخته برش",
            auto_copy: "کپی خودکار",
            copy_source: "منبع",
            copy_target: "هدف",
            copy_source_target: "منبع + هدف",
            copy_disable: "متن",
            ocr_recognize: "تشخیص متن",
            ocr_translate: "ترجمه عکس",
            config: "تنظیمات ترجیح",
            check_update: "بررسی بروزرسانی",
            view_log: "مشاهده گزارشات",
            restart: "راه‌اندازی مجدد برنامه",
            quit: "خروج",
        },
        "pt_br" => TrayLabels {
            input_translate: "Traduzir Entrada",
            clipboard_monitor: "Monitorando a área de transferência",
            auto_copy: "Copiar Automaticamente",
            copy_source: "Origem",
            copy_target: "Destino",
            copy_source_target: "Origem+Destino",
            copy_disable: "Desabilitar",
            ocr_recognize: "Reconhecimento de Texto",
            ocr_translate: "Tradução de Imagem",
            config: "Configurações",
            check_update: "Checar por Atualização",
            view_log: "Exibir Registro",
            restart: "Reiniciar aplicativo",
            quit: "Sair",
        },
        "uk" => TrayLabels {
            input_translate: "Введення перекладу",
            clipboard_monitor: "Стежити за буфером обміну",
            auto_copy: "Автоматичне копіювання",
            copy_source: "Джерело",
            copy_target: "Мета",
            copy_source_target: "Джерело+Мета",
            copy_disable: "Відключивши",
            ocr_recognize: "Розпізнавання тексту",
            ocr_translate: "Переклад зображення",
            config: "Настройка",
            check_update: "Перевірити оновлення",
            view_log: "Перегляд журналу",
            restart: "Перезапустити додаток",
            quit: "Вихід",
        },
        _ => TrayLabels {
            input_translate: "Input Translate",
            clipboard_monitor: "Clipboard Monitor",
            auto_copy: "Auto Copy",
            copy_source: "Source",
            copy_target: "Target",
            copy_source_target: "Source+Target",
            copy_disable: "Disable",
            ocr_recognize: "OCR Recognize",
            ocr_translate: "OCR Translate",
            config: "Config",
            check_update: "Check Update",
            view_log: "View Log",
            restart: "Restart",
            quit: "Quit",
        },
    }
}
