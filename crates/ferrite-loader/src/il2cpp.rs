use crate::config::{Entrypoint, LoaderConfig};
use crate::log::{self, Level, error, info, warning};
use crate::{console, native};
use ferrite_sdk::il2cpp::{
    Class, FIELD_LITERAL, FIELD_STATIC, Il2Cpp, METHOD_STATIC, METHOD_VIRTUAL, Object, Ptr,
};
use std::fmt::Write as _;
use std::panic::catch_unwind;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use crate::sys::Module;

type UpdateFn = unsafe extern "C" fn(Ptr);
type SceneLoadedFn = unsafe extern "C" fn(i32, i32, Ptr);
type SceneNameFn = unsafe extern "C" fn(i32, Ptr) -> Ptr;
type LogCallbackFn = unsafe extern "C" fn(Ptr, Ptr, i32, bool, Ptr);
type SetLogDefinedFn = unsafe extern "C" fn(bool);

const UPDATE_CANDIDATES: [(&str, &str, &str); 3] = [
    ("UnityEngine", "UnitySynchronizationContext", "ExecuteTasks"),
    ("UnityEngine", "BeforeRenderHelper", "Invoke"),
    ("UnityEngine", "Canvas", "SendWillRenderCanvases"),
];

type FrameCountFn = unsafe extern "C" fn() -> i32;

#[derive(Debug)]
struct State {
    api: Il2Cpp,
    module: usize,
    root: PathBuf,
    config: LoaderConfig,
    scene_name: Option<(usize, usize)>,
}

static STATE: OnceLock<State> = OnceLock::new();
static ORIGINAL_UPDATE: [AtomicUsize; 3] = [const { AtomicUsize::new(0) }; 3];
static FRAME_COUNT: AtomicUsize = AtomicUsize::new(0);
static LAST_FRAME: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(-1);
static ORIGINAL_SCENE: AtomicUsize = AtomicUsize::new(0);
static ORIGINAL_LOG: AtomicUsize = AtomicUsize::new(0);
static LOADED: AtomicBool = AtomicBool::new(false);
static MENU_WAS_OPEN: AtomicBool = AtomicBool::new(false);
static SAVED_CURSOR: AtomicUsize = AtomicUsize::new(0);

type GetLockFn = unsafe extern "C" fn() -> i32;
type SetLockFn = unsafe extern "C" fn(i32);
type GetVisibleFn = unsafe extern "C" fn() -> bool;
type SetVisibleFn = unsafe extern "C" fn(bool);

fn update_cursor() {
    let open = crate::overlay::is_open();
    let was = MENU_WAS_OPEN.swap(open, Ordering::Relaxed);
    if !open && !was {
        return;
    }
    let Some(state) = STATE.get() else { return };
    let api = &state.api;
    let (Some(get_lock), Some(set_lock), Some(get_visible), Some(set_visible)) = (
        api.icall("UnityEngine.Cursor::get_lockState()"),
        api.icall("UnityEngine.Cursor::set_lockState(UnityEngine.CursorLockMode)"),
        api.icall("UnityEngine.Cursor::get_visible()"),
        api.icall("UnityEngine.Cursor::set_visible(System.Boolean)"),
    ) else {
        return;
    };
    unsafe {
        let get_lock = std::mem::transmute::<Ptr, GetLockFn>(get_lock);
        let set_lock = std::mem::transmute::<Ptr, SetLockFn>(set_lock);
        let get_visible = std::mem::transmute::<Ptr, GetVisibleFn>(get_visible);
        let set_visible = std::mem::transmute::<Ptr, SetVisibleFn>(set_visible);
        if open && !was {
            let saved =
                usize::try_from(get_lock()).unwrap_or(0) | (usize::from(get_visible()) << 8);
            SAVED_CURSOR.store(saved, Ordering::Relaxed);
        }
        if open {
            set_lock(0);
            set_visible(true);
        } else {
            let saved = SAVED_CURSOR.load(Ordering::Relaxed);
            set_lock(i32::try_from(saved & 0xFF).unwrap_or(0));
            set_visible(saved & 0x100 != 0);
        }
    }
}

