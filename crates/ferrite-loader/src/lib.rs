mod config;
mod console;
mod deps;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod detour;
#[cfg(windows)]
mod iat;
mod il2cpp;
mod log;
mod mods;
mod mono;
mod native;
#[cfg(windows)]
mod overlay;
mod patched;
#[cfg(windows)]
mod proxy;
mod sys;
#[cfg(unix)]
mod unix;

#[cfg(unix)]
use unix::overlay;

use crate::config::{Kind, LoaderConfig, ModConfigs};
use crate::log::{Level, error, info, warning};
use crate::mono::{Class, Mono, Ptr};
use std::ffi::{c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use sys::Module;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const ROOT_DIR: &str = "Ferrite";
const API_NAMESPACE: &str = "Ferrite";

type JitInit = unsafe extern "C" fn(*const c_char, *const c_char) -> Ptr;
type Il2CppInit = unsafe extern "C" fn(*const c_char) -> i32;

#[derive(Debug)]
struct Setup {
    root: PathBuf,
    config: LoaderConfig,
}

#[derive(Debug)]
pub(crate) struct Runtime {
    pub(crate) mono: Mono,
    mod_class: Class,
    attr_class: Class,
}

unsafe impl Send for Runtime {}
unsafe impl Sync for Runtime {}

static SETUP: OnceLock<Setup> = OnceLock::new();
pub(crate) static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static ORIGINAL_JIT_INIT: AtomicUsize = AtomicUsize::new(0);
static MONO_MODULE: AtomicUsize = AtomicUsize::new(0);
static ORIGINAL_IL2CPP_INIT: AtomicUsize = AtomicUsize::new(0);
static IL2CPP_MODULE: AtomicUsize = AtomicUsize::new(0);

#[cfg(windows)]
mod entry {
    use super::{ROOT_DIR, attach, get_proc_address_hook};
    use crate::log::{error, info};
    use crate::{iat, proxy, sys};
    use std::ffi::c_void;
    use std::panic::catch_unwind;
    use windows_sys::Win32::Foundation::{FALSE, HMODULE, TRUE};
    use windows_sys::Win32::System::LibraryLoader::{DisableThreadLibraryCalls, GetModuleHandleW};
    use windows_sys::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
    use windows_sys::core::BOOL;

    #[unsafe(no_mangle)]
    pub unsafe extern "system" fn DllMain(
        module: HMODULE,
        reason: u32,
        _reserved: *mut c_void,
    ) -> BOOL {
        if reason != DLL_PROCESS_ATTACH {
            return TRUE;
        }
        unsafe { DisableThreadLibraryCalls(module) };
        if proxy::init().is_err() {
            return FALSE;
        }
        let _ = catch_unwind(|| start(module));
        TRUE
    }

    fn start(module: HMODULE) {
        let Some(game_dir) = sys::module_path(module).parent().map(std::path::Path::to_path_buf)
        else {
            return;
        };
        if !attach(&game_dir, game_dir.join(ROOT_DIR)) {
            return;
        }
        let player = unsafe { GetModuleHandleW(sys::wide("UnityPlayer.dll").as_ptr()) };
        let target = if player.is_null() {
            unsafe { GetModuleHandleW(std::ptr::null()) }
        } else {
            player
        };
        match unsafe {
            iat::hook(
                target,
                "kernel32.dll",
                "GetProcAddress",
                get_proc_address_hook as *const () as usize,
            )
        } {
            Some(_) => info!("GetProcAddress hooked"),
            None => error!("failed to hook GetProcAddress"),
        }
    }
}

pub(crate) fn attach(game_dir: &Path, root: PathBuf) -> bool {
    if !root.is_dir() || !is_unity_player() {
        return false;
    }
    let (config, config_error) = LoaderConfig::load(&root.join("ferrite.toml"));
    let enabled = config.loader.enabled && !sys::command_line().contains("--no-mods");
    log::init(
        &root.join("log.txt"),
        enabled && config.loader.console,
        &config.log,
    );
    info!(
        "Ferrite {VERSION} attached, game dir: {}",
        game_dir.display()
    );
    if let Some(e) = config_error {
        warning!("invalid config, using defaults: {e}");
    }
    if !enabled {
        info!("loader disabled, skipping");
        return false;
    }
    *config::MOD_CONFIGS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) =
        Some(ModConfigs::new(root.join("config")));
    let _ = SETUP.set(Setup { root, config });
    true
}

#[cfg(unix)]
pub(crate) fn active() -> bool {
    SETUP.get().is_some()
}

#[cfg(not(target_os = "macos"))]
fn data_dir() -> PathBuf {
    sys::exe_path().with_file_name(format!("{}_Data", game_name()))
}

