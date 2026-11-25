mod black_screen;
mod clipboard;
mod color_picker;
mod explorer_menu;
mod shutdown_timer;
mod snipper;

pub fn try_handle_cli() -> bool {
    explorer_menu::try_handle_cli()
}

use std::sync::{Arc, Mutex};
use clipboard::{ClipboardItem, ClipboardManager};
use serde::{Deserialize, Serialize};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State, WindowEvent,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone)]
struct AppConfig {
    #[serde(default)]
    shortcut: Option<String>,
    #[serde(default)]
    shortcuts: Vec<String>,
    #[serde(default)]
    color_picker_shortcut: Option<String>,
    #[serde(default)]
    snipper_shortcut: Option<String>,
    #[serde(default)]
    explorer_context_menu: Option<bool>,
    #[serde(default = "default_true")]
    explorer_include_extensions: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ExplorerSettings {
    pub enabled: bool,
    pub include_extensions: bool,
}

fn get_config_path(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_data_dir().ok().map(|p| p.join("config.json"))
}

fn load_saved_config(app: &AppHandle) -> (Vec<String>, String, String, ExplorerSettings) {
    if let Some(path) = get_config_path(app) {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                let mut list = config.shortcuts;
                if let Some(single) = config.shortcut {
                    let s_trimmed = single.trim().to_string();
                    if !s_trimmed.is_empty() && !list.iter().any(|item| item.eq_ignore_ascii_case(&s_trimmed)) {
                        list.push(s_trimmed);
                    }
                }
                if list.is_empty() {
                    list = vec!["Alt+V".to_string()];
                }

                let picker_sc = config
                    .color_picker_shortcut
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| "Alt+C".to_string());

                let snipper_sc = config
                    .snipper_shortcut
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| "Ctrl+Shift+S".to_string());

                let explorer = ExplorerSettings {
                    enabled: config.explorer_context_menu.unwrap_or(true),
                    include_extensions: config.explorer_include_extensions,
                };

                return (list, picker_sc, snipper_sc, explorer);
            }
        }
    }
    (
        vec!["Alt+V".to_string()],
        "Alt+C".to_string(),
        "Ctrl+Shift+S".to_string(),
        ExplorerSettings {
            enabled: true,
            include_extensions: true,
        },
    )
}

fn save_saved_config(
    app: &AppHandle,
    shortcuts: &[String],
    color_picker_shortcut: &str,
    snipper_shortcut: &str,
    explorer: &ExplorerSettings,
) {
    if let Some(path) = get_config_path(app) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let config = AppConfig {
            shortcut: shortcuts.first().cloned(),
            shortcuts: shortcuts.to_vec(),
            color_picker_shortcut: Some(color_picker_shortcut.to_string()),
            snipper_shortcut: Some(snipper_shortcut.to_string()),
            explorer_context_menu: Some(explorer.enabled),
            explorer_include_extensions: explorer.include_extensions,
        };
        if let Ok(json) = serde_json::to_string_pretty(&config) {
            let _ = std::fs::write(path, json);
        }
    }
}

pub struct AppState {
    pub manager: Arc<ClipboardManager>,
    pub current_shortcuts: Arc<Mutex<Vec<String>>>,
    pub color_picker_shortcut: Arc<Mutex<String>>,
    pub snipper_shortcut: Arc<Mutex<String>>,
    pub explorer_settings: Arc<Mutex<ExplorerSettings>>,
    pub shutdown_timer: Arc<shutdown_timer::ShutdownTimerManager>,
}

#[cfg(windows)]
fn simulate_ctrl_v() {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL,
    };
    use std::mem::size_of;

    unsafe {
        let mut inputs: [INPUT; 4] = std::mem::zeroed();

        // 1. Ctrl Key Down
        inputs[0].r#type = INPUT_KEYBOARD;
        inputs[0].Anonymous.ki = KEYBDINPUT {
            wVk: VK_CONTROL,
            wScan: 0,
            dwFlags: 0,
            time: 0,
            dwExtraInfo: 0,
        };

        // 2. 'V' Key Down (0x56)
        inputs[1].r#type = INPUT_KEYBOARD;
        inputs[1].Anonymous.ki = KEYBDINPUT {
            wVk: 0x56,
            wScan: 0,
            dwFlags: 0,
            time: 0,
            dwExtraInfo: 0,
        };

        // 3. 'V' Key Up
        inputs[2].r#type = INPUT_KEYBOARD;
        inputs[2].Anonymous.ki = KEYBDINPUT {
            wVk: 0x56,
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };

        // 4. Ctrl Key Up
        inputs[3].r#type = INPUT_KEYBOARD;
        inputs[3].Anonymous.ki = KEYBDINPUT {
            wVk: VK_CONTROL,
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };

        SendInput(4, inputs.as_mut_ptr(), size_of::<INPUT>() as i32);
    }
}

