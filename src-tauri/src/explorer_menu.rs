use serde::Deserialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Default)]
struct PartialConfig {
    #[serde(default = "default_true")]
    explorer_include_extensions: bool,
}

fn default_true() -> bool {
    true
}

/// Reads `explorer_include_extensions` from `%APPDATA%\com.winflow.app\config.json`.
/// Defaults to `true` if the config file doesn't exist or can't be parsed.
fn read_include_extensions_setting() -> bool {
    if let Ok(appdata) = std::env::var("APPDATA") {
        let config_path = PathBuf::from(appdata)
            .join("com.winflow.app")
            .join("config.json");
        if let Ok(content) = std::fs::read_to_string(config_path) {
            if let Ok(cfg) = serde_json::from_str::<PartialConfig>(&content) {
                return cfg.explorer_include_extensions;
            }
        }
    }
    true
}

/// Extracts the display name of a path, optionally stripping the extension for files.
fn format_entry_name(path: &Path, is_dir: bool, include_extensions: bool) -> Option<String> {
    if include_extensions || is_dir {
        path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .filter(|s| !s.is_empty())
    } else {
        path.file_stem()
            .or_else(|| path.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .filter(|s| !s.is_empty())
    }
}

/// Copies all file and folder names inside `dir_path` (one per line) to the clipboard.
fn copy_folder_names(dir_path: &Path, include_extensions: bool) -> Result<(), String> {
    let entries = std::fs::read_dir(dir_path)
        .map_err(|e| format!("Failed to read folder '{}': {}", dir_path.display(), e))?;

    let mut names: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if let Some(name) = format_entry_name(&path, is_dir, include_extensions) {
            names.push(name);
        }
    }

    if names.is_empty() {
        return Ok(());
    }

    names.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    let joined = names.join("\r\n");

    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(joined).map_err(|e| e.to_string())?;
    Ok(())
}

/// Aggregates multi-selected items from sibling Explorer-spawned processes and copies their names.
#[cfg(windows)]
fn copy_selected_names_batched(item_path: &Path, include_extensions: bool) {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::time::Duration;
    use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{
        CreateMutexW, ReleaseMutex, WaitForSingleObject, INFINITE,
    };

    let batch_file = std::env::temp_dir().join("winflow_selected_batch.txt");
    let mutex_name: Vec<u16> = "Local\\WinFlowCopySelectedMutex\0".encode_utf16().collect();

    let h_mutex = unsafe { CreateMutexW(std::ptr::null(), 0, mutex_name.as_ptr()) };
    if h_mutex.is_null() {
        // Fallback: copy just this single item
        let is_dir = item_path.is_dir();
        if let Some(name) = format_entry_name(item_path, is_dir, include_extensions) {
            if let Ok(mut cb) = arboard::Clipboard::new() {
                let _ = cb.set_text(name);
            }
        }
        return;
    }

    // 1. Append our path under the mutex
    unsafe {
        let wait = WaitForSingleObject(h_mutex, INFINITE);
        if wait == WAIT_OBJECT_0 {
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&batch_file) {
                let _ = writeln!(f, "{}", item_path.to_string_lossy());
            }
            let _ = ReleaseMutex(h_mutex);
        }
    }

    // 2. Wait for sibling Explorer-spawned processes to append their paths, then flush once
    loop {
        std::thread::sleep(Duration::from_millis(120));

        let wait = unsafe { WaitForSingleObject(h_mutex, INFINITE) };
        if wait != WAIT_OBJECT_0 {
            break;
        }

        if !batch_file.exists() {
            // Another sibling process already flushed and copied the batch
            unsafe {
                let _ = ReleaseMutex(h_mutex);
            }
            break;
        }

        let settled = std::fs::metadata(&batch_file)
            .and_then(|m| m.modified())
            .and_then(|t| t.elapsed().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)))
            .map(|elapsed| elapsed >= Duration::from_millis(110))
            .unwrap_or(true);

        if settled {
            if let Ok(content) = std::fs::read_to_string(&batch_file) {
                let _ = std::fs::remove_file(&batch_file);
                unsafe {
                    let _ = ReleaseMutex(h_mutex);
                }

                let mut seen = HashSet::new();
                let mut names: Vec<String> = Vec::new();
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || !seen.insert(trimmed.to_string()) {
                        continue;
                    }
                    let p = Path::new(trimmed);
                    let is_dir = p.is_dir();
                    if let Some(name) = format_entry_name(p, is_dir, include_extensions) {
                        names.push(name);
                    }
                }

                if !names.is_empty() {
                    names.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
                    if let Ok(mut cb) = arboard::Clipboard::new() {
                        let _ = cb.set_text(names.join("\r\n"));
                    }
                }
            } else {
                let _ = std::fs::remove_file(&batch_file);
                unsafe {
                    let _ = ReleaseMutex(h_mutex);
                }
            }
            break;
        } else {
            unsafe {
                let _ = ReleaseMutex(h_mutex);
            }
        }
    }

    unsafe {
        let _ = CloseHandle(h_mutex);
    }
}