#[cfg(target_os = "macos")]
fn data_dir() -> PathBuf {
    let exe = sys::exe_path();
    exe.parent()
        .and_then(Path::parent)
        .map(|contents| contents.join("Resources").join("Data"))
        .unwrap_or_default()
}

fn is_unity_player() -> bool {
    !game_name().is_empty() && data_dir().is_dir()
}

fn managed_dir() -> PathBuf {
    data_dir().join("Managed")
}

pub(crate) fn game_name() -> String {
    sys::exe_path()
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(windows)]
unsafe extern "system" fn get_proc_address_hook(module: Module, name: *const u8) -> *mut c_void {
    let real = unsafe { windows_sys::Win32::System::LibraryLoader::GetProcAddress(module, name) }
        .map_or(std::ptr::null_mut(), |f| f as *mut c_void);
    if real.is_null() || (name as usize) <= 0xFFFF {
        return real;
    }
    intercept(module, unsafe { sys::c_bytes(name) }, real)
}

pub(crate) fn intercept(module: Module, name: &[u8], real: *mut c_void) -> *mut c_void {
    if name == b"mono_jit_init_version" {
        ORIGINAL_JIT_INIT.store(real as usize, Ordering::SeqCst);
        MONO_MODULE.store(module as usize, Ordering::SeqCst);
        return jit_init_hook as *mut c_void;
    }
    if let Some(hook) = patched::intercept(name, real) {
        return hook;
    }
    if name == b"il2cpp_init" {
        ORIGINAL_IL2CPP_INIT.store(real as usize, Ordering::SeqCst);
        IL2CPP_MODULE.store(module as usize, Ordering::SeqCst);
        return il2cpp_init_hook as *mut c_void;
    }
    real
}

fn open_console() {
    if SETUP.get().is_some_and(|s| s.config.loader.console) {
        log::open_console(&format!("{} - Ferrite {VERSION}", game_name()));
    }
}

fn start_menu(runtime: &'static str) {
    if let Some(setup) = SETUP.get().filter(|s| s.config.loader.menu) {
        overlay::init(setup.root.clone(), runtime, &setup.config.loader.menu_key);
    }
}

unsafe extern "C" fn il2cpp_init_hook(domain_name: *const c_char) -> i32 {
    open_console();
    let _ = catch_unwind(|| start_menu("IL2CPP"));
    let original: Il2CppInit = unsafe {
        std::mem::transmute::<usize, Il2CppInit>(ORIGINAL_IL2CPP_INIT.load(Ordering::SeqCst))
    };
    let result = unsafe { original(domain_name) };
    let booted = catch_unwind(|| {
        let Some(setup) = SETUP.get() else { return };
        info!("IL2CPP runtime initialized");
        il2cpp::boot(
            IL2CPP_MODULE.load(Ordering::SeqCst) as Module,
            setup.root.clone(),
            setup.config.clone(),
        );
    });
    if booted.is_err() {
        error!("IL2CPP boot panicked");
    }
    result
}

fn enable_debugger() {
    let Some(setup) = SETUP.get().filter(|s| s.config.debug.enabled) else {
        return;
    };
    let module = MONO_MODULE.load(Ordering::SeqCst) as Module;
    let (Some(parse), Some(debug_init)) = (
        sys::symbol(module, c"mono_jit_parse_options"),
        sys::symbol(module, c"mono_debug_init"),
    ) else {
        error!("this Mono build has no debugger support");
        return;
    };
    let debug = &setup.config.debug;
    let option = format!(
        "--debugger-agent=transport=dt_socket,server=y,address={},suspend={}\0",
        debug.address,
        if debug.suspend { "y" } else { "n" }
    );
    let mut argv = [option.as_ptr().cast::<c_char>()];
    unsafe {
        let parse: unsafe extern "C" fn(i32, *mut *const c_char) = std::mem::transmute(parse);
        let debug_init: unsafe extern "C" fn(i32) = std::mem::transmute(debug_init);
        parse(1, argv.as_mut_ptr());
        debug_init(1);
    }
    info!(
        "Mono debugger listening on {}{}",
        debug.address,
        if debug.suspend {
            ", waiting for a debugger"
        } else {
            ""
        }
    );
}

unsafe extern "C" fn jit_init_hook(root: *const c_char, runtime: *const c_char) -> Ptr {
    open_console();
    let _ = catch_unwind(enable_debugger);
    let _ = catch_unwind(|| start_menu("Mono"));
    let original: JitInit =
        unsafe { std::mem::transmute::<usize, JitInit>(ORIGINAL_JIT_INIT.load(Ordering::SeqCst)) };
    let domain = unsafe { original(root, runtime) };
    if !domain.is_null() && catch_unwind(boot).is_err() {
        error!("boot panicked");
    }
    domain
}

