use image::codecs::png::PngEncoder;
use image::imageops::FilterType;
use image::{ColorType, GenericImageView, ImageEncoder, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PosterCandidate {
    pub title: String,
    pub year: String,
    pub poster_url: String,
    pub source: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SubfolderScanItem {
    pub path: String,
    pub name: String,
    pub parsed_query: String,
    pub has_poster: bool,
}

/// Cleans a messy movie folder name (e.g. "Inception.2010.1080p.BluRay.x264") into ("Inception", Some("2010")).
pub fn parse_movie_folder_name(raw_name: &str) -> (String, Option<String>) {
    let normalized = raw_name
        .replace('.', " ")
        .replace('_', " ")
        .replace(['[', ']', '(', ')'], " ");

    let stop_tokens = [
        "1080p", "2160p", "720p", "480p", "4k", "uhd", "hdr", "hdr10", "dv", "bluray", "bdrip",
        "brrip", "webrip", "web-dl", "webdl", "web", "hdrip", "dvdrip", "x264", "x265", "h264",
        "h265", "hevc", "avc", "aac", "dts", "ac3", "atmos", "truehd", "remux", "extended",
        "unrated", "directors", "cut", "proper", "repack", "yify", "yts", "rarbg", "evo",
    ];

    let mut title_parts: Vec<String> = Vec::new();
    let mut found_year: Option<String> = None;

    for token in normalized.split_whitespace() {
        let lower = token.to_lowercase();
        if token.len() == 4 && token.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(y) = token.parse::<u32>() {
                if (1920..=2035).contains(&y) {
                    found_year = Some(token.to_string());
                    break;
                }
            }
        }
        if stop_tokens.contains(&lower.as_str()) {
            break;
        }
        title_parts.push(token.to_string());
    }

    let clean_title = if title_parts.is_empty() {
        raw_name.trim().to_string()
    } else {
        title_parts.join(" ")
    };

    (clean_title, found_year)
}

#[derive(Deserialize)]
struct ImdbResponse {
    #[serde(default)]
    d: Vec<ImdbItem>,
}

#[derive(Deserialize)]
struct ImdbImage {
    #[serde(rename = "imageUrl")]
    image_url: Option<String>,
}

#[derive(Deserialize)]
struct ImdbItem {
    l: Option<String>,
    y: Option<u32>,
    qid: Option<String>,
    i: Option<ImdbImage>,
}

/// Searches IMDb's public suggestion endpoint (zero API key required) for movie posters.
pub fn search_itunes_posters(query: &str, year_hint: Option<&str>) -> Result<Vec<PosterCandidate>, String> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(Vec::new());
    }

    let search_term = match year_hint {
        Some(y) if !y.is_empty() => format!("{} {}", q, y),
        _ => q.to_string(),
    };

    // URL-encode query for path segment
    let encoded: String = search_term
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => (b as char).to_string(),
            b' ' => "%20".to_string(),
            _ => format!("%{:02X}", b),
        })
        .collect();

    let url = format!("https://v3.sg.media-imdb.com/suggestion/x/{}.json", encoded);
    let resp = ureq::get(&url)
        .set("User-Agent", "WinFlow/0.2.5")
        .timeout(std::time::Duration::from_secs(10))
        .call()
        .map_err(|e| format!("Movie search failed: {}", e))?;

    let parsed: ImdbResponse = resp
        .into_json()
        .map_err(|e| format!("Failed to parse movie search response: {}", e))?;

    let mut out = Vec::new();
    for item in parsed.d {
        let qid = item.qid.as_deref().unwrap_or("");
        if qid != "movie" && qid != "tvMovie" && qid != "tvSeries" && qid != "video" {
            continue;
        }
        let Some(title) = item.l else {
            continue;
        };
        let Some(img_url) = item.i.and_then(|i| i.image_url) else {
            continue;
        };
        if img_url.is_empty() {
            continue;
        }
        // Use high-resolution 1000px-wide IMDb CDN variant if standard _V1_.jpg
        let poster_url = if img_url.ends_with("._V1_.jpg") {
            img_url.replace("._V1_.jpg", "._V1_UX1000_.jpg")
        } else {
            img_url
        };
        let year = item.y.map(|y| y.to_string()).unwrap_or_default();

        out.push(PosterCandidate {
            title,
            year,
            poster_url,
            source: "IMDb".to_string(),
        });
    }

    if let Some(target_year) = year_hint.filter(|y| !y.is_empty()) {
        out.sort_by_key(|c| if c.year == target_year { 0 } else { 1 });
    }

    Ok(out)
}