pub fn boot(module: Module, root: PathBuf, config: LoaderConfig) {
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
        module: module as usize,
        root,
        config,
        scene_name,
    });
    console::register(
        "reload",
        "<mod id> - Reloads a native mod from its dll.",
        |args| match args.first() {
            Some(id) => {
                native::reload(id);
            }
            None => console::print("usage: reload <mod id>"),
        },
    );
    console::register(
        "dump",
        "[filter] - Writes all IL2CPP classes, fields with offsets and methods with RVAs to Ferrite\\dump.",
        |args| dump(args.first().copied()),
    );
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
    let detours: [Ptr; 3] = [
        update_detour::<0> as *const () as Ptr,
        update_detour::<1> as *const () as Ptr,
        update_detour::<2> as *const () as Ptr,
    ];
    let mut per_frame = 0;
    for (i, (namespace, class, method)) in UPDATE_CANDIDATES.iter().enumerate() {
        let Some(m) = state.api.find_method(namespace, class, method, 0) else {
            continue;
        };
        let previous = unsafe { m.replace_pointer(detours[i]) };
        ORIGINAL_UPDATE[i].store(previous as usize, Ordering::Release);
        per_frame += 1;
        hooked = true;
    }
    if let Some(f) = state.api.icall("UnityEngine.Time::get_frameCount()") {
        FRAME_COUNT.store(f as usize, Ordering::Release);
    }
    if per_frame == 0 {
        warning!(
            "no per-frame hook found, mod update callbacks and console commands are unavailable"
        );
    } else {
        info!("{per_frame} per-frame hook(s) installed");
    }
    if state.config.loader.unity_log {
        capture_unity_log(&state.api);
    }
    if !hooked || state.config.loader.entrypoint == Entrypoint::Early {
        load_once();
    }
}

fn capture_unity_log(api: &Il2Cpp) {
    let Some(m) = api.find_method("UnityEngine", "Application", "CallLogCallback", 4) else {
        info!("Application.CallLogCallback is stripped, Unity log is not captured");
        return;
    };
    let previous = unsafe { m.replace_pointer(log_detour as *const () as Ptr) };
    ORIGINAL_LOG.store(previous as usize, Ordering::Release);
    if let Some(set) = api.icall("UnityEngine.Application::SetLogCallbackDefined(System.Boolean)") {
        let set: SetLogDefinedFn = unsafe { std::mem::transmute::<Ptr, SetLogDefinedFn>(set) };
        unsafe { set(true) };
    }
    info!("capturing Unity log");
}