fn boot() {
    let Some(setup) = SETUP.get() else { return };
    let module = MONO_MODULE.load(Ordering::SeqCst) as Module;
    let mono = match Mono::load(module) {
        Ok(m) => m,
        Err(missing) => {
            error!("mono export missing: {}", missing.trim_end_matches('\0'));
            return;
        }
    };
    mono.set_main_thread();
    let api = setup.root.join("core").join("Ferrite.dll");
    let Some(image) = mono.open_image(&api.to_string_lossy()) else {
        error!("cannot open {}", api.display());
        return;
    };
    let (Some(mod_class), Some(attr_class)) = (
        mono.class(image, API_NAMESPACE, "Mod"),
        mono.class(image, API_NAMESPACE, "ModAttribute"),
    ) else {
        error!(
            "Ferrite.Mod or Ferrite.ModAttribute not found in {}",
            api.display()
        );
        return;
    };
    register_icalls(&mono);
    let runtime = RUNTIME.get_or_init(|| Runtime {
        mono,
        mod_class,
        attr_class,
    });
    match runtime
        .mono
        .invoke_static(image, API_NAMESPACE, "Bootstrap", "Start")
    {
        Ok(()) => info!("managed runtime started"),
        Err(e) => error!("managed runtime failed: {e}"),
    }
}

fn register_icalls(mono: &Mono) {
    let calls: [(&str, *const c_void); 17] = [
        ("MenuOpen", icall_menu_open as *const c_void),
        ("ReportMods", icall_report_mods as *const c_void),
        ("LoadPatched", icall_load_patched as *const c_void),
        ("ConfigDescribe", icall_config_describe as *const c_void),
        ("ConfigList", icall_config_list as *const c_void),
        ("ConfigPoll", icall_config_poll as *const c_void),
        ("Builtin", icall_builtin as *const c_void),
        ("BuiltinHelp", icall_builtin_help as *const c_void),
        ("LogSequence", icall_log_sequence as *const c_void),
        ("LogTail", icall_log_tail as *const c_void),
        ("Log", icall_log as *const c_void),
        ("NextCommand", icall_next_command as *const c_void),
        ("DiscoverMods", icall_discover_mods as *const c_void),
        ("Option", icall_option as *const c_void),
        ("ConfigGet", icall_config_get as *const c_void),
        ("ConfigSet", icall_config_set as *const c_void),
        ("RootDir", icall_root_dir as *const c_void),
    ];
    for (name, f) in calls {
        mono.add_icall(&format!("{API_NAMESPACE}.Native::{name}"), f);
    }
}

fn guarded<T>(fallback: T, f: impl FnOnce(&Runtime) -> T) -> T {
    let Some(runtime) = RUNTIME.get() else {
        return fallback;
    };
    catch_unwind(AssertUnwindSafe(|| f(runtime))).unwrap_or(fallback)
}

fn null() -> Ptr {
    std::ptr::null_mut()
}

unsafe extern "C" fn icall_log(level: i32, source: Ptr, message: Ptr) {
    guarded((), |rt| {
        let source = unsafe { rt.mono.read_string(source) }.unwrap_or_default();
        let message = unsafe { rt.mono.read_string(message) }.unwrap_or_default();
        log::write(Level::from_raw(level), &source, &message);
    });
}

unsafe extern "C" fn icall_next_command() -> Ptr {
    guarded(null(), |rt| {
        log::next_command().map_or_else(null, |line| rt.mono.new_string(&line))
    })
}

unsafe extern "C" fn icall_discover_mods() -> Ptr {
    guarded(null(), |rt| {
        let Some(setup) = SETUP.get() else {
            return null();
        };
        let entries = mods::discover(
            &rt.mono,
            &setup.root.join("mods"),
            rt.mod_class,
            rt.attr_class,
            &setup.config.mods.disabled,
        );
        for e in &entries {
            info!(
                "found {} {} ({}) in {}",
                e.name,
                e.version,
                e.id,
                e.file.display()
            );
        }
        let classes: Vec<Class> = entries.iter().map(|e| e.class).collect();
        rt.mono.type_array(&classes)
    })
}

unsafe extern "C" fn icall_option(key: Ptr) -> Ptr {
    guarded(null(), |rt| {
        let key = unsafe { rt.mono.read_string(key) }.unwrap_or_default();
        if key == "managed_dir" {
            return rt.mono.new_string(&managed_dir().to_string_lossy());
        }
        SETUP
            .get()
            .and_then(|s| s.config.option(&key))
            .map_or_else(null, |v| rt.mono.new_string(&v))
    })
}

