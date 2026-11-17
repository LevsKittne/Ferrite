use crate::log::{debug, error, info, warning};
use crate::sys;
use ferrite_sdk::abi::{Api, Handle};
use std::ffi::c_void;
use std::io::Read;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::Instant;
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

pub const ASSEMBLY: &str = "Ferrite.Il2Cpp";
const MIN_RUNTIME: u64 = 8;
const LOAD_ASSEMBLY_AND_GET_FUNCTION_POINTER: i32 = 5;
const UNMANAGED_CALLERS_ONLY: usize = usize::MAX;
const RUNTIME_CONFIG: &str = r#"{
  "runtimeOptions": {
    "tfm": "net8.0",
    "rollForward": "Major",
    "framework": {
      "name": "Microsoft.NETCore.App",
      "version": "8.0.0"
    },
    "configProperties": {
      "System.GC.Concurrent": false,
      "System.Globalization.Invariant": true
    }
  }
}
"#;

type InitializeFn = unsafe extern "C" fn(*const u16, *const InitParams, *mut *mut c_void) -> i32;
type GetDelegateFn = unsafe extern "C" fn(*mut c_void, i32, *mut *mut c_void) -> i32;
type ErrorWriterFn = unsafe extern "C" fn(*const u16);
type SetErrorWriterFn = unsafe extern "C" fn(Option<ErrorWriterFn>) -> Option<ErrorWriterFn>;
type LoadAssemblyFn = unsafe extern "C" fn(
    *const u16,
    *const u16,
    *const u16,
    usize,
    *mut c_void,
    *mut *mut c_void,
) -> i32;
type EntryFn = unsafe extern "C" fn(*const Api, *mut HostTable) -> i32;

#[repr(C)]
struct InitParams {
    size: usize,
    host_path: *const u16,
    dotnet_root: *const u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct HostTable {
    inspect: unsafe extern "C" fn(*const u16, i32, *mut u8, i32, *mut usize) -> i32,
    load: unsafe extern "C" fn(usize, Handle) -> i32,
    update: unsafe extern "C" fn(usize),
    scene: unsafe extern "C" fn(usize, *const u8, usize),
    config: unsafe extern "C" fn(usize),
    unload: unsafe extern "C" fn(usize),
    release: unsafe extern "C" fn(usize),
}

unsafe impl Send for HostTable {}
unsafe impl Sync for HostTable {}

#[derive(Debug)]
pub struct Info {
    pub token: usize,
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub dependencies: String,
}

static HOST: OnceLock<Option<HostTable>> = OnceLock::new();
static API: OnceLock<usize> = OnceLock::new();
static ROOT: OnceLock<PathBuf> = OnceLock::new();
static ERRORS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static REPORTED: AtomicBool = AtomicBool::new(false);

pub fn configure(root: &Path, api: *const Api) {
    let _ = ROOT.set(root.to_path_buf());
    let _ = API.set(api as usize);
}

pub fn is_clr_image(bytes: &[u8]) -> bool {
    let read_u16 = |at: usize| {
        bytes
            .get(at..at + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
    };
    let read_u32 = |at: usize| {
        bytes
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    if bytes.get(..2) != Some(b"MZ") {
        return false;
    }
    let Some(pe) = read_u32(0x3C).map(|v| v as usize) else {
        return false;
    };
    if bytes.get(pe..pe + 4) != Some(b"PE\0\0") {
        return false;
    }
    let optional = pe + 24;
    let directories = match read_u16(optional) {
        Some(0x10b) => optional + 96,
        Some(0x20b) => optional + 112,
        _ => return false,
    };
    let Some(count) = read_u32(directories - 4) else {
        return false;
    };
    count > 14 && read_u32(directories + 14 * 8).is_some_and(|rva| rva != 0)
}

pub fn is_managed(file: &Path) -> bool {
    let Ok(mut f) = std::fs::File::open(file) else {
        return false;
    };
    let mut head = vec![0u8; 4096];
    let Ok(n) = f.read(&mut head) else {
        return false;
    };
    head.truncate(n);
    is_clr_image(&head)
}

fn version_key(name: &str) -> Vec<u64> {
    name.split(['.', '-'])
        .map(|p| p.parse::<u64>().unwrap_or(0))
        .collect()
}

fn newest_fxr(dotnet_root: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dotnet_root.join("host").join("fxr")).ok()?;
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("hostfxr.dll").is_file())
        .max_by_key(|p| version_key(&p.file_name().unwrap_or_default().to_string_lossy()))
        .map(|p| p.join("hostfxr.dll"))
}

fn has_runtime(dotnet_root: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dotnet_root.join("shared").join("Microsoft.NETCore.App"))
    else {
        return false;
    };
    entries.flatten().any(|e| {
        version_key(&e.file_name().to_string_lossy())
            .first()
            .is_some_and(|&major| major >= MIN_RUNTIME)
    })
}

