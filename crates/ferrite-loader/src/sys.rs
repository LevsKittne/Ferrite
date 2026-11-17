use std::ffi::{CStr, c_void};
use std::path::{Path, PathBuf};

pub type Module = *mut c_void;

#[cfg(windows)]
pub unsafe fn c_bytes<'a>(p: *const u8) -> &'a [u8] {
    unsafe { CStr::from_ptr(p.cast()) }.to_bytes()
}

pub fn exe_path() -> PathBuf {
    imp::exe_path()
}

pub fn symbol(module: Module, name: &CStr) -> Option<*mut c_void> {
    imp::symbol(module, name)
}

pub fn open_library(file: &Path, search: Option<&Path>) -> Option<Module> {
    imp::open_library(file, search)
}

pub fn free_library(module: Module) {
    imp::free_library(module);
}

pub fn command_line() -> String {
    imp::command_line()
}

pub use imp::LIBRARY_EXTENSION;

#[cfg(windows)]
pub use imp::{from_wide_ptr, module_path, system_dir, wide};

#[cfg(windows)]
mod imp {
    use super::Module;
    use std::ffi::{CStr, OsStr, OsString, c_void};
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::{Path, PathBuf};
    use windows_sys::Win32::Foundation::FreeLibrary;
    use windows_sys::Win32::System::Environment::GetCommandLineW;
    use windows_sys::Win32::System::LibraryLoader::{
        AddDllDirectory, GetModuleFileNameW, GetProcAddress, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
        LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR, LOAD_LIBRARY_SEARCH_USER_DIRS, LoadLibraryExW,
    };
    use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;

    pub const LIBRARY_EXTENSION: &str = "dll";

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

    pub fn module_path(module: Module) -> PathBuf {
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

    pub fn exe_path() -> PathBuf {
        module_path(std::ptr::null_mut())
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

    pub fn symbol(module: Module, name: &CStr) -> Option<*mut c_void> {
        unsafe { GetProcAddress(module, name.as_ptr().cast()) }.map(|f| f as *mut c_void)
    }

    pub fn open_library(file: &Path, search: Option<&Path>) -> Option<Module> {
        let module = if let Some(dir) = search {
            unsafe { AddDllDirectory(wide(dir).as_ptr()) };
            unsafe {
                LoadLibraryExW(
                    wide(file).as_ptr(),
                    std::ptr::null_mut(),
                    LOAD_LIBRARY_SEARCH_DEFAULT_DIRS | LOAD_LIBRARY_SEARCH_USER_DIRS,
                )
            }
        } else {
            unsafe {
                LoadLibraryExW(
                    wide(file).as_ptr(),
                    std::ptr::null_mut(),
                    LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
                )
            }
        };
        (!module.is_null()).then_some(module)
    }

    pub fn free_library(module: Module) {
        unsafe { FreeLibrary(module) };
    }

    pub fn command_line() -> String {
        unsafe { from_wide_ptr(GetCommandLineW()) }
    }
}

#[cfg(unix)]
mod imp {
    use super::Module;
    use std::ffi::{CStr, CString, c_void};
    use std::os::unix::ffi::OsStrExt;
    use std::path::{Path, PathBuf};

    #[cfg(target_os = "macos")]
    pub const LIBRARY_EXTENSION: &str = "dylib";
    #[cfg(not(target_os = "macos"))]
    pub const LIBRARY_EXTENSION: &str = "so";

    pub fn exe_path() -> PathBuf {
        std::env::current_exe().unwrap_or_default()
    }

    pub fn symbol(module: Module, name: &CStr) -> Option<*mut c_void> {
        let f = unsafe { crate::unix::real_dlsym(module, name.as_ptr()) };
        (!f.is_null()).then_some(f)
    }

    pub fn open_library(file: &Path, _search: Option<&Path>) -> Option<Module> {
        let path = CString::new(file.as_os_str().as_bytes()).ok()?;
        let module = unsafe { libc::dlopen(path.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
        (!module.is_null()).then_some(module)
    }

    pub fn free_library(module: Module) {
        unsafe { libc::dlclose(module) };
    }

    #[cfg(target_os = "linux")]
    pub fn command_line() -> String {
        std::fs::read("/proc/self/cmdline")
            .map(|raw| {
                raw.split(|&b| b == 0)
                    .map(String::from_utf8_lossy)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default()
    }

    #[cfg(not(target_os = "linux"))]
    pub fn command_line() -> String {
        std::env::args().collect::<Vec<_>>().join(" ")
    }
}