unsafe extern "C" fn icall_config_get(id: Ptr, key: Ptr) -> Ptr {
    guarded(null(), |rt| {
        let (id, key) = unsafe {
            (
                rt.mono.read_string(id).unwrap_or_default(),
                rt.mono.read_string(key).unwrap_or_default(),
            )
        };
        config::with_mod_configs(|c| c.get(&id, &key))
            .flatten()
            .map_or_else(null, |v| rt.mono.new_string(&v))
    })
}

unsafe extern "C" fn icall_config_set(id: Ptr, key: Ptr, value: Ptr, kind: i32) {
    guarded((), |rt| {
        let read = |p| unsafe { rt.mono.read_string(p) }.unwrap_or_default();
        let (id, key, value) = (read(id), read(key), read(value));
        if let Some(Err(e)) =
            config::with_mod_configs(|c| c.set(&id, &key, &value, Kind::from_raw(kind)))
        {
            error!("cannot save config for {id}: {e}");
        }
    });
}

unsafe extern "C" fn icall_root_dir() -> Ptr {
    guarded(null(), |rt| {
        SETUP
            .get()
            .map_or_else(null, |s| rt.mono.new_string(&s.root.to_string_lossy()))
    })
}

fn joined(lines: impl IntoIterator<Item = String>) -> String {
    lines.into_iter().collect::<Vec<_>>().join("\n")
}

unsafe extern "C" fn icall_config_describe(id: Ptr, key: Ptr, description: Ptr) {
    guarded((), |rt| {
        let read = |p| unsafe { rt.mono.read_string(p) }.unwrap_or_default();
        let (id, key, description) = (read(id), read(key), read(description));
        if let Some(Err(e)) = config::with_mod_configs(|c| c.describe(&id, &key, &description)) {
            error!("cannot save config for {id}: {e}");
        }
    });
}

unsafe extern "C" fn icall_config_list(id: Ptr) -> Ptr {
    guarded(null(), |rt| {
        let id = unsafe { rt.mono.read_string(id) }.unwrap_or_default();
        let entries = config::with_mod_configs(|c| c.list(&id)).unwrap_or_default();
        rt.mono.new_string(&joined(
            entries.into_iter().map(|(k, v)| format!("{k}\t{v}")),
        ))
    })
}

unsafe extern "C" fn icall_config_poll() -> Ptr {
    guarded(null(), |rt| {
        let changed = config::with_mod_configs(config::ModConfigs::poll).unwrap_or_default();
        if changed.is_empty() {
            null()
        } else {
            rt.mono.new_string(&joined(changed))
        }
    })
}

unsafe extern "C" fn icall_builtin(line: Ptr) -> bool {
    guarded(false, |rt| {
        let line = unsafe { rt.mono.read_string(line) }.unwrap_or_default();
        console::builtin(&line)
    })
}

unsafe extern "C" fn icall_builtin_help() -> Ptr {
    guarded(null(), |rt| rt.mono.new_string(&joined(console::help())))
}

unsafe extern "C" fn icall_log_sequence() -> i64 {
    log::sequence()
}

unsafe extern "C" fn icall_log_tail() -> Ptr {
    guarded(null(), |rt| rt.mono.new_string(&log::recent()))
}

unsafe extern "C" fn icall_load_patched(path: Ptr, data: Ptr) -> bool {
    guarded(false, |rt| {
        let path = unsafe { rt.mono.read_string(path) }.unwrap_or_default();
        let bytes = unsafe { rt.mono.bytes(data) };
        match rt.mono.load_from_memory(&path, bytes) {
            Ok((assembly, image)) if rt.mono.assembly_image(assembly) != image => {
                error!(
                    "{path} was loaded before patching, patches are not applied; patchers must not reference game assemblies"
                );
                false
            }
            Ok((assembly, image)) => {
                patched::register(&path, assembly, image);
                true
            }
            Err(e) => {
                error!("cannot load patched {path}: {e}");
                false
            }
        }
    })
}

unsafe extern "C" fn icall_menu_open() -> bool {
    overlay::is_open()
}

unsafe extern "C" fn icall_report_mods(rows: Ptr) {
    guarded((), |rt| {
        let text = unsafe { rt.mono.read_string(rows) }.unwrap_or_default();
        let rows = text
            .lines()
            .filter_map(|line| {
                let mut f = line.split('\t');
                Some(overlay::ModRow {
                    id: f.next()?.to_owned(),
                    name: f.next()?.to_owned(),
                    version: f.next()?.to_owned(),
                    author: f.next()?.to_owned(),
                    faulted: f.next() == Some("1"),
                    kind: "C#",
                })
            })
            .collect();
        overlay::set_mods("C#", rows);
    });
}