/// Downloads (or reads from disk/data URL) a poster image and applies it to `folder_path`.
pub fn apply_poster_to_folder(folder_path: &str, poster_url: &str) -> Result<(), String> {
    let folder = Path::new(folder_path);
    if !folder.is_dir() {
        return Err(format!("Folder does not exist: {}", folder_path));
    }

    let source = poster_url.trim().trim_matches('"');
    let img_bytes: Vec<u8> = if let Some(comma_idx) = source.find(";base64,") {
        if source.starts_with("data:image/") {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD
                .decode(&source[comma_idx + 8..])
                .map_err(|e| format!("Failed to decode uploaded image: {}", e))?
        } else {
            return Err("Unsupported data URL format.".to_string());
        }
    } else if source.starts_with("http://") || source.starts_with("https://") {
        let resp = ureq::get(source)
            .timeout(std::time::Duration::from_secs(15))
            .call()
            .map_err(|e| format!("Failed to download poster: {}", e))?;

        let mut buf = Vec::new();
        use std::io::Read;
        resp.into_reader()
            .take(20 * 1024 * 1024)
            .read_to_end(&mut buf)
            .map_err(|e| format!("Failed to read poster bytes: {}", e))?;
        buf
    } else if Path::new(source).is_file() {
        std::fs::read(source).map_err(|e| format!("Failed to read local poster file: {}", e))?
    } else {
        return Err("Invalid poster URL or file path.".to_string());
    };

    let src = image::load_from_memory(&img_bytes)
        .map_err(|e| format!("Failed to decode poster image: {}", e))?;

    // Build crisp 256x256 lossless PNG-in-ICO directly per Windows ICO spec
    let (sw, sh) = src.dimensions();
    let dim = 256u32;
    let scale = (dim as f32 / sw as f32).min(dim as f32 / sh as f32);
    let nw = ((sw as f32 * scale).round() as u32).clamp(1, dim);
    let nh = ((sh as f32 * scale).round() as u32).clamp(1, dim);
    let resized = src.resize_exact(nw, nh, FilterType::Lanczos3).to_rgba8();
    let sharpened = image::imageops::unsharpen(&resized, 0.85, 2);

    let mut canvas = RgbaImage::from_pixel(dim, dim, Rgba([0, 0, 0, 0]));
    let off_x = (dim - nw) / 2;
    let off_y = (dim - nh) / 2;
    image::imageops::overlay(&mut canvas, &sharpened, off_x as i64, off_y as i64);

    let mut png_buf = Vec::new();
    PngEncoder::new(&mut png_buf)
        .write_image(canvas.as_raw(), dim, dim, ColorType::Rgba8.into())
        .map_err(|e| format!("Failed to encode 256px PNG: {}", e))?;

    let ts = chrono::Local::now().timestamp_millis();
    let ico_filename = format!("poster_{}.ico", ts);
    let ico_path = folder.join(&ico_filename);
    let ini_path = folder.join("desktop.ini");

    // Remove any previous poster*.ico, poster.jpg/png, and desktop.ini so Windows Explorer never uses a stale cached icon
    if let Ok(entries) = std::fs::read_dir(folder) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if (name.starts_with("poster") && (name.ends_with(".ico") || name.ends_with(".jpg") || name.ends_with(".png") || name.ends_with(".jpeg")))
                || name == "desktop.ini"
            {
                let p = entry.path();
                clear_file_attrs(&p);
                let _ = std::fs::remove_file(&p);
            }
        }
    }

    // Save full-resolution poster.jpg (used by winflow_thumb.dll IThumbnailProvider for up to 1024px HD thumbnails)
    let hd_poster_path = folder.join("poster.jpg");
    {
        src.to_rgb8()
            .save_with_format(&hd_poster_path, image::ImageFormat::Jpeg)
            .map_err(|e| format!("Failed to save HD poster.jpg: {}", e))?;
        set_hidden_system_attrs(&hd_poster_path);
    }

    // If winflow_thumb.dll is available, let IThumbnailProvider render the full HD poster.jpg
    // without desktop.ini IconResource overriding it with a 256px .ico.
    if crate::explorer_menu::is_thumb_dll_available() {
        set_folder_readonly_attr(folder, false);
        notify_shell_folder_updated(folder);
        return Ok(());
    }

    // Fallback: Write crisp 256x256 lossless PNG-in-ICO + desktop.ini via SHGetSetFolderCustomSettings

    // Write standard 22-byte Windows ICO header + PNG payload
    {
        use std::io::Write;
        let mut ico_file = std::fs::File::create(&ico_path)
            .map_err(|e| format!("Failed to create poster.ico: {}", e))?;

        let png_len = png_buf.len() as u32;
        let mut header = [0u8; 22];
        // ICONDIR: reserved(2)=0, type(2)=1 (ICO), count(2)=1
        header[2] = 1;
        header[4] = 1;
        // ICONDIRENTRY: width=0(256), height=0(256), colors=0, reserved=0
        // planes(2)=1, bitcount(2)=32
        header[10] = 1;
        header[12] = 32;
        // bytes_in_res(4)
        header[14..18].copy_from_slice(&png_len.to_le_bytes());
        // image_offset(4) = 22
        header[18..22].copy_from_slice(&22u32.to_le_bytes());

        ico_file
            .write_all(&header)
            .and_then(|_| ico_file.write_all(&png_buf))
            .and_then(|_| ico_file.flush())
            .map_err(|e| format!("Failed to write poster.ico: {}", e))?;
    }

    set_hidden_system_attrs(&ico_path);

    // Use official Win32 SHGetSetFolderCustomSettings (with desktop.ini fallback)
    #[cfg(windows)]
    {
        use windows::core::PWSTR;
        use windows::Win32::UI::Shell::{
            SHGetSetFolderCustomSettings, FCSM_ICONFILE, FCS_FORCEWRITE, SHFOLDERCUSTOMSETTINGS,
        };

        let mut ico_wide = to_wide(&ico_path);
        let folder_wide = to_wide(folder);
        let mut fcs = SHFOLDERCUSTOMSETTINGS {
            dwSize: std::mem::size_of::<SHFOLDERCUSTOMSETTINGS>() as u32,
            dwMask: FCSM_ICONFILE,
            pszIconFile: PWSTR(ico_wide.as_mut_ptr()),
            cchIconFile: 0,
            iIconIndex: 0,
            ..Default::default()
        };

        let hr = unsafe {
            SHGetSetFolderCustomSettings(
                &mut fcs,
                windows::core::PCWSTR(folder_wide.as_ptr()),
                FCS_FORCEWRITE,
            )
        };

        if hr.is_err() {
            let ini_content = format!(
                "[.ShellClassInfo]\r\nIconResource={},0\r\n",
                ico_path.to_string_lossy()
            );
            std::fs::write(&ini_path, ini_content)
                .map_err(|e| format!("Failed to write desktop.ini: {}", e))?;
            set_hidden_system_attrs(&ini_path);
            set_folder_readonly_attr(folder, true);
        }
    }

    #[cfg(not(windows))]
    {
        let _ = ini_path;
    }

    notify_shell_folder_updated(folder);

    Ok(())
}

