use crate::log::{error, info, warning};
use crate::native;
use ferrite_sdk::il2cpp::{Il2Cpp, Object, Ptr};
use std::panic::catch_unwind;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use windows_sys::Win32::Foundation::HMODULE;

type UpdateFn = unsafe extern "C" fn(Ptr);
type SceneLoadedFn = unsafe extern "C" fn(i32, i32, Ptr);
type SceneNameFn = unsafe extern "C" fn(i32, Ptr) -> Ptr;

const UPDATE_CANDIDATES: [(&str, &str, &str); 2] = [
    ("UnityEngine", "UnitySynchronizationContext", "ExecuteTasks"),
    ("UnityEngine", "BeforeRenderHelper", "Invoke"),
];

#[derive(Debug)]
struct State {
    api: Il2Cpp,
    mods_dir: PathBuf,
    disabled: Vec<String>,
    scene_name: Option<(usize, usize)>,
}

static STATE: OnceLock<State> = OnceLock::new();
static ORIGINAL_UPDATE: AtomicUsize = AtomicUsize::new(0);
static ORIGINAL_SCENE: AtomicUsize = AtomicUsize::new(0);
static LOADED: AtomicBool = AtomicBool::new(false);

pub fn boot(module: HMODULE, mods_dir: PathBuf, disabled: Vec<String>) {
    native::set_symbol_module(module);
    let Some(api) = Il2Cpp::from_resolver(native::symbol) else {
        error!("il2cpp exports missing in GameAssembly.dll");
        return;
    };
    let scene_name = api
        .find_method("UnityEngine.SceneManagement", "Scene", "GetNameInternal", 1)
        .map(|m| (m.pointer() as usize, m.0 as usize));
    let state = STATE.get_or_init(|| State {
        api,
        mods_dir,
        disabled,
        scene_name,
    });
    let mut hooked = false;
    if let Some(m) = state.api.find_method(
        "UnityEngine.SceneManagement",
        "SceneManager",
        "Internal_SceneLoaded",
        2,
    ) {
        let previous = unsafe { m.replace_pointer(scene_detour as *const () as Ptr) };
        ORIGINAL_SCENE.store(previous as usize, Ordering::Release);
        info!("hooked SceneManager.Internal_SceneLoaded");
        hooked = true;
    }
    for (namespace, class, method) in UPDATE_CANDIDATES {
        let Some(m) = state.api.find_method(namespace, class, method, 0) else {
            continue;
        };
        let previous = unsafe { m.replace_pointer(update_detour as *const () as Ptr) };
        ORIGINAL_UPDATE.store(previous as usize, Ordering::Release);
        info!("hooked {namespace}.{class}.{method} for updates");
        hooked = true;
        break;
    }
    if ORIGINAL_UPDATE.load(Ordering::Acquire) == 0 {
        warning!(
            "no per-frame hook found, mod update callbacks and console commands are unavailable"
        );
    }
    if !hooked {
        load_once();
    }
}

fn load_once() {
    if LOADED.swap(true, Ordering::AcqRel) {
        return;
    }
    let Some(state) = STATE.get() else { return };
    native::load_all(&state.mods_dir, &state.disabled);
}

fn scene_name(handle: i32) -> String {
    let Some(state) = STATE.get() else {
        return String::new();
    };
    let Some((pointer, method)) = state.scene_name else {
        return String::new();
    };
    let get: SceneNameFn = unsafe { std::mem::transmute::<usize, SceneNameFn>(pointer) };
    let name = unsafe { get(handle, method as Ptr) };
    unsafe { state.api.read_string(Object(name)) }.unwrap_or_default()
}

unsafe extern "C" fn update_detour(method: Ptr) {
    let _ = catch_unwind(|| {
        load_once();
        native::tick();
    });
    let original: UpdateFn =
        unsafe { std::mem::transmute::<usize, UpdateFn>(ORIGINAL_UPDATE.load(Ordering::Acquire)) };
    unsafe { original(method) };
}

unsafe extern "C" fn scene_detour(scene: i32, mode: i32, method: Ptr) {
    let original: SceneLoadedFn = unsafe {
        std::mem::transmute::<usize, SceneLoadedFn>(ORIGINAL_SCENE.load(Ordering::Acquire))
    };
    unsafe { original(scene, mode, method) };
    let _ = catch_unwind(|| {
        load_once();
        let name = scene_name(scene);
        info!("scene loaded: {name}");
        native::scene_loaded(&name);
    });
}
