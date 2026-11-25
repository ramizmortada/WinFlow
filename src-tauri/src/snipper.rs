use base64::Engine;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WindowBounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub title: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SnipSnapshotPayload {
    pub data_url: String,
    pub offset_x: i32,
    pub offset_y: i32,
    pub width: i32,
    pub height: i32,
    pub windows: Vec<WindowBounds>,
}

static LAST_SNIP_SNAPSHOT: Mutex<Option<SnipSnapshotPayload>> = Mutex::new(None);
static PINNED_SNIP_DATA: Mutex<Option<String>> = Mutex::new(None);

#[cfg(windows)]
fn enumerate_visible_windows(offset_x: i32, offset_y: i32) -> Vec<WindowBounds> {
    use windows_sys::core::BOOL;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowRect, GetWindowTextLengthW, GetWindowTextW, IsIconic,
        IsWindowVisible,
    };

    struct EnumState {
        offset_x: i32,
        offset_y: i32,
        windows: Vec<WindowBounds>,
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let state = &mut *(lparam as *mut EnumState);

        if IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
            return 1;
        }

        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return 1;
        }

        let mut buf = vec![0u16; (len + 1) as usize];
        let copied = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        let title = String::from_utf16_lossy(&buf[..copied.max(0) as usize]);

        if title.is_empty()
            || title == "WinFlow"
            || title == "WinFlow Snipper"
            || title == "WinFlow Color Picker"
            || title == "WinFlow Black Screen"
            || title == "Program Manager"
        {
            return 1;
        }

        let mut rect: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return 1;
        }

        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        if w < 30 || h < 30 {
            return 1;
        }

        state.windows.push(WindowBounds {
            x: rect.left - state.offset_x,
            y: rect.top - state.offset_y,
            width: w,
            height: h,
            title,
        });

        1
    }

    let mut state = EnumState {
        offset_x,
        offset_y,
        windows: Vec::new(),
    };

    unsafe {
        EnumWindows(Some(enum_proc), &mut state as *mut EnumState as LPARAM);
    }

    state.windows
}

#[cfg(windows)]
fn capture_virtual_screen() -> Result<SnipSnapshotPayload, String> {
    use image::codecs::jpeg::JpegEncoder;
    use image::ColorType;
    use windows_sys::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits,
        GetDC, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        SRCCOPY,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN,
    };

    unsafe {
        let mut offset_x = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let mut offset_y = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let mut width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let mut height = GetSystemMetrics(SM_CYVIRTUALSCREEN);

        if width <= 0 || height <= 0 {
            offset_x = 0;
            offset_y = 0;
            width = GetSystemMetrics(0);
            height = GetSystemMetrics(1);
        }

        if width <= 0 || height <= 0 {
            width = 1920;
            height = 1080;
        }

        let hdc_screen = GetDC(std::ptr::null_mut());
        if hdc_screen.is_null() {
            return Err("Failed to acquire screen DC".to_string());
        }

        let hdc_mem = CreateCompatibleDC(hdc_screen);
        if hdc_mem.is_null() {
            ReleaseDC(std::ptr::null_mut(), hdc_screen);
            return Err("Failed to create memory DC".to_string());
        }

        let hbitmap = CreateCompatibleBitmap(hdc_screen, width, height);
        if hbitmap.is_null() {
            DeleteDC(hdc_mem);
            ReleaseDC(std::ptr::null_mut(), hdc_screen);
            return Err("Failed to create compatible bitmap".to_string());
        }

        let old_obj = SelectObject(hdc_mem, hbitmap);
        BitBlt(
            hdc_mem,
            0,
            0,
            width,
            height,
            hdc_screen,
            offset_x,
            offset_y,
            SRCCOPY,
        );

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = width;
        bmi.bmiHeader.biHeight = -height;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB as u32;

        let buf_size = (width * height * 4) as usize;
        let mut bgra = vec![0u8; buf_size];

        GetDIBits(
            hdc_mem,
            hbitmap,
            0,
            height as u32,
            bgra.as_mut_ptr() as *mut _,
            &mut bmi,
            DIB_RGB_COLORS,
        );

        SelectObject(hdc_mem, old_obj);
        DeleteObject(hbitmap);
        DeleteDC(hdc_mem);
        ReleaseDC(std::ptr::null_mut(), hdc_screen);

        // Convert BGRA -> RGB for fast JPEG preview encoding
        let pixel_count = (width as usize) * (height as usize);
        let mut rgb = Vec::with_capacity(pixel_count * 3);
        for chunk in bgra.chunks_exact(4) {
            rgb.push(chunk[2]);
            rgb.push(chunk[1]);
            rgb.push(chunk[0]);
        }

        let mut jpg_bytes = Vec::new();
        let mut encoder = JpegEncoder::new_with_quality(&mut jpg_bytes, 92);
        encoder
            .encode(&rgb, width as u32, height as u32, ColorType::Rgb8.into())
            .map_err(|e| e.to_string())?;

        let b64 = base64::engine::general_purpose::STANDARD.encode(&jpg_bytes);
        let data_url = format!("data:image/jpeg;base64,{}", b64);
        let windows = enumerate_visible_windows(offset_x, offset_y);

        Ok(SnipSnapshotPayload {
            data_url,
            offset_x,
            offset_y,
            width,
            height,
            windows,
        })
    }
}