fn candidates(root: &Path) -> Vec<(PathBuf, bool)> {
    let mut out = vec![(root.join("dotnet"), true)];
    if let Some(dir) = std::env::var_os("DOTNET_ROOT") {
        out.push((PathBuf::from(dir), false));
    }
    for var in ["ProgramW6432", "ProgramFiles"] {
        if let Some(dir) = std::env::var_os(var) {
            out.push((PathBuf::from(dir).join("dotnet"), false));
        }
    }
    out.push((PathBuf::from(r"C:\Program Files\dotnet"), false));
    if let Some(dir) = std::env::var_os("LOCALAPPDATA") {
        out.push((PathBuf::from(dir).join("Microsoft").join("dotnet"), false));
    }
    out
}

fn find_dotnet(root: &Path) -> Option<(PathBuf, PathBuf, bool)> {
    candidates(root).into_iter().find_map(|(dir, bundled)| {
        let fxr = newest_fxr(&dir)?;
        has_runtime(&dir).then_some((dir, fxr, bundled))
    })
}

unsafe extern "C" fn error_writer(message: *const u16) {
    let text = unsafe { sys::from_wide_ptr(message) };
    ERRORS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(text);
}

fn flush_errors() {
    let errors = std::mem::take(&mut *ERRORS.lock().unwrap_or_else(PoisonError::into_inner));
    for e in errors {
        error!(".NET host: {}", e.trim_end());
    }
}

fn proc<T>(module: *mut c_void, name: &[u8]) -> Option<T> {
    let f = unsafe { GetProcAddress(module, name.as_ptr()) }?;
    Some(unsafe { std::mem::transmute_copy::<unsafe extern "system" fn() -> isize, T>(&f) })
}

fn runtime_config(core: &Path) -> PathBuf {
    let path = core.join(format!("{ASSEMBLY}.runtimeconfig.json"));
    if !path.is_file()
        && let Err(e) = std::fs::write(&path, RUNTIME_CONFIG)
    {
        warning!("cannot write {}: {e}", path.display());
    }
    path
}

fn runtime(dotnet_root: &Path, fxr: &Path, core: &Path) -> Option<LoadAssemblyFn> {
    let module = unsafe { LoadLibraryW(sys::wide(fxr).as_ptr()) };
    if module.is_null() {
        error!("cannot load {}", fxr.display());
        return None;
    }
    let (Some(initialize), Some(get_delegate), Some(set_writer)) = (
        proc::<InitializeFn>(module, b"hostfxr_initialize_for_runtime_config\0"),
        proc::<GetDelegateFn>(module, b"hostfxr_get_runtime_delegate\0"),
        proc::<SetErrorWriterFn>(module, b"hostfxr_set_error_writer\0"),
    ) else {
        error!("{} is not a usable hostfxr", fxr.display());
        return None;
    };
    unsafe { set_writer(Some(error_writer)) };
    let config = runtime_config(core);
    let host_path = sys::wide(sys::module_path(std::ptr::null_mut()));
    let dotnet_root_wide = sys::wide(dotnet_root);
    let params = InitParams {
        size: size_of::<InitParams>(),
        host_path: host_path.as_ptr(),
        dotnet_root: dotnet_root_wide.as_ptr(),
    };
    let mut context: *mut c_void = std::ptr::null_mut();
    let rc = unsafe {
        initialize(
            sys::wide(&config).as_ptr(),
            &raw const params,
            &raw mut context,
        )
    };
    if rc < 0 || context.is_null() {
        flush_errors();
        error!(
            "cannot start .NET from {} (hostfxr {:#x}), C# mods are skipped",
            dotnet_root.display(),
            rc.cast_unsigned()
        );
        return None;
    }
    let mut load: *mut c_void = std::ptr::null_mut();
    let rc = unsafe {
        get_delegate(
            context,
            LOAD_ASSEMBLY_AND_GET_FUNCTION_POINTER,
            &raw mut load,
        )
    };
    if rc < 0 || load.is_null() {
        flush_errors();
        error!(
            "hostfxr_get_runtime_delegate failed ({:#x})",
            rc.cast_unsigned()
        );
        return None;
    }
    Some(unsafe { std::mem::transmute::<*mut c_void, LoadAssemblyFn>(load) })
}

