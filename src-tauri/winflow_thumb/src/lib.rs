use image::imageops::FilterType;
use image::GenericImageView;
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use windows::core::{implement, IUnknown, Interface, GUID, HRESULT, PCWSTR};
use windows::Win32::Foundation::{CLASS_E_CLASSNOTAVAILABLE, E_FAIL, E_NOINTERFACE, HINSTANCE, S_FALSE, S_OK};
use windows::Win32::Graphics::Gdi::{
    CreateDIBSection, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HDC,
};
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl};
use windows::Win32::UI::Shell::PropertiesSystem::{
    IInitializeWithFile, IInitializeWithFile_Impl,
};
use windows::Win32::UI::Shell::{
    IInitializeWithItem, IInitializeWithItem_Impl, IShellItem, IThumbnailProvider,
    IThumbnailProvider_Impl, SIGDN_FILESYSPATH, WTSAT_ARGB, WTS_ALPHATYPE,
};

// {B7F9E3A1-4D2C-4E8B-9F1A-8C2D5E6F7A01}
pub const CLSID_WINFLOW_THUMB: GUID = GUID::from_u128(0xb7f9e3a1_4d2c_4e8b_9f1a_8c2d5e6f7a01);

static OBJECT_COUNT: AtomicU32 = AtomicU32::new(0);

#[implement(IThumbnailProvider, IInitializeWithItem, IInitializeWithFile)]
struct PosterThumbnailProvider {
    folder_path: Mutex<Option<PathBuf>>,
}

impl PosterThumbnailProvider {
    fn new() -> Self {
        OBJECT_COUNT.fetch_add(1, Ordering::SeqCst);
        Self {
            folder_path: Mutex::new(None),
        }
    }
}

impl Drop for PosterThumbnailProvider {
    fn drop(&mut self) {
        OBJECT_COUNT.fetch_sub(1, Ordering::SeqCst);
    }
}

impl IInitializeWithItem_Impl for PosterThumbnailProvider_Impl {
    fn Initialize(&self, psi: Option<&IShellItem>, _grfmode: u32) -> windows::core::Result<()> {
        let Some(item) = psi else {
            return Err(E_FAIL.into());
        };
        let pwstr = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH)? };
        let path_str = unsafe { pwstr.to_string().unwrap_or_default() };
        unsafe {
            windows::Win32::System::Com::CoTaskMemFree(Some(pwstr.0 as *const c_void));
        }
        if path_str.is_empty() {
            return Err(E_FAIL.into());
        }
        *self.folder_path.lock().unwrap() = Some(PathBuf::from(path_str));
        Ok(())
    }
}

impl IInitializeWithFile_Impl for PosterThumbnailProvider_Impl {
    fn Initialize(&self, pszfilepath: &PCWSTR, _grfmode: u32) -> windows::core::Result<()> {
        let path_str = unsafe { pszfilepath.to_string().unwrap_or_default() };
        if path_str.is_empty() {
            return Err(E_FAIL.into());
        }
        *self.folder_path.lock().unwrap() = Some(PathBuf::from(path_str));
        Ok(())
    }
}

fn find_poster_file(folder: &Path) -> Option<PathBuf> {
    for name in ["poster.jpg", "poster.png", "poster.jpeg"] {
        let p = folder.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

impl IThumbnailProvider_Impl for PosterThumbnailProvider_Impl {
    fn GetThumbnail(
        &self,
        cx: u32,
        phbmp: *mut HBITMAP,
        pdwalpha: *mut WTS_ALPHATYPE,
    ) -> windows::core::Result<()> {
        let folder = self
            .folder_path
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| windows::core::Error::from(E_FAIL))?;

        let poster_path =
            find_poster_file(&folder).ok_or_else(|| windows::core::Error::from(E_FAIL))?;

        let img = image::open(&poster_path).map_err(|_| windows::core::Error::from(E_FAIL))?;
        let (sw, sh) = img.dimensions();
        if sw == 0 || sh == 0 || cx == 0 {
            return Err(E_FAIL.into());
        }

        let target = cx.clamp(16, 1024);
        let scale = (target as f32 / sw as f32).min(target as f32 / sh as f32);
        let nw = ((sw as f32 * scale).round() as u32).clamp(1, target);
        let nh = ((sh as f32 * scale).round() as u32).clamp(1, target);

        let resized = img.resize_exact(nw, nh, FilterType::Lanczos3).to_rgba8();

        let mut bmi: BITMAPINFO = unsafe { std::mem::zeroed() };
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = nw as i32;
        bmi.bmiHeader.biHeight = -(nh as i32); // top-down DIB
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB.0;

        let mut bits_ptr: *mut c_void = std::ptr::null_mut();
        let hbmp = unsafe {
            CreateDIBSection(HDC::default(), &bmi, DIB_RGB_COLORS, &mut bits_ptr, None, 0)?
        };

        if hbmp.is_invalid() || bits_ptr.is_null() {
            return Err(E_FAIL.into());
        }

        // Convert RGBA -> BGRA premultiplied into DIB memory
        let pixel_count = (nw * nh) as usize;
        let dst = unsafe { std::slice::from_raw_parts_mut(bits_ptr as *mut u8, pixel_count * 4) };
        let src = resized.as_raw();
        for (d, s) in dst.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
            d[0] = s[2]; // B
            d[1] = s[1]; // G
            d[2] = s[0]; // R
            d[3] = s[3]; // A
        }

        unsafe {
            *phbmp = hbmp;
            if !pdwalpha.is_null() {
                *pdwalpha = WTSAT_ARGB;
            }
        }

        Ok(())
    }
}

#[implement(IClassFactory)]
struct ClassFactory;

impl IClassFactory_Impl for ClassFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: Option<&IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut c_void,
    ) -> windows::core::Result<()> {
        if punkouter.is_some() {
            return Err(E_NOINTERFACE.into());
        }
        let provider: IUnknown = PosterThumbnailProvider::new().into();
        let hr = unsafe { provider.query(&*riid, ppvobject) };
        hr.ok()
    }

    fn LockServer(&self, _flock: windows::Win32::Foundation::BOOL) -> windows::core::Result<()> {
        Ok(())
    }
}

#[no_mangle]
pub extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if rclsid.is_null() || riid.is_null() || ppv.is_null() {
        return E_FAIL;
    }
    unsafe {
        *ppv = std::ptr::null_mut();
        if *rclsid != CLSID_WINFLOW_THUMB {
            return CLASS_E_CLASSNOTAVAILABLE;
        }
        let factory: IUnknown = ClassFactory.into();
        factory.query(&*riid, ppv)
    }
}

#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    if OBJECT_COUNT.load(Ordering::SeqCst) == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

#[no_mangle]
pub extern "system" fn DllMain(
    _hinst: HINSTANCE,
    _reason: u32,
    _reserved: *mut c_void,
) -> windows::Win32::Foundation::BOOL {
    true.into()
}