/// Checks if the application was invoked from the Windows File Explorer context menu.
/// Returns `true` if a CLI action was handled and the process should exit immediately.
pub fn try_handle_cli() -> bool {
    let mut args = std::env::args_os();
    let _exe = args.next();

    let Some(flag) = args.next() else {
        return false;
    };

    let flag_str = flag.to_string_lossy();
    if flag_str == "--copy-folder-names" {
        if let Some(path_os) = args.next() {
            let include_ext = read_include_extensions_setting();
            let _ = copy_folder_names(Path::new(&path_os), include_ext);
        }
        return true;
    }

    if flag_str == "--copy-selected-names" {
        if let Some(path_os) = args.next() {
            let include_ext = read_include_extensions_setting();
            #[cfg(windows)]
            copy_selected_names_batched(Path::new(&path_os), include_ext);
        }
        return true;
    }

    if flag_str == "--reset-poster" {
        if let Some(path_os) = args.next() {
            let _ = crate::movie_poster::reset_folder_poster(&path_os.to_string_lossy());
        }
        return true;
    }

    if flag_str == "--find-poster" {
        if let Some(path_os) = args.next() {
            let folder_str = path_os.to_string_lossy().to_string();
            #[cfg(windows)]
            {
                use std::fs::OpenOptions;
                use std::io::Write;
                use std::time::Duration;
                use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
                use windows_sys::Win32::System::Threading::{
                    CreateMutexW, OpenEventW, ReleaseMutex, SetEvent, WaitForSingleObject,
                    EVENT_MODIFY_STATE, INFINITE,
                };

                let batch_file = std::env::temp_dir().join("winflow_find_poster_batch.txt");
                let mutex_name = to_wide("Local\\WinFlowFindPosterMutex");
                let h_mutex = unsafe { CreateMutexW(std::ptr::null(), 0, mutex_name.as_ptr()) };

                if !h_mutex.is_null() {
                    unsafe {
                        if WaitForSingleObject(h_mutex, INFINITE) == WAIT_OBJECT_0 {
                            if let Ok(mut f) =
                                OpenOptions::new().create(true).append(true).open(&batch_file)
                            {
                                let _ = writeln!(f, "{}", folder_str);
                            }
                            let _ = ReleaseMutex(h_mutex);
                        }
                    }

                    // Wait for sibling Explorer-spawned processes to append their paths
                    loop {
                        std::thread::sleep(Duration::from_millis(120));
                        let wait = unsafe { WaitForSingleObject(h_mutex, INFINITE) };
                        if wait != WAIT_OBJECT_0 {
                            break;
                        }
                        if !batch_file.exists() {
                            unsafe {
                                let _ = ReleaseMutex(h_mutex);
                            }
                            return true;
                        }
                        let settled = std::fs::metadata(&batch_file)
                            .and_then(|m| m.modified())
                            .and_then(|t| {
                                t.elapsed()
                                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
                            })
                            .map(|elapsed| elapsed >= Duration::from_millis(110))
                            .unwrap_or(true);

                        unsafe {
                            let _ = ReleaseMutex(h_mutex);
                        }
                        if settled {
                            break;
                        }
                    }
                    unsafe {
                        let _ = CloseHandle(h_mutex);
                    }
                } else {
                    crate::movie_poster::write_pending_poster_folder(&folder_str);
                }

                let ev_name = to_wide("Local\\WinFlowFindPosterEvent");
                let h_ev = unsafe { OpenEventW(EVENT_MODIFY_STATE, 0, ev_name.as_ptr()) };
                if !h_ev.is_null() {
                    unsafe {
                        let _ = SetEvent(h_ev);
                        let _ = CloseHandle(h_ev);
                    }
                    return true;
                }
            }
        }
        return false;
    }

    false
}

#[cfg(windows)]
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn set_reg_sz(subkey: &str, value_name: Option<&str>, data: &str) -> Result<(), String> {
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, REG_SZ,
    };

    let subkey_w = to_wide(subkey);
    let mut hkey: HKEY = std::ptr::null_mut();

    let status = unsafe { RegCreateKeyW(HKEY_CURRENT_USER, subkey_w.as_ptr(), &mut hkey) };

    if status != ERROR_SUCCESS {
        return Err(format!("RegCreateKeyW failed for '{}' (code {})", subkey, status));
    }

    let name_w = value_name.map(to_wide);
    let data_w = to_wide(data);
    let byte_len = (data_w.len() * std::mem::size_of::<u16>()) as u32;

    let set_status = unsafe {
        RegSetValueExW(
            hkey,
            name_w.as_ref().map_or(std::ptr::null(), |v| v.as_ptr()),
            0,
            REG_SZ,
            data_w.as_ptr() as *const u8,
            byte_len,
        )
    };

    unsafe {
        RegCloseKey(hkey);
    }

    if set_status != ERROR_SUCCESS {
        return Err(format!("RegSetValueExW failed for '{}' (code {})", subkey, set_status));
    }

    Ok(())
}

