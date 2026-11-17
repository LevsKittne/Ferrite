use ferrite_sdk::il2cpp::{Class, GcHandle, Il2Cpp, Object, Ptr};
use ferrite_sdk::inject::ClassBuilder;
use ferrite_sdk::{Context, Mod, export_mod};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::Instant;

static FPS: AtomicU64 = AtomicU64::new(0);
static CANVAS_CALLS: AtomicU64 = AtomicU64::new(0);
static CANVAS_ORIGINAL: AtomicUsize = AtomicUsize::new(0);

static BEHAVIOUR: OnceLock<Class> = OnceLock::new();
static SPAWNED: AtomicBool = AtomicBool::new(false);
static GUI_CALLS: AtomicU64 = AtomicU64::new(0);
static HOST: Mutex<Option<GcHandle>> = Mutex::new(None);

struct HelloNative {
    frames: u32,
    since: Instant,
}

fn frames_field(il2cpp: &Il2Cpp) -> Option<ferrite_sdk::il2cpp::Field> {
    il2cpp.field(*BEHAVIOUR.get()?, "frames")
}

fn say(message: &str) {
    if let Some(ctx) = ferrite_sdk::context() {
        ctx.info(message);
    }
}

unsafe extern "C" fn behaviour_awake(this: Object, _: Ptr) {
    say(&format!("FerriteBehaviour.Awake on {:p}", this.0));
}

unsafe extern "C" fn behaviour_start(_: Object, _: Ptr) {
    say("FerriteBehaviour.Start");
}

unsafe extern "C" fn behaviour_update(this: Object, _: Ptr) {
    let Some(il2cpp) = ferrite_sdk::context().and_then(|c| c.il2cpp()) else {
        return;
    };
    let Some(field) = frames_field(&il2cpp) else {
        return;
    };
    let frames = unsafe { il2cpp.get::<i32>(this, field) } + 1;
    unsafe { il2cpp.set(this, field, frames) };
    if frames == 1 || frames % 600 == 0 {
        say(&format!(
            "FerriteBehaviour.Update frame {frames}, OnGUI calls {}",
            GUI_CALLS.load(Ordering::Relaxed)
        ));
    }
}

unsafe extern "C" fn behaviour_gui(_: Object, _: Ptr) {
    GUI_CALLS.fetch_add(1, Ordering::Relaxed);
}

unsafe extern "C" fn behaviour_destroy(_: Object, _: Ptr) {
    say("FerriteBehaviour.OnDestroy");
}

fn inject_behaviour(ctx: &Context) {
    let result = ClassBuilder::new(ctx, "Ferrite.Examples", "FerriteBehaviour")
        .parent("UnityEngine", "MonoBehaviour")
        .field_of::<i32>("frames")
        .method("Awake", behaviour_awake)
        .method("Start", behaviour_start)
        .method("Update", behaviour_update)
        .method("OnGUI", behaviour_gui)
        .method("OnDestroy", behaviour_destroy)
        .build();
    match result {
        Ok(injected) => {
            let _ = BEHAVIOUR.set(injected.class);
        }
        Err(e) => ctx.error(&format!("cannot inject FerriteBehaviour: {e}")),
    }
}

fn spawn_behaviour(ctx: &Context) {
    let (Some(il2cpp), Some(&class)) = (ctx.il2cpp(), BEHAVIOUR.get()) else {
        return;
    };
    let Some(game_object) = il2cpp.class("UnityEngine", "GameObject") else {
        return;
    };
    let (Some(ctor), Some(add)) = (
        il2cpp.method(game_object, ".ctor", 1),
        il2cpp.method(game_object, "AddComponent", 1),
    ) else {
        ctx.warning("GameObject constructor or AddComponent(Type) is stripped");
        return;
    };
    let host = il2cpp.new_object(game_object);
    let mut name = [il2cpp.string("Ferrite Behaviour Host").0];
    if unsafe { il2cpp.invoke(ctor, host, &mut name) }.is_err() {
        ctx.error("new GameObject() threw");
        return;
    }
    if let Some(keep) = il2cpp.find_method("UnityEngine", "Object", "DontDestroyOnLoad", 1) {
        let _ = unsafe { il2cpp.invoke(keep, Object(std::ptr::null_mut()), &mut [host.0]) };
    }
    let mut ty = [il2cpp.type_object(class).0];
    match unsafe { il2cpp.invoke(add, host, &mut ty) } {
        Ok(component) if !component.is_null() => {
            *HOST.lock().unwrap_or_else(PoisonError::into_inner) = Some(il2cpp.pin(host));
            ctx.info(&format!(
                "added {} to a new GameObject",
                il2cpp.full_class_name(il2cpp.class_of(component).unwrap_or(class))
            ));
        }
        _ => ctx.error("AddComponent(FerriteBehaviour) failed"),
    }
}

fn destroy_host(ctx: &Context) {
    let Some(handle) = HOST.lock().unwrap_or_else(PoisonError::into_inner).take() else {
        return;
    };
    let host = handle.target();
    if let Some(il2cpp) = ctx.il2cpp()
        && let Some(destroy) = il2cpp.find_method("UnityEngine", "Object", "DestroyImmediate", 1)
    {
        let _ = unsafe { il2cpp.invoke(destroy, Object(std::ptr::null_mut()), &mut [host.0]) };
    }
}

fn unity_log(il2cpp: &Il2Cpp, message: &str) -> bool {
    let Some(log) = il2cpp.find_method("UnityEngine", "Debug", "Log", 1) else {
        return false;
    };
    let mut args = [il2cpp.string(message).0];
    unsafe { il2cpp.invoke(log, Object(std::ptr::null_mut()), &mut args) }.is_ok()
}