#[cfg(windows)]
fn get_caret_or_cursor_position() -> (i32, i32) {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetCursorPos, GetForegroundWindow, GetGUIThreadInfo,
        GetWindowThreadProcessId, GUITHREADINFO,
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if !hwnd.is_null() {
            let thread_id = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
            let mut gui_info: GUITHREADINFO = std::mem::zeroed();
            gui_info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;

            if GetGUIThreadInfo(thread_id, &mut gui_info) != 0 && !gui_info.hwndCaret.is_null() {
                let mut pt = POINT {
                    x: gui_info.rcCaret.left,
                    y: gui_info.rcCaret.bottom,
                };
                if ClientToScreen(gui_info.hwndCaret, &mut pt) != 0 {
                    if pt.x > 0 || pt.y > 0 {
                        return (pt.x, pt.y);
                    }
                }
            }
        }

        // Fallback: mouse cursor position
        let mut pt = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut pt) != 0 {
            return (pt.x, pt.y);
        }

        (500, 500)
    }
}

#[cfg(windows)]
fn calculate_smart_position(window_width: i32, window_height: i32) -> (i32, i32) {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };

    let (target_x, target_y) = get_caret_or_cursor_position();

    unsafe {
        let pt = POINT {
            x: target_x,
            y: target_y,
        };
        let h_monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;

        let (work_left, work_top, work_right, work_bottom) =
            if GetMonitorInfoW(h_monitor, &mut mi) != 0 {
                (
                    mi.rcWork.left,
                    mi.rcWork.top,
                    mi.rcWork.right,
                    mi.rcWork.bottom,
                )
            } else {
                (0, 0, 1920, 1080)
            };

        // Align horizontally near the input field, clamping to screen edges
        let mut x = target_x - 30;
        if x + window_width > work_right - 12 {
            x = work_right - window_width - 12;
        }
        if x < work_left + 12 {
            x = work_left + 12;
        }

        // Check if there is enough space BELOW the input field
        let space_below = work_bottom - (target_y + 8);
        let y = if space_below >= window_height {
            // Position BELOW the input field
            target_y + 8
        } else {
            // Not enough room below -> position ABOVE the input field
            let y_above = target_y - window_height - 8;
            if y_above < work_top + 12 {
                work_bottom - window_height - 12
            } else {
                y_above
            }
        };

        (x, y)
    }
}

#[tauri::command]
fn get_history(state: State<'_, AppState>) -> Vec<ClipboardItem> {
    state.manager.get_items()
}

#[tauri::command]
fn copy_to_system(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.manager.copy_and_bump_to_top(&id, &app)
}

#[tauri::command]
fn paste_item(window: tauri::Window, app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.manager.copy_and_bump_to_top(&id, &app)?;
    let _ = window.hide();

    #[cfg(windows)]
    {
        std::thread::spawn(|| {
            // Give 50ms for Windows to return focus back to the target text box/window
            std::thread::sleep(std::time::Duration::from_millis(50));
            simulate_ctrl_v();
        });
    }

    Ok(())
}

#[tauri::command]
fn toggle_pin(app: AppHandle, state: State<'_, AppState>, id: String) -> Vec<ClipboardItem> {
    state.manager.toggle_pin(&id, &app)
}

#[tauri::command]
fn delete_item(app: AppHandle, state: State<'_, AppState>, id: String) -> Vec<ClipboardItem> {
    state.manager.delete_item(&id, &app)
}

#[tauri::command]
fn clear_unpinned(app: AppHandle, state: State<'_, AppState>) -> Vec<ClipboardItem> {
    state.manager.clear_unpinned(&app)
}

