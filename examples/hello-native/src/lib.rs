use ferrite_sdk::il2cpp::{Il2Cpp, Object, Ptr};
use ferrite_sdk::{Context, Mod, export_mod};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

static FPS: AtomicU64 = AtomicU64::new(0);
static CANVAS_CALLS: AtomicU64 = AtomicU64::new(0);
static CANVAS_ORIGINAL: AtomicUsize = AtomicUsize::new(0);

struct HelloNative {
    frames: u32,
    since: Instant,
}

fn unity_log(il2cpp: &Il2Cpp, message: &str) -> bool {
    let Some(log) = il2cpp.find_method("UnityEngine", "Debug", "Log", 1) else {
        return false;
    };
    let mut args = [il2cpp.string(message).0];
    unsafe { il2cpp.invoke(log, Object(std::ptr::null_mut()), &mut args) }.is_ok()
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
        let greeting: String = ctx.config_get("greeting", "Hello from Rust!".to_owned());
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
        Self {
            frames: 0,
            since: Instant::now(),
        }
    }

    fn update(&mut self, _ctx: &Context) {
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
}

export_mod!(
    HelloNative,
    id = "hello-native",
    name = "Hello Native",
    version = "1.0.0"
);
