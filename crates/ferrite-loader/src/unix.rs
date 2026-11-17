use std::ffi::{CStr, c_char, c_void};
use std::path::PathBuf;

pub mod overlay {
    use crate::log::warning;
    use std::path::PathBuf;

    #[allow(dead_code)]
    #[derive(Debug, Clone)]
    pub struct ModRow {
        pub id: String,
        pub name: String,
        pub version: String,
        pub author: String,
        pub kind: &'static str,
        pub faulted: bool,
    }

    pub fn init(_root: PathBuf, _runtime: &'static str, _menu_key: &str) {
        warning!("the in-game menu is only available on Windows, use the console");
    }

    pub fn is_open() -> bool {
        false
    }

    pub fn toggle() {
        warning!("the in-game menu is only available on Windows");
    }

    pub fn set_mods(_kind: &'static str, _rows: Vec<ModRow>) {}
}

#[cfg(target_os = "linux")]
#[used]
#[unsafe(link_section = ".init_array")]
static INIT: extern "C" fn() = init;

#[cfg(target_os = "macos")]
#[used]
#[unsafe(link_section = "__DATA,__mod_init_func")]
static INIT: extern "C" fn() = init;

extern "C" fn init() {
    let _ = std::panic::catch_unwind(start);
}

fn start() {
    let exe = crate::sys::exe_path();
    let Some(game_dir) = exe.parent().map(PathBuf::from) else {
        return;
    };
    let root =
        std::env::var_os("FERRITE_DIR").map_or_else(|| default_root(&game_dir), PathBuf::from);
    if crate::attach(&game_dir, root) {
        crate::log::info!("dlsym interposed, waiting for the Unity runtime");
    }
}

#[cfg(target_os = "macos")]
fn default_root(game_dir: &std::path::Path) -> PathBuf {
    let bundle = game_dir
        .ancestors()
        .find(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("app")));
    bundle
        .and_then(std::path::Path::parent)
        .unwrap_or(game_dir)
        .join(crate::ROOT_DIR)
}

#[cfg(not(target_os = "macos"))]
fn default_root(game_dir: &std::path::Path) -> PathBuf {
    game_dir.join(crate::ROOT_DIR)
}

#[cfg(target_os = "linux")]
type DlsymFn = unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void;

#[cfg(target_os = "linux")]
fn next_dlsym() -> Option<DlsymFn> {
    use std::sync::OnceLock;
    static NEXT: OnceLock<usize> = OnceLock::new();
    let f = *NEXT.get_or_init(|| {
        [c"GLIBC_2.34", c"GLIBC_2.2.5"]
            .into_iter()
            .map(|version| unsafe {
                libc::dlvsym(libc::RTLD_NEXT, c"dlsym".as_ptr(), version.as_ptr())
            })
            .find(|f| !f.is_null())
            .map_or(0, |f| f as usize)
    });
    (f != 0).then(|| unsafe { std::mem::transmute::<usize, DlsymFn>(f) })
}

#[cfg(target_os = "linux")]
pub unsafe fn real_dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void {
    next_dlsym().map_or(std::ptr::null_mut(), |f| unsafe { f(handle, name) })
}

#[cfg(not(target_os = "linux"))]
pub unsafe fn real_dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void {
    unsafe { libc::dlsym(handle, name) }
}

unsafe fn hooked_dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void {
    let real = unsafe { real_dlsym(handle, name) };
    if real.is_null() || name.is_null() || !crate::active() {
        return real;
    }
    let bytes = unsafe { CStr::from_ptr(name) }.to_bytes();
    crate::intercept(handle, bytes, real)
}

#[cfg(target_os = "linux")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void {
    unsafe { hooked_dlsym(handle, name) }
}

#[cfg(target_os = "macos")]
unsafe extern "C" fn interposed_dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void {
    unsafe { hooked_dlsym(handle, name) }
}

#[cfg(target_os = "macos")]
#[repr(C)]
struct Interpose {
    replacement: *const c_void,
    original: *const c_void,
}

#[cfg(target_os = "macos")]
unsafe impl Sync for Interpose {}

#[cfg(target_os = "macos")]
#[used]
#[unsafe(link_section = "__DATA,__interpose")]
static INTERPOSE: Interpose = Interpose {
    replacement: interposed_dlsym as *const c_void,
    original: libc::dlsym as *const c_void,
};