#[tauri::command]
fn save_image_to_disk(app: AppHandle, data_url: String, default_name: Option<String>) -> Result<String, String> {
    use base64::Engine;
    let base64_str = data_url.trim_start_matches("data:image/png;base64,");
    let png_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_str)
        .map_err(|e| e.to_string())?;

    let download_dir = app
        .path()
        .download_dir()
        .or_else(|_| app.path().picture_dir())
        .or_else(|_| app.path().desktop_dir())
        .map_err(|e| e.to_string())?;

    let filename = default_name.unwrap_or_else(|| {
        format!("WinFlow_Image_{}.png", chrono::Local::now().format("%Y%m%d_%H%M%S"))
    });

    let target_path = download_dir.join(&filename);
    std::fs::write(&target_path, png_bytes).map_err(|e| e.to_string())?;

    Ok(target_path.to_string_lossy().to_string())
}

#[tauri::command]
fn hide_window(window: tauri::Window) {
    let _ = window.hide();
}

#[tauri::command]
fn toggle_window(app: AppHandle) {
    // If color picker is active, automatically close it so clipboard opens cleanly
    if let Some(picker) = app.get_webview_window("picker") {
        if picker.is_visible().unwrap_or(false) {
            let _ = color_picker::cancel_color_picker(app.clone());
        }
    }
    if let Some(bs) = app.get_webview_window("blackscreen") {
        if bs.is_visible().unwrap_or(false) {
            let _ = black_screen::stop_black_screen(app.clone());
        }
    }

    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            #[cfg(windows)]
            {
                let (x, y) = calculate_smart_position(370, 440);
                let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
            }
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

#[tauri::command]
fn get_shortcuts(state: State<'_, AppState>) -> Vec<String> {
    state.current_shortcuts.lock().unwrap().clone()
}

#[tauri::command]
fn add_shortcut(app: AppHandle, state: State<'_, AppState>, new_shortcut: String) -> Result<Vec<String>, String> {
    let clean = new_shortcut.trim().to_string();
    let parsed: Shortcut = clean
        .parse()
        .map_err(|e| format!("Invalid shortcut combination '{}': {:?}", clean, e))?;

    let mut current = state.current_shortcuts.lock().unwrap();

    if current.iter().any(|s| s.eq_ignore_ascii_case(&clean)) {
        return Err("This shortcut is already added.".to_string());
    }

    if current.len() >= 5 {
        return Err("Maximum of 5 shortcuts reached.".to_string());
    }

    // Register new shortcut with OS
    app.global_shortcut()
        .register(parsed)
        .map_err(|e| format!("Failed to register '{}' (it may be reserved by Windows or another app): {}", clean, e))?;

    current.push(clean);
    let picker_sc = state.color_picker_shortcut.lock().unwrap().clone();
    let snipper_sc = state.snipper_shortcut.lock().unwrap().clone();
    let explorer = state.explorer_settings.lock().unwrap().clone();
    save_saved_config(&app, &current, &picker_sc, &snipper_sc, &explorer);

    Ok(current.clone())
}

#[tauri::command]
fn remove_shortcut(app: AppHandle, state: State<'_, AppState>, shortcut_to_remove: String) -> Result<Vec<String>, String> {
    let clean = shortcut_to_remove.trim();
    let mut current = state.current_shortcuts.lock().unwrap();

    if current.len() <= 1 {
        return Err("You must keep at least one active shortcut.".to_string());
    }

    if let Some(pos) = current.iter().position(|s| s.eq_ignore_ascii_case(clean)) {
        let removed = current.remove(pos);
        if let Ok(parsed) = removed.parse::<Shortcut>() {
            let _ = app.global_shortcut().unregister(parsed);
        }
        let picker_sc = state.color_picker_shortcut.lock().unwrap().clone();
        let snipper_sc = state.snipper_shortcut.lock().unwrap().clone();
        let explorer = state.explorer_settings.lock().unwrap().clone();
        save_saved_config(&app, &current, &picker_sc, &snipper_sc, &explorer);
    }

    Ok(current.clone())
}

#[tauri::command]
fn reset_shortcuts(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let mut current = state.current_shortcuts.lock().unwrap();

    // Unregister all
    for sc in current.iter() {
        if let Ok(parsed) = sc.parse::<Shortcut>() {
            let _ = app.global_shortcut().unregister(parsed);
        }
    }

    let default_list = vec!["Alt+V".to_string()];
    if let Ok(parsed) = "Alt+V".parse::<Shortcut>() {
        let _ = app.global_shortcut().register(parsed);
    }

    *current = default_list.clone();
    let picker_sc = state.color_picker_shortcut.lock().unwrap().clone();
    let snipper_sc = state.snipper_shortcut.lock().unwrap().clone();
    let explorer = state.explorer_settings.lock().unwrap().clone();
    save_saved_config(&app, &default_list, &picker_sc, &snipper_sc, &explorer);

    Ok(default_list)
}

#[tauri::command]
fn get_color_picker_shortcut(state: State<'_, AppState>) -> String {
    state.color_picker_shortcut.lock().unwrap().clone()
}

#[tauri::command]
fn set_color_picker_shortcut(app: AppHandle, state: State<'_, AppState>, new_shortcut: String) -> Result<String, String> {
    let clean = new_shortcut.trim().to_string();
    let parsed: Shortcut = clean
        .parse()
        .map_err(|e| format!("Invalid shortcut combination '{}': {:?}", clean, e))?;

    let mut current = state.color_picker_shortcut.lock().unwrap();

    // Unregister old shortcut
    if let Ok(old_parsed) = current.parse::<Shortcut>() {
        let _ = app.global_shortcut().unregister(old_parsed);
    }

    // Register new shortcut
    app.global_shortcut()
        .register(parsed)
        .map_err(|e| format!("Failed to register '{}' (it may be in use by Windows or another app): {}", clean, e))?;

    *current = clean.clone();
    let shortcuts = state.current_shortcuts.lock().unwrap().clone();
    let snipper_sc = state.snipper_shortcut.lock().unwrap().clone();
    let explorer = state.explorer_settings.lock().unwrap().clone();
    save_saved_config(&app, &shortcuts, &clean, &snipper_sc, &explorer);

    Ok(clean)
}

#[tauri::command]
fn get_snipper_shortcut(state: State<'_, AppState>) -> String {
    state.snipper_shortcut.lock().unwrap().clone()
}

#[tauri::command]
fn set_snipper_shortcut(app: AppHandle, state: State<'_, AppState>, new_shortcut: String) -> Result<String, String> {
    let clean = new_shortcut.trim().to_string();
    let parsed: Shortcut = clean
        .parse()
        .map_err(|e| format!("Invalid shortcut combination '{}': {:?}", clean, e))?;

    let mut current = state.snipper_shortcut.lock().unwrap();
    if let Ok(old_parsed) = current.parse::<Shortcut>() {
        let _ = app.global_shortcut().unregister(old_parsed);
    }

    app.global_shortcut()
        .register(parsed)
        .map_err(|e| format!("Failed to register '{}' (it may be in use by Windows or another app): {}", clean, e))?;

    *current = clean.clone();
    let shortcuts = state.current_shortcuts.lock().unwrap().clone();
    let picker_sc = state.color_picker_shortcut.lock().unwrap().clone();
    let explorer = state.explorer_settings.lock().unwrap().clone();
    save_saved_config(&app, &shortcuts, &picker_sc, &clean, &explorer);

    Ok(clean)
}

#[tauri::command]
fn get_explorer_settings(state: State<'_, AppState>) -> ExplorerSettings {
    let mut settings = state.explorer_settings.lock().unwrap().clone();
    settings.enabled = explorer_menu::is_explorer_context_menu_enabled();
    settings
}

#[tauri::command]
fn set_explorer_context_menu(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<ExplorerSettings, String> {
    explorer_menu::set_explorer_context_menu(enabled)?;
    let mut explorer = state.explorer_settings.lock().unwrap();
    explorer.enabled = enabled;
    let updated = explorer.clone();
    drop(explorer);

    let shortcuts = state.current_shortcuts.lock().unwrap().clone();
    let picker_sc = state.color_picker_shortcut.lock().unwrap().clone();
    let snipper_sc = state.snipper_shortcut.lock().unwrap().clone();
    save_saved_config(&app, &shortcuts, &picker_sc, &snipper_sc, &updated);
    Ok(updated)
}

#[tauri::command]
fn set_explorer_include_extensions(
    app: AppHandle,
    state: State<'_, AppState>,
    include_extensions: bool,
) -> Result<ExplorerSettings, String> {
    let mut explorer = state.explorer_settings.lock().unwrap();
    explorer.include_extensions = include_extensions;
    let updated = explorer.clone();
    drop(explorer);

    let shortcuts = state.current_shortcuts.lock().unwrap().clone();
    let picker_sc = state.color_picker_shortcut.lock().unwrap().clone();
    let snipper_sc = state.snipper_shortcut.lock().unwrap().clone();
    save_saved_config(&app, &shortcuts, &picker_sc, &snipper_sc, &updated);
    Ok(updated)
}

#[tauri::command]
fn get_shutdown_timer_status(state: State<'_, AppState>) -> shutdown_timer::ShutdownTimerStatus {
    state.shutdown_timer.get_status()
}

#[tauri::command]
fn start_shutdown_timer(
    app: AppHandle,
    state: State<'_, AppState>,
    request: shutdown_timer::StartPowerTimerRequest,
) -> Result<shutdown_timer::ShutdownTimerStatus, String> {
    state.shutdown_timer.start(app, request)
}

#[tauri::command]
fn cancel_shutdown_timer(
    app: AppHandle,
    state: State<'_, AppState>,
) -> shutdown_timer::ShutdownTimerStatus {
    state.shutdown_timer.cancel(app)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let manager = Arc::new(ClipboardManager::new());
    let current_shortcuts = Arc::new(Mutex::new(vec!["Alt+V".to_string()]));
    let color_picker_shortcut = Arc::new(Mutex::new("Alt+C".to_string()));
    let snipper_shortcut = Arc::new(Mutex::new("Ctrl+Shift+S".to_string()));
    let explorer_settings = Arc::new(Mutex::new(ExplorerSettings {
        enabled: true,
        include_extensions: true,
    }));
    let shutdown_timer = Arc::new(shutdown_timer::ShutdownTimerManager::new());

    let manager_for_thread = Arc::clone(&manager);
    let manager_for_setup = Arc::clone(&manager);
    let current_shortcuts_for_setup = Arc::clone(&current_shortcuts);
    let color_picker_shortcut_for_setup = Arc::clone(&color_picker_shortcut);
    let snipper_shortcut_for_setup = Arc::clone(&snipper_shortcut);
    let explorer_settings_for_setup = Arc::clone(&explorer_settings);

    tauri::Builder::default()
        .manage(AppState {
            manager,
            current_shortcuts,
            color_picker_shortcut,
            snipper_shortcut,
            explorer_settings,
            shutdown_timer,
        })
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        let state = app.state::<AppState>();
                        let picker_sc = state.color_picker_shortcut.lock().unwrap().clone();
                        let snipper_sc = state.snipper_shortcut.lock().unwrap().clone();

                        if let Ok(picker_parsed) = picker_sc.parse::<Shortcut>() {
                            if picker_parsed == *shortcut {
                                let _ = color_picker::toggle_color_picker(app.clone());
                                return;
                            }
                        }

                        if let Ok(snipper_parsed) = snipper_sc.parse::<Shortcut>() {
                            if snipper_parsed == *shortcut {
                                let _ = snipper::start_snipper(app.clone());
                                return;
                            }
                        }

                        // Otherwise toggle clipboard window
                        toggle_window(app.clone());
                    }
                })
                .build(),
        )
        .on_window_event(|window, event| {
            // Auto-close / hide main window when clicking outside (window loses focus)
            if window.label() == "main" {
                if let WindowEvent::Focused(focused) = event {
                    if !focused {
                        let _ = window.hide();
                    }
                }
            }
        })
        .setup(move |app| {
            let app_handle = app.handle().clone();

            // Load saved clipboard history from disk
            manager_for_setup.init_from_disk(&app_handle);

            // Load saved shortcuts, color picker shortcut, snipper shortcut & explorer settings from disk
            let (saved_shortcuts, saved_picker_sc, saved_snipper_sc, saved_explorer) = load_saved_config(&app_handle);
            *current_shortcuts_for_setup.lock().unwrap() = saved_shortcuts.clone();
            *color_picker_shortcut_for_setup.lock().unwrap() = saved_picker_sc.clone();
            *snipper_shortcut_for_setup.lock().unwrap() = saved_snipper_sc.clone();
            *explorer_settings_for_setup.lock().unwrap() = saved_explorer.clone();

            // Sync Windows File Explorer context menu registry entries
            let _ = explorer_menu::set_explorer_context_menu(saved_explorer.enabled);

            for sc in &saved_shortcuts {
                if let Ok(parsed) = sc.parse::<Shortcut>() {
                    let _ = app.global_shortcut().register(parsed);
                }
            }

            if let Ok(picker_parsed) = saved_picker_sc.parse::<Shortcut>() {
                let _ = app.global_shortcut().register(picker_parsed);
            }

            if let Ok(snipper_parsed) = saved_snipper_sc.parse::<Shortcut>() {
                let _ = app.global_shortcut().register(snipper_parsed);
            }

            // Enable autostart with Windows on boot
            let _ = app.autolaunch().enable();

            // Start clipboard listener background thread
            manager_for_thread.start_monitoring(app_handle.clone());

            // Create System Tray Menu
            let show_i = MenuItem::with_id(app, "show", "Open WinFlow", true, None::<&str>)?;
            let snipper_i = MenuItem::with_id(app, "snipper", "Snipping Tool (Ctrl+Shift+S)", true, None::<&str>)?;
            let picker_i = MenuItem::with_id(app, "color_picker", "Color Picker (Alt+C)", true, None::<&str>)?;
            let black_screen_i = MenuItem::with_id(app, "black_screen", "Black Screen", true, None::<&str>)?;
            let shutdown_timer_i = MenuItem::with_id(app, "shutdown_timer", "Shutdown Timer...", true, None::<&str>)?;
            let update_i = MenuItem::with_id(app, "check_update", "Check for Updates...", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit WinFlow", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[
                    &show_i,
                    &snipper_i,
                    &picker_i,
                    &black_screen_i,
                    &shutdown_timer_i,
                    &update_i,
                    &quit_i,
                ],
            )?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        toggle_window(app.clone());
                    }
                    "snipper" => {
                        let _ = snipper::start_snipper(app.clone());
                    }
                    "color_picker" => {
                        let _ = color_picker::toggle_color_picker(app.clone());
                    }
                    "black_screen" => {
                        let _ = black_screen::start_black_screen(app.clone());
                    }
                    "shutdown_timer" => {
                        if let Some(window) = app.get_webview_window("main") {
                            if !window.is_visible().unwrap_or(false) {
                                toggle_window(app.clone());
                            }
                        }
                        let _ = app.emit("open-tools-modal", ());
                    }
                    "check_update" => {
                        toggle_window(app.clone());
                        let _ = app.emit("check-for-updates", ());
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        toggle_window(app.clone());
                    }
                })
                .build(app)?;

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_history,
            copy_to_system,
            paste_item,
            toggle_pin,
            delete_item,
            clear_unpinned,
            hide_window,
            toggle_window,
            get_shortcuts,
            add_shortcut,
            remove_shortcut,
            reset_shortcuts,
            get_color_picker_shortcut,
            set_color_picker_shortcut,
            get_snipper_shortcut,
            set_snipper_shortcut,
            save_image_to_disk,
            get_explorer_settings,
            set_explorer_context_menu,
            set_explorer_include_extensions,
            get_shutdown_timer_status,
            start_shutdown_timer,
            cancel_shutdown_timer,
            black_screen::start_black_screen,
            black_screen::stop_black_screen,
            snipper::start_snipper,
            snipper::get_snip_snapshot,
            snipper::delayed_snip_capture,
            snipper::close_snipper,
            snipper::copy_snip_image,
            snipper::save_snip_image,
            snipper::pin_snip_image,
            snipper::get_pinned_snip_data,
            snipper::minimize_pinned_snip,
            snipper::close_pinned_snip,
            snipper::ocr_snip_image,
            color_picker::start_color_picker,
            color_picker::toggle_color_picker,
            color_picker::finish_color_picker,
            color_picker::cancel_color_picker,
            color_picker::sample_pixel_at_cursor,
        ])
        .run(tauri::generate_context!())
        .expect("error while running winflow application");
}
