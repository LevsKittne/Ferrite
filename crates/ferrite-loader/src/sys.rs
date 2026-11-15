use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;
use windows_sys::Win32::Foundation::HMODULE;
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;

pub fn wide(s: impl AsRef<OsStr>) -> Vec<u16> {
    s.as_ref().encode_wide().chain(Some(0)).collect()
}

pub unsafe fn from_wide_ptr(p: *const u16) -> String {
    if p.is_null() {
        return String::new();
    }
    let mut len = 0;
    while unsafe { *p.add(len) } != 0 {
        len += 1;
    }
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(p, len) })
}

pub unsafe fn c_bytes<'a>(p: *const u8) -> &'a [u8] {
    unsafe { std::ffi::CStr::from_ptr(p.cast()) }.to_bytes()
}

pub fn module_path(module: HMODULE) -> PathBuf {
    let mut buf = vec![0u16; 32_768];
    let len = unsafe {
        GetModuleFileNameW(
            module,
            buf.as_mut_ptr(),
            u32::try_from(buf.len()).unwrap_or(u32::MAX),
        )
    };
    buf.truncate(len as usize);
    PathBuf::from(OsString::from_wide(&buf))
}

pub fn system_dir() -> PathBuf {
    let mut buf = vec![0u16; 1024];
    let len = unsafe {
        GetSystemDirectoryW(
            buf.as_mut_ptr(),
            u32::try_from(buf.len()).unwrap_or(u32::MAX),
        )
    };
    buf.truncate(len as usize);
    PathBuf::from(OsString::from_wide(&buf))
}