fn load_once() {
    if LOADED.swap(true, Ordering::AcqRel) {
        return;
    }
    let Some(state) = STATE.get() else { return };
    native::load_all(
        &state.root,
        &state.root.join("mods"),
        &state.config.mods.disabled,
        state.config.mods.hot_reload,
    );
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

fn new_frame() -> bool {
    let f = FRAME_COUNT.load(Ordering::Acquire);
    if f == 0 {
        return true;
    }
    let get: FrameCountFn = unsafe { std::mem::transmute::<usize, FrameCountFn>(f) };
    let frame = i64::from(unsafe { get() });
    LAST_FRAME.swap(frame, Ordering::AcqRel) != frame
}

unsafe extern "C" fn update_detour<const N: usize>(method: Ptr) {
    let _ = catch_unwind(|| {
        if new_frame() {
            load_once();
            update_cursor();
            native::tick();
        }
    });
    let original = ORIGINAL_UPDATE[N].load(Ordering::Acquire);
    if original != 0 {
        let original: UpdateFn = unsafe { std::mem::transmute::<usize, UpdateFn>(original) };
        unsafe { original(method) };
    }
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

unsafe extern "C" fn log_detour(message: Ptr, trace: Ptr, kind: i32, main: bool, method: Ptr) {
    let _ = catch_unwind(|| {
        let Some(state) = STATE.get() else { return };
        let text = unsafe { state.api.read_string(Object(message)) }.unwrap_or_default();
        let level = match kind {
            2 => Level::Warning,
            3 => Level::Info,
            _ => Level::Error,
        };
        let text = if level == Level::Error {
            let trace = unsafe { state.api.read_string(Object(trace)) }.unwrap_or_default();
            if trace.trim().is_empty() {
                text
            } else {
                format!("{text}\n{}", trace.trim_end())
            }
        } else {
            text
        };
        log::write(level, "Unity", &text);
    });
    let original = ORIGINAL_LOG.load(Ordering::Acquire);
    if original != 0 {
        let original: LogCallbackFn =
            unsafe { std::mem::transmute::<usize, LogCallbackFn>(original) };
        unsafe { original(message, trace, kind, main, method) };
    }
}

fn dump(filter: Option<&str>) {
    let Some(state) = STATE.get() else { return };
    let api = &state.api;
    let filter = filter.map(str::to_lowercase);
    let dir = state.root.join("dump");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("il2cpp.cs");
    let base = state.module;
    let mut out = String::new();
    let mut classes = 0usize;
    let mut methods = 0usize;
    for image in api.images() {
        let image_name = api.image_name(image);
        let mut section = String::new();
        for class in api.classes(image) {
            let full = api.full_class_name(class);
            if filter
                .as_deref()
                .is_some_and(|f| !full.to_lowercase().contains(f))
            {
                continue;
            }
            classes += 1;
            methods += write_class(api, class, &full, base, &mut section);
        }
        if !section.is_empty() {
            let _ = writeln!(out, "// {image_name}\n{section}");
        }
    }
    match std::fs::write(&path, out) {
        Ok(()) => console::print(&format!(
            "dumped {classes} classes, {methods} methods to {}",
            path.display()
        )),
        Err(e) => console::print(&format!("cannot write {}: {e}", path.display())),
    }
}

fn write_class(api: &Il2Cpp, class: Class, full: &str, base: usize, out: &mut String) -> usize {
    let kind = if api.is_value_type(class) {
        "struct"
    } else if api.class_flags(class) & 0x20 != 0 {
        "interface"
    } else {
        "class"
    };
    let parent = api
        .parent(class)
        .map(|p| api.full_class_name(p))
        .filter(|p| p != "System.Object" && p != "System.ValueType");
    let _ = write!(out, "{kind} {full}");
    if let Some(p) = parent {
        let _ = write!(out, " : {p}");
    }
    out.push_str("\n{\n");
    for field in api.fields(class) {
        let flags = api.field_flags(field);
        let prefix = if flags & FIELD_LITERAL != 0 {
            "const "
        } else if flags & FIELD_STATIC != 0 {
            "static "
        } else {
            ""
        };
        let offset = if flags & (FIELD_STATIC | FIELD_LITERAL) == 0 {
            format!(" // {:#x}", api.offset(field))
        } else {
            String::new()
        };
        let _ = writeln!(
            out,
            "    {prefix}{} {};{offset}",
            api.field_type(field),
            api.field_name(field)
        );
    }
    let mut count = 0;
    for method in api.methods(class) {
        count += 1;
        let flags = api.method_flags(method);
        let prefix = match (flags & METHOD_STATIC != 0, flags & METHOD_VIRTUAL != 0) {
            (true, _) => "static ",
            (false, true) => "virtual ",
            _ => "",
        };
        let params: Vec<String> = api
            .method_params(method)
            .into_iter()
            .map(|(ty, name)| format!("{ty} {name}"))
            .collect();
        let pointer = method.pointer() as usize;
        let rva = if pointer > base {
            format!(" // RVA {:#x}", pointer - base)
        } else {
            String::new()
        };
        let _ = writeln!(
            out,
            "    {prefix}{} {}({});{rva}",
            api.method_return_type(method),
            api.method_name(method),
            params.join(", ")
        );
    }
    out.push_str("}\n\n");
    count
}