/// Removes `desktop.ini` and `poster.ico` from a folder and restores its default Windows icon.
pub fn reset_folder_poster(folder_path: &str) -> Result<(), String> {
    let folder = Path::new(folder_path);
    if !folder.is_dir() {
        return Err(format!("Folder does not exist: {}", folder_path));
    }

    if let Ok(entries) = std::fs::read_dir(folder) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if (name.starts_with("poster") && (name.ends_with(".ico") || name.ends_with(".jpg") || name.ends_with(".png") || name.ends_with(".jpeg")))
                || name == "desktop.ini"
            {
                let p = entry.path();
                clear_file_attrs(&p);
                let _ = std::fs::remove_file(&p);
            }
        }
    }

    set_folder_readonly_attr(folder, false);
    notify_shell_folder_updated(folder);
    Ok(())
}

/// Lists immediate subfolders of a parent directory for batch poster scanning.
pub fn scan_movie_subfolders(parent_path: &str) -> Result<Vec<SubfolderScanItem>, String> {
    let parent = Path::new(parent_path);
    if !parent.is_dir() {
        return Err(format!("Folder does not exist: {}", parent_path));
    }

    let entries = std::fs::read_dir(parent)
        .map_err(|e| format!("Failed to read folder: {}", e))?;

    let mut items = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let (clean, year) = parse_movie_folder_name(&name);
        let parsed_query = match year {
            Some(y) => format!("{} {}", clean, y),
            None => clean,
        };
        let has_poster = path.join("poster.jpg").exists()
            || (path.join("desktop.ini").exists()
                && std::fs::read_dir(&path)
                    .map(|rd| {
                        rd.flatten().any(|e| {
                            let n = e.file_name().to_string_lossy().to_lowercase();
                            n.starts_with("poster") && n.ends_with(".ico")
                        })
                    })
                    .unwrap_or(false));
        items.push(SubfolderScanItem {
            path: path.to_string_lossy().to_string(),
            name,
            parsed_query,
            has_poster,
        });
    }

    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(items)
}