fn start() -> Option<HostTable> {
    let root = ROOT.get()?;
    let api = *API.get()? as *const Api;
    let core = root.join("core");
    let assembly = core.join(format!("{ASSEMBLY}.dll"));
    if !assembly.is_file() {
        error!(
            "C# mods found but {} is missing, reinstall Ferrite",
            assembly.display()
        );
        return None;
    }
    let Some((dotnet_root, fxr, bundled)) = find_dotnet(root) else {
        error!(
            "C# mods need the .NET {MIN_RUNTIME}+ runtime (x64), it was not found in Ferrite\\dotnet, DOTNET_ROOT or Program Files. Install it from https://dotnet.microsoft.com/download or copy a runtime into {}; C# mods are skipped",
            root.join("dotnet").display()
        );
        return None;
    };
    let started = Instant::now();
    let load = runtime(&dotnet_root, &fxr, &core)?;
    let mut entry: *mut c_void = std::ptr::null_mut();
    let rc = unsafe {
        load(
            sys::wide(&assembly).as_ptr(),
            sys::wide(format!("Ferrite.Host, {ASSEMBLY}")).as_ptr(),
            sys::wide("Init").as_ptr(),
            UNMANAGED_CALLERS_ONLY,
            std::ptr::null_mut(),
            &raw mut entry,
        )
    };
    if rc < 0 || entry.is_null() {
        flush_errors();
        error!(
            "cannot load {} ({:#x}), C# mods are skipped",
            assembly.display(),
            rc.cast_unsigned()
        );
        return None;
    }
    let entry = unsafe { std::mem::transmute::<*mut c_void, EntryFn>(entry) };
    let mut table = std::mem::MaybeUninit::<HostTable>::zeroed();
    let rc = unsafe { entry(api, table.as_mut_ptr()) };
    flush_errors();
    if rc != 0 {
        error!("{ASSEMBLY} failed to initialize ({rc}), C# mods are skipped");
        return None;
    }
    info!(
        ".NET runtime started from {}{} in {} ms",
        dotnet_root.display(),
        if bundled { " (bundled)" } else { "" },
        started.elapsed().as_millis()
    );
    Some(unsafe { table.assume_init() })
}

fn host() -> Option<HostTable> {
    *HOST.get_or_init(start)
}

fn started() -> Option<HostTable> {
    HOST.get().copied().flatten()
}

pub fn inspect(file: &Path) -> Option<Info> {
    let Some(host) = host() else {
        if !REPORTED.swap(true, Ordering::Relaxed) {
            warning!("C# mods are skipped, see the .NET error above");
        }
        warning!("skipping C# mod {}", file.display());
        return None;
    };
    let path: Vec<u16> = file.as_os_str().encode_wide().collect();
    let mut buffer = vec![0u8; 16 * 1024];
    let mut token = 0usize;
    let len = unsafe {
        (host.inspect)(
            path.as_ptr(),
            i32::try_from(path.len()).unwrap_or(0),
            buffer.as_mut_ptr(),
            i32::try_from(buffer.len()).unwrap_or(0),
            &raw mut token,
        )
    };
    let text = |n: i32| {
        let n = usize::try_from(n.unsigned_abs())
            .unwrap_or(0)
            .min(buffer.len());
        String::from_utf8_lossy(&buffer[..n]).into_owned()
    };
    if len == 0 {
        debug!("{} is not a Ferrite C# mod", file.display());
        return None;
    }
    if len < 0 {
        warning!("skipping {}: {}", file.display(), text(len));
        return None;
    }
    let raw = text(len);
    let mut parts = raw.split('\n').map(str::to_owned);
    let mut next = || parts.next().unwrap_or_default();
    Some(Info {
        token,
        id: next(),
        name: next(),
        version: next(),
        author: next(),
        dependencies: next(),
    })
}

pub fn load(token: usize, handle: usize) -> bool {
    started().is_some_and(|h| unsafe { (h.load)(token, handle as Handle) } != 0)
}

pub fn update(token: usize) {
    if let Some(h) = started() {
        unsafe { (h.update)(token) };
    }
}

pub fn scene(token: usize, name: &str) {
    if let Some(h) = started() {
        unsafe { (h.scene)(token, name.as_ptr(), name.len()) };
    }
}

pub fn config_changed(token: usize) {
    if let Some(h) = started() {
        unsafe { (h.config)(token) };
    }
}

pub fn unload(token: usize) {
    if let Some(h) = started() {
        unsafe { (h.unload)(token) };
    }
}

pub fn release(token: usize) {
    if let Some(h) = started() {
        unsafe { (h.release)(token) };
    }
}

#[cfg(test)]
mod tests {
    use super::{is_clr_image, version_key};

    fn image(magic: u16, clr: u32) -> Vec<u8> {
        let mut b = vec![0u8; 512];
        b[..2].copy_from_slice(b"MZ");
        b[0x3C..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        b[0x80..0x84].copy_from_slice(b"PE\0\0");
        let optional = 0x80 + 24;
        b[optional..optional + 2].copy_from_slice(&magic.to_le_bytes());
        let directories = optional + if magic == 0x20b { 112 } else { 96 };
        b[directories - 4..directories].copy_from_slice(&16u32.to_le_bytes());
        let com = directories + 14 * 8;
        b[com..com + 4].copy_from_slice(&clr.to_le_bytes());
        b
    }

    #[test]
    fn detects_clr_header() {
        assert!(is_clr_image(&image(0x10b, 0x2008)));
        assert!(is_clr_image(&image(0x20b, 0x2008)));
        assert!(!is_clr_image(&image(0x20b, 0)));
        assert!(!is_clr_image(b"MZ"));
        assert!(!is_clr_image(&[]));
    }

    #[test]
    fn orders_versions() {
        assert!(version_key("10.0.1") > version_key("8.0.31"));
        assert!(version_key("8.0.31") > version_key("8.0.4"));
    }
}