#[cfg(not(windows))]
fn capture_virtual_screen() -> Result<SnipSnapshotPayload, String> {
    Err("Snipping tool is only supported on Windows.".to_string())
}

#[tauri::command]
pub fn start_snipper(app: AppHandle) -> Result<(), String> {
    if let Some(main_win) = app.get_webview_window("main") {
        if main_win.is_visible().unwrap_or(false) {
            let _ = main_win.hide();
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    let payload = capture_virtual_screen()?;
    *LAST_SNIP_SNAPSHOT.lock() = Some(payload.clone());

    if let Some(window) = app.get_webview_window("snipper") {
        let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: payload.offset_x,
            y: payload.offset_y,
        }));
        let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize {
            width: payload.width as u32,
            height: payload.height as u32,
        }));

        let _ = window.emit("snipper-opened", ());
        let _ = window.show();
        let _ = window.set_focus();
    }

    Ok(())
}

#[tauri::command]
pub fn get_snip_snapshot() -> Option<SnipSnapshotPayload> {
    LAST_SNIP_SNAPSHOT.lock().clone()
}

#[tauri::command]
pub fn delayed_snip_capture(app: AppHandle, delay_seconds: u64) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("snipper") {
        let _ = window.hide();
    }

    let delay = delay_seconds.min(30);
    if delay > 0 {
        std::thread::sleep(std::time::Duration::from_secs(delay));
    } else {
        std::thread::sleep(std::time::Duration::from_millis(80));
    }

    let payload = capture_virtual_screen()?;
    *LAST_SNIP_SNAPSHOT.lock() = Some(payload.clone());

    if let Some(window) = app.get_webview_window("snipper") {
        let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: payload.offset_x,
            y: payload.offset_y,
        }));
        let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize {
            width: payload.width as u32,
            height: payload.height as u32,
        }));
        let _ = window.emit("snipper-opened", ());
        let _ = window.show();
        let _ = window.set_focus();
    }

    Ok(())
}

#[tauri::command]
pub fn close_snipper(app: AppHandle) {
    if let Some(window) = app.get_webview_window("snipper") {
        let _ = window.hide();
    }
    *LAST_SNIP_SNAPSHOT.lock() = None;
}

#[tauri::command]
pub fn copy_snip_image(app: AppHandle, data_url: String) -> Result<(), String> {
    let base64_str = data_url.trim_start_matches("data:image/png;base64,");
    let png_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_str)
        .map_err(|e| e.to_string())?;

    let img = image::load_from_memory(&png_bytes).map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let raw_bytes = rgba.into_raw();

    let mut last_err = String::new();
    for attempt in 0..4 {
        match arboard::Clipboard::new().and_then(|mut cb| {
            cb.set_image(arboard::ImageData {
                width: w as usize,
                height: h as usize,
                bytes: std::borrow::Cow::Borrowed(&raw_bytes),
            })
        }) {
            Ok(()) => {
                close_snipper(app);
                return Ok(());
            }
            Err(e) => {
                last_err = e.to_string();
                if attempt < 3 {
                    std::thread::sleep(std::time::Duration::from_millis(30));
                }
            }
        }
    }

    Err(last_err)
}

