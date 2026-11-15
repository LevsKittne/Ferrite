mod config;
mod iat;
mod log;
mod mods;
mod mono;
mod proxy;
mod sys;

use crate::config::{Kind, LoaderConfig, ModConfigs};
use crate::log::{Level, error, info, warning};
use crate::mono::{Class, Mono, Ptr};
use std::ffi::{c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use windows_sys::Win32::Foundation::{FALSE, HMODULE, TRUE};
use windows_sys::Win32::System::Environment::GetCommandLineW;
use windows_sys::Win32::System::LibraryLoader::{
    DisableThreadLibraryCalls, GetModuleHandleW, GetProcAddress,
};
use windows_sys::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows_sys::core::BOOL;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const ROOT_DIR: &str = "Ferrite";
const API_NAMESPACE: &str = "Ferrite";

type JitInit = unsafe extern "C" fn(*const c_char, *const c_char) -> Ptr;

#[derive(Debug)]
struct Setup {
    root: PathBuf,
    config: LoaderConfig,
}

#[derive(Debug)]
struct Runtime {
    mono: Mono,
    mod_class: Class,
    attr_class: Class,
}

unsafe impl Send for Runtime {}
unsafe impl Sync for Runtime {}

static SETUP: OnceLock<Setup> = OnceLock::new();
static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static ORIGINAL_JIT_INIT: AtomicUsize = AtomicUsize::new(0);
static MONO_MODULE: AtomicUsize = AtomicUsize::new(0);

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
    let _ = catch_unwind(|| attach(module));
    TRUE
}

fn attach(module: HMODULE) {
    let Some(game_dir) = sys::module_path(module).parent().map(Path::to_path_buf) else {
        return;
    };
    let root = game_dir.join(ROOT_DIR);
    if !root.is_dir() || !is_unity_player() {
        return;
    }
    let (config, config_error) = LoaderConfig::load(&root.join("ferrite.toml"));
    let command_line = unsafe { sys::from_wide_ptr(GetCommandLineW()) };
    let enabled = config.loader.enabled && !command_line.contains("--no-mods");
    log::init(&root.join("log.txt"), enabled && config.loader.console);
    info!(
        "Ferrite {VERSION} attached, game dir: {}",
        game_dir.display()
    );
    if let Some(e) = config_error {
        warning!("invalid config, using defaults: {e}");
    }
    if !enabled {
        info!("loader disabled, skipping");
        return;
    }
    *config::MOD_CONFIGS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) =
        Some(ModConfigs::new(root.join("config")));
    let _ = SETUP.set(Setup { root, config });
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

fn is_unity_player() -> bool {
    let exe = sys::module_path(std::ptr::null_mut());
    exe.file_stem().is_some_and(|stem| {
        exe.with_file_name(format!("{}_Data", stem.to_string_lossy()))
            .is_dir()
    })
}

fn game_name() -> String {
    sys::module_path(std::ptr::null_mut())
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

unsafe extern "system" fn get_proc_address_hook(module: HMODULE, name: *const u8) -> *mut c_void {
    let real =
        unsafe { GetProcAddress(module, name) }.map_or(std::ptr::null_mut(), |f| f as *mut c_void);
    if real.is_null() || (name as usize) <= 0xFFFF {
        return real;
    }
    if unsafe { sys::c_bytes(name) } == b"mono_jit_init_version" {
        ORIGINAL_JIT_INIT.store(real as usize, Ordering::SeqCst);
        MONO_MODULE.store(module as usize, Ordering::SeqCst);
        return jit_init_hook as *mut c_void;
    }
    real
}

unsafe extern "C" fn jit_init_hook(root: *const c_char, runtime: *const c_char) -> Ptr {
    if SETUP.get().is_some_and(|s| s.config.loader.console) {
        log::open_console(&format!("{} - Ferrite {VERSION}", game_name()));
    }
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
    let module = MONO_MODULE.load(Ordering::SeqCst) as HMODULE;
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
        .invoke_static(image, API_NAMESPACE, "Runtime", "Boot")
    {
        Ok(()) => info!("managed runtime started"),
        Err(e) => error!("managed runtime failed: {e}"),
    }
}

fn register_icalls(mono: &Mono) {
    let calls: [(&str, *const c_void); 7] = [
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