#[cfg(windows)]
fn delete_reg_tree(subkey: &str) {
    use windows_sys::Win32::System::Registry::{RegDeleteTreeW, HKEY_CURRENT_USER};
    let subkey_w = to_wide(subkey);
    unsafe {
        let _ = RegDeleteTreeW(HKEY_CURRENT_USER, subkey_w.as_ptr());
    }
}

const REG_PATHS: [&str; 3] = [
    r"Software\Classes\Directory\Background\shell\WinFlowCopyNames",
    r"Software\Classes\Directory\shell\WinFlowCopyNames",
    r"Software\Classes\*\shell\WinFlowCopyNames",
];

/// Registers or unregisters the WinFlow File Explorer context menu entries in HKCU.
pub fn set_explorer_context_menu(enabled: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        if !enabled {
            for path in REG_PATHS {
                delete_reg_tree(path);
            }
            return Ok(());
        }

        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Could not resolve executable path: {}", e))?;
        let exe_str = exe_path.to_string_lossy().to_string();
        let icon_val = format!("{},0", exe_str);

        // 1. Inside an open folder (background right-click) -> copies all names inside that folder
        let bg_key = REG_PATHS[0];
        set_reg_sz(bg_key, None, "Copy All Names (WinFlow)")?;
        set_reg_sz(bg_key, Some("Icon"), &icon_val)?;
        set_reg_sz(
            &format!(r"{}\command", bg_key),
            None,
            &format!("\"{}\" --copy-folder-names \"%V\"", exe_str),
        )?;

        // 2. Right-clicking selected folder(s)
        let dir_key = REG_PATHS[1];
        set_reg_sz(dir_key, None, "Copy Selected Names (WinFlow)")?;
        set_reg_sz(dir_key, Some("Icon"), &icon_val)?;
        set_reg_sz(dir_key, Some("MultiSelectModel"), "Player")?;
        set_reg_sz(
            &format!(r"{}\command", dir_key),
            None,
            &format!("\"{}\" --copy-selected-names \"%1\"", exe_str),
        )?;

        // 3. Right-clicking selected file(s)
        let file_key = REG_PATHS[2];
        set_reg_sz(file_key, None, "Copy Selected Names (WinFlow)")?;
        set_reg_sz(file_key, Some("Icon"), &icon_val)?;
        set_reg_sz(file_key, Some("MultiSelectModel"), "Player")?;
        set_reg_sz(
            &format!(r"{}\command", file_key),
            None,
            &format!("\"{}\" --copy-selected-names \"%1\"", exe_str),
        )?;

        Ok(())
    }

    #[cfg(not(windows))]
    {
        let _ = enabled;
        Err("File Explorer context menu is only supported on Windows.".to_string())
    }
}

const POSTER_REG_PATHS: [&str; 2] = [
    r"Software\Classes\Directory\shell\WinFlowFindPoster",
    r"Software\Classes\Directory\shell\WinFlowResetPoster",
];

const THUMB_CLSID: &str = "{B7F9E3A1-4D2C-4E8B-9F1A-8C2D5E6F7A01}";
const THUMB_CLSID_KEY: &str = r"Software\Classes\CLSID\{B7F9E3A1-4D2C-4E8B-9F1A-8C2D5E6F7A01}";
const THUMB_FOLDER_SHELLEX_KEY: &str =
    r"Software\Classes\Folder\shellex\{e357fccd-a995-4576-b01f-234630154e96}";
const THUMB_DIR_SHELLEX_KEY: &str =
    r"Software\Classes\Directory\shellex\{e357fccd-a995-4576-b01f-234630154e96}";

#[cfg(windows)]
fn set_reg_dword(subkey: &str, value_name: &str, data: u32) -> Result<(), String> {
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, REG_DWORD,
    };
    let subkey_w = to_wide(subkey);
    let mut hkey: HKEY = std::ptr::null_mut();
    let status = unsafe { RegCreateKeyW(HKEY_CURRENT_USER, subkey_w.as_ptr(), &mut hkey) };
    if status != ERROR_SUCCESS {
        return Err(format!("RegCreateKeyW failed for '{}' (code {})", subkey, status));
    }
    let name_w = to_wide(value_name);
    let bytes = data.to_le_bytes();
    let set_status = unsafe {
        RegSetValueExW(
            hkey,
            name_w.as_ptr(),
            0,
            REG_DWORD,
            bytes.as_ptr(),
            bytes.len() as u32,
        )
    };
    unsafe {
        RegCloseKey(hkey);
    }
    if set_status != ERROR_SUCCESS {
        return Err(format!("RegSetValueExW DWORD failed (code {})", set_status));
    }
    Ok(())
}