fn report_behaviour(ctx: &Context) {
    let Some(il2cpp) = ctx.il2cpp() else { return };
    let Some(class) = il2cpp.class("Ferrite.Examples", "FerriteBehaviour") else {
        ctx.warning("FerriteBehaviour is not resolvable by name");
        return;
    };
    let ty = il2cpp.type_object(class);
    let type_name = il2cpp
        .class_of(ty)
        .and_then(|c| il2cpp.method(c, "get_FullName", 0))
        .and_then(|m| unsafe { il2cpp.invoke(m, ty, &mut []) }.ok())
        .and_then(|s| unsafe { il2cpp.read_string(s) })
        .unwrap_or_default();
    let host = HOST
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .map(GcHandle::target);
    let component = host.and_then(|host| {
        let get = il2cpp.find_method("UnityEngine", "GameObject", "GetComponent", 1)?;
        unsafe { il2cpp.invoke(get, host, &mut [ty.0]) }
            .ok()
            .filter(|c| !c.is_null())
    });
    let frames = component
        .zip(frames_field(&il2cpp))
        .map(|(c, f)| unsafe { il2cpp.get::<i32>(c, f) });
    ctx.info(&format!(
        "by name: {}, System.Type: {type_name}, GetComponent: {}, frames: {frames:?}",
        il2cpp.full_class_name(class),
        component.map_or("none".to_owned(), |c| format!("{:p}", c.0))
    ));
}

unsafe extern "C" fn canvas_detour(method: Ptr) {
    CANVAS_CALLS.fetch_add(1, Ordering::Relaxed);
    let original = CANVAS_ORIGINAL.load(Ordering::Acquire);
    let original: unsafe extern "C" fn(Ptr) = unsafe { std::mem::transmute(original) };
    unsafe { original(method) };
}

fn hook_canvas(ctx: &Context) {
    if CANVAS_ORIGINAL.load(Ordering::Acquire) != 0 {
        ctx.info(&format!(
            "Canvas.SendWillRenderCanvases called {} times",
            CANVAS_CALLS.load(Ordering::Relaxed)
        ));
        return;
    }
    let Some(method) = ctx
        .il2cpp()
        .and_then(|a| a.find_method("UnityEngine", "Canvas", "SendWillRenderCanvases", 0))
    else {
        ctx.warning("Canvas.SendWillRenderCanvases not found");
        return;
    };
    match unsafe { ctx.hook(method.pointer(), canvas_detour as *const () as Ptr) } {
        Some(original) => {
            CANVAS_ORIGINAL.store(original as usize, Ordering::Release);
            ctx.info("hooked Canvas.SendWillRenderCanvases, run hooktest again to see the count");
        }
        None => ctx.error("hook failed"),
    }
}

impl Mod for HelloNative {
    fn load(ctx: &Context) -> Self {
        let greeting: String = ctx.config(
            "greeting",
            "Hello from Rust!".to_owned(),
            "Text printed on load and by the hello command.",
        );
        match ctx.il2cpp() {
            Some(il2cpp) if unity_log(&il2cpp, &format!("[Hello Native] {greeting}")) => {
                ctx.info(&format!("{greeting} Also written to the game's Unity log."));
            }
            Some(_) => ctx.info(&greeting),
            None => ctx.warning("IL2CPP API unavailable, this mod needs an IL2CPP game"),
        }
        ctx.command(
            "hello",
            "Prints the greeting, `hello <text>` sets a new one.",
            |ctx, args| {
                if !args.is_empty() {
                    ctx.config_set("greeting", &args.join(" "));
                }
                let greeting: String = ctx.config_get("greeting", "Hello from Rust!".to_owned());
                ctx.info(&greeting);
            },
        );
        ctx.command(
            "fps",
            "Prints frames per second measured by the mod.",
            |ctx, _| {
                ctx.info(&format!(
                    "{:.1} fps",
                    f64::from_bits(FPS.load(Ordering::Relaxed))
                ));
            },
        );
        ctx.command(
            "hooktest",
            "Hooks Canvas.SendWillRenderCanvases and counts calls.",
            |ctx, _| hook_canvas(ctx),
        );
        ctx.command(
            "behaviour",
            "Looks up the injected FerriteBehaviour and prints its frame counter.",
            |ctx, _| report_behaviour(ctx),
        );
        inject_behaviour(ctx);
        Self {
            frames: 0,
            since: Instant::now(),
        }
    }

    fn update(&mut self, ctx: &Context) {
        if !SPAWNED.swap(true, Ordering::AcqRel) {
            spawn_behaviour(ctx);
        }
        self.frames += 1;
        let elapsed = self.since.elapsed().as_secs_f64();
        if elapsed >= 1.0 {
            FPS.store(
                (f64::from(self.frames) / elapsed).to_bits(),
                Ordering::Relaxed,
            );
            self.frames = 0;
            self.since = Instant::now();
        }
    }

    fn scene_loaded(&mut self, ctx: &Context, scene: &str) {
        ctx.info(&format!("scene {scene} loaded"));
    }

    fn config_changed(&mut self, ctx: &Context) {
        let greeting: String = ctx.config_get("greeting", "Hello from Rust!".to_owned());
        ctx.info(&format!("config changed, greeting is now: {greeting}"));
    }

    fn unload(&mut self, ctx: &Context) {
        CANVAS_ORIGINAL.store(0, Ordering::Release);
        destroy_host(ctx);
        ctx.info("unloading, hooks are removed by Ferrite");
    }
}

export_mod!(
    HelloNative,
    id = "hello-native",
    name = "Hello Native",
    version = "1.0.0"
);
