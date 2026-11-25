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
