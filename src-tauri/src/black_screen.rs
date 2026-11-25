use tauri::{AppHandle, Emitter, Manager};

#[cfg(windows)]
fn get_virtual_screen_bounds() -> (i32, i32, i32, i32) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN,
    };

    unsafe {
        let mut x = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let mut y = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let mut w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let mut h = GetSystemMetrics(SM_CYVIRTUALSCREEN);

        if w <= 0 || h <= 0 {
            x = 0;
            y = 0;
            w = GetSystemMetrics(0);
            h = GetSystemMetrics(1);
        }

        if w <= 0 || h <= 0 {
            w = 1920;
            h = 1080;
        }

        (x, y, w, h)
    }
}

#[tauri::command]
pub fn start_black_screen(app: AppHandle) -> Result<(), String> {
    if let Some(main_win) = app.get_webview_window("main") {
        let _ = main_win.hide();
    }

    if let Some(window) = app.get_webview_window("blackscreen") {
        #[cfg(windows)]
        {
            let (x, y, w, h) = get_virtual_screen_bounds();
            let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
            let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize {
                width: w as u32,
                height: h as u32,
            }));
        }

        let _ = window.emit("black-screen-opened", ());
        let _ = window.show();
        let _ = window.set_focus();
    }

    Ok(())
}

#[tauri::command]
pub fn stop_black_screen(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("blackscreen") {
        let _ = window.hide();
    }
    Ok(())
}