#[cfg(windows)]
fn to_wide(p: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    p.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(windows)]
fn clear_file_attrs(path: &Path) {
    use windows_sys::Win32::Storage::FileSystem::{SetFileAttributesW, FILE_ATTRIBUTE_NORMAL};
    if path.exists() {
        let w = to_wide(path);
        unsafe {
            SetFileAttributesW(w.as_ptr(), FILE_ATTRIBUTE_NORMAL);
        }
    }
}

#[cfg(windows)]
fn set_hidden_system_attrs(path: &Path) {
    use windows_sys::Win32::Storage::FileSystem::{
        SetFileAttributesW, FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_SYSTEM,
    };
    let w = to_wide(path);
    unsafe {
        SetFileAttributesW(w.as_ptr(), FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM);
    }
}

#[cfg(windows)]
fn set_folder_readonly_attr(folder: &Path, readonly: bool) {
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileAttributesW, SetFileAttributesW, FILE_ATTRIBUTE_READONLY, INVALID_FILE_ATTRIBUTES,
    };
    let w = to_wide(folder);
    unsafe {
        let attrs = GetFileAttributesW(w.as_ptr());
        if attrs != INVALID_FILE_ATTRIBUTES {
            let next = if readonly {
                attrs | FILE_ATTRIBUTE_READONLY
            } else {
                attrs & !FILE_ATTRIBUTE_READONLY
            };
            SetFileAttributesW(w.as_ptr(), next);
        }
    }
}

#[cfg(windows)]
fn notify_shell_folder_updated(folder: &Path) {
    use windows_sys::Win32::UI::Shell::{
        SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNE_ATTRIBUTES, SHCNE_UPDATEDIR, SHCNE_UPDATEITEM,
        SHCNF_FLUSH, SHCNF_IDLIST, SHCNF_PATHW,
    };
    let w = to_wide(folder);
    unsafe {
        SHChangeNotify(
            SHCNE_ATTRIBUTES as i32,
            SHCNF_PATHW | SHCNF_FLUSH,
            w.as_ptr() as *const _,
            std::ptr::null(),
        );
        SHChangeNotify(
            SHCNE_UPDATEITEM as i32,
            SHCNF_PATHW | SHCNF_FLUSH,
            w.as_ptr() as *const _,
            std::ptr::null(),
        );
        if let Some(parent) = folder.parent() {
            let pw = to_wide(parent);
            SHChangeNotify(
                SHCNE_UPDATEDIR as i32,
                SHCNF_PATHW | SHCNF_FLUSH,
                pw.as_ptr() as *const _,
                std::ptr::null(),
            );
        }
        SHChangeNotify(
            SHCNE_ASSOCCHANGED as i32,
            SHCNF_IDLIST | SHCNF_FLUSH,
            std::ptr::null(),
            std::ptr::null(),
        );
    }
}

#[cfg(not(windows))]
fn clear_file_attrs(_path: &Path) {}
#[cfg(not(windows))]
fn set_hidden_system_attrs(_path: &Path) {}
#[cfg(not(windows))]
fn set_folder_readonly_attr(_folder: &Path, _readonly: bool) {}
#[cfg(not(windows))]
fn notify_shell_folder_updated(_folder: &Path) {}

/// Writes a pending folder path to `%TEMP%\winflow_find_poster_batch.txt` for the running Hub to pick up.
pub fn write_pending_poster_folder(folder_path: &str) {
    let p: PathBuf = std::env::temp_dir().join("winflow_find_poster_batch.txt");
    let _ = std::fs::write(p, format!("{}\n", folder_path));
}

pub fn take_pending_poster_folders() -> Vec<SubfolderScanItem> {
    let p: PathBuf = std::env::temp_dir().join("winflow_find_poster_batch.txt");
    let Ok(content) = std::fs::read_to_string(&p) else {
        return Vec::new();
    };
    let _ = std::fs::remove_file(&p);

    let mut seen = std::collections::HashSet::new();
    let mut items = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || !seen.insert(trimmed.to_string()) {
            continue;
        }
        let path = Path::new(trimmed);
        if !path.is_dir() {
            continue;
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| trimmed.to_string());
        let (clean, year) = parse_movie_folder_name(&name);
        let parsed_query = match year {
            Some(y) => format!("{} {}", clean, y),
            None => clean,
        };
        let has_poster = path.join("poster.jpg").exists()
            || (path.join("desktop.ini").exists()
                && std::fs::read_dir(path)
                    .map(|rd| {
                        rd.flatten().any(|e| {
                            let n = e.file_name().to_string_lossy().to_lowercase();
                            n.starts_with("poster") && n.ends_with(".ico")
                        })
                    })
                    .unwrap_or(false));

        items.push(SubfolderScanItem {
            path: trimmed.to_string(),
            name,
            parsed_query,
            has_poster,
        });
    }

    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    items
}