pub fn is_thumb_dll_available() -> bool {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            if exe_dir.join("winflow_thumb.dll").is_file() {
                return true;
            }
            if let Some(rel) = exe_dir.parent().map(|p| p.join("release").join("winflow_thumb.dll")) {
                if rel.is_file() {
                    return true;
                }
            }
        }
    }
    false
}

/// Registers or unregisters the Movie Folder Poster Explorer context menu & IThumbnailProvider in HKCU.
pub fn set_poster_context_menu(enabled: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        if !enabled {
            for path in POSTER_REG_PATHS {
                delete_reg_tree(path);
            }
            delete_reg_tree(THUMB_CLSID_KEY);
            delete_reg_tree(THUMB_FOLDER_SHELLEX_KEY);
            delete_reg_tree(THUMB_DIR_SHELLEX_KEY);
            return Ok(());
        }

        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Could not resolve executable path: {}", e))?;
        let exe_str = exe_path.to_string_lossy().to_string();
        let icon_val = format!("{},0", exe_str);

        let find_key = POSTER_REG_PATHS[0];
        set_reg_sz(find_key, None, "Find Movie Poster (WinFlow)")?;
        set_reg_sz(find_key, Some("Icon"), &icon_val)?;
        set_reg_sz(find_key, Some("MultiSelectModel"), "Player")?;
        set_reg_sz(
            &format!(r"{}\command", find_key),
            None,
            &format!("\"{}\" --find-poster \"%1\"", exe_str),
        )?;

        let reset_key = POSTER_REG_PATHS[1];
        set_reg_sz(reset_key, None, "Reset Folder Icon (WinFlow)")?;
        set_reg_sz(reset_key, Some("Icon"), &icon_val)?;
        set_reg_sz(reset_key, Some("MultiSelectModel"), "Player")?;
        set_reg_sz(
            &format!(r"{}\command", reset_key),
            None,
            &format!("\"{}\" --reset-poster \"%1\"", exe_str),
        )?;

        // Register HD IThumbnailProvider COM DLL if present next to exe (or in target/release)
        if let Some(exe_dir) = exe_path.parent() {
            let local_dll = exe_dir.join("winflow_thumb.dll");
            let release_dll = exe_dir
                .parent()
                .map(|p| p.join("release").join("winflow_thumb.dll"));
            let dll_path = if local_dll.is_file() {
                Some(local_dll)
            } else {
                release_dll.filter(|p| p.is_file())
            };

            if let Some(dll) = dll_path {
                let dll_str = dll.to_string_lossy().to_string();
                let _ = set_reg_sz(THUMB_CLSID_KEY, None, "WinFlow Movie Poster Thumbnail Provider");
                let _ = set_reg_dword(THUMB_CLSID_KEY, "DisableProcessIsolation", 1);
                let _ = set_reg_sz(&format!(r"{}\InprocServer32", THUMB_CLSID_KEY), None, &dll_str);
                let _ = set_reg_sz(
                    &format!(r"{}\InprocServer32", THUMB_CLSID_KEY),
                    Some("ThreadingModel"),
                    "Apartment",
                );
                let _ = set_reg_sz(THUMB_FOLDER_SHELLEX_KEY, None, THUMB_CLSID);
                let _ = set_reg_sz(THUMB_DIR_SHELLEX_KEY, None, THUMB_CLSID);
                unsafe {
                    use windows_sys::Win32::UI::Shell::{
                        SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST,
                    };
                    SHChangeNotify(
                        SHCNE_ASSOCCHANGED as i32,
                        SHCNF_IDLIST,
                        std::ptr::null(),
                        std::ptr::null(),
                    );
                }
            }
        }

        Ok(())
    }

    #[cfg(not(windows))]
    {
        let _ = enabled;
        Err("File Explorer context menu is only supported on Windows.".to_string())
    }
}

/// Checks if the File Explorer context menu registry entry is currently present.
pub fn is_explorer_context_menu_enabled() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::ERROR_SUCCESS;
        use windows_sys::Win32::System::Registry::{
            RegCloseKey, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_READ,
        };

        let subkey_w = to_wide(REG_PATHS[0]);
        let mut hkey: HKEY = std::ptr::null_mut();
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                subkey_w.as_ptr(),
                0,
                KEY_READ,
                &mut hkey,
            )
        };
        if status == ERROR_SUCCESS {
            unsafe {
                RegCloseKey(hkey);
            }
            true
        } else {
            false
        }
    }

    #[cfg(not(windows))]
    {
        false
    }
}