#[tauri::command]
pub fn save_snip_image(app: AppHandle, data_url: String) -> Result<String, String> {
    let base64_str = data_url.trim_start_matches("data:image/png;base64,");
    let png_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_str)
        .map_err(|e| e.to_string())?;

    let target_dir = app
        .path()
        .picture_dir()
        .or_else(|_| app.path().download_dir())
        .or_else(|_| app.path().desktop_dir())
        .map_err(|e| e.to_string())?;

    let filename = format!(
        "WinFlow_Snip_{}.png",
        chrono::Local::now().format("%Y%m%d_%H%M%S")
    );
    let target_path = target_dir.join(filename);
    std::fs::write(&target_path, png_bytes).map_err(|e| e.to_string())?;

    close_snipper(app);
    Ok(target_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn pin_snip_image(app: AppHandle, data_url: String, width: u32, height: u32) -> Result<(), String> {
    *PINNED_SNIP_DATA.lock() = Some(data_url.clone());

    let clamped_w = width.clamp(180, 680);
    let clamped_h = height.clamp(120, 520);

    if let Some(pin_win) = app.get_webview_window("pinned-snip") {
        let _ = pin_win.set_size(tauri::Size::Logical(tauri::LogicalSize {
            width: clamped_w as f64,
            height: clamped_h as f64,
        }));
        let _ = pin_win.emit("pinned-snip-updated", &data_url);
        let _ = pin_win.show();
        let _ = pin_win.set_focus();
    }

    close_snipper(app);
    Ok(())
}

#[tauri::command]
pub fn get_pinned_snip_data() -> Option<String> {
    PINNED_SNIP_DATA.lock().clone()
}

#[tauri::command]
pub fn minimize_pinned_snip(app: AppHandle) {
    if let Some(pin_win) = app.get_webview_window("pinned-snip") {
        let _ = pin_win.minimize();
    }
}

#[tauri::command]
pub fn close_pinned_snip(app: AppHandle) {
    if let Some(pin_win) = app.get_webview_window("pinned-snip") {
        let _ = pin_win.hide();
    }
    *PINNED_SNIP_DATA.lock() = None;
}

#[cfg(windows)]
#[tauri::command]
pub fn ocr_snip_image(app: AppHandle, data_url: String) -> Result<String, String> {
    use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::DataWriter;

    let base64_str = data_url.trim_start_matches("data:image/png;base64,");
    let png_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_str)
        .map_err(|e| e.to_string())?;

    let img = image::load_from_memory(&png_bytes).map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();

    let writer = DataWriter::new().map_err(|e| e.to_string())?;
    writer
        .WriteBytes(rgba.as_raw())
        .map_err(|e| e.to_string())?;
    let buffer = writer.DetachBuffer().map_err(|e| e.to_string())?;

    let bitmap = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
        &buffer,
        BitmapPixelFormat::Rgba8,
        w as i32,
        h as i32,
        BitmapAlphaMode::Premultiplied,
    )
    .map_err(|e| e.to_string())?;

    let engine = OcrEngine::TryCreateFromUserProfileLanguages()
        .map_err(|e| format!("Windows OCR is not available: {}", e))?;

    let result = engine
        .RecognizeAsync(&bitmap)
        .map_err(|e| e.to_string())?
        .get()
        .map_err(|e| e.to_string())?;

    let lines = result.Lines().map_err(|e| e.to_string())?;
    let mut extracted = Vec::new();
    for line in lines {
        if let Ok(text) = line.Text() {
            extracted.push(text.to_string());
        }
    }

    let final_text = extracted.join("\r\n");
    if !final_text.trim().is_empty() {
        for attempt in 0..3 {
            if let Ok(mut cb) = arboard::Clipboard::new() {
                if cb.set_text(&final_text).is_ok() {
                    break;
                }
            }
            if attempt < 2 {
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
        }
    }

    close_snipper(app);
    Ok(final_text)
}

#[cfg(not(windows))]
#[tauri::command]
pub fn ocr_snip_image(_app: AppHandle, _data_url: String) -> Result<String, String> {
    Err("OCR is only supported on Windows.".to_string())
}
