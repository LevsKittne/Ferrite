pub mod abi;
pub mod il2cpp;

use std::collections::HashMap;
use std::ffi::{CString, c_void};
use std::sync::{Mutex, PoisonError};

pub use il2cpp::Il2Cpp;

type Handler = Box<dyn Fn(&Context, &[&str]) + Send>;

static COMMANDS: Mutex<Option<HashMap<String, Handler>>> = Mutex::new(None);
static CONTEXT: Mutex<Option<Context>> = Mutex::new(None);

#[derive(Debug, Clone, Copy)]
pub struct Context {
    api: &'static abi::Api,
    handle: abi::Handle,
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

impl Context {
    #[doc(hidden)]
    #[must_use]
    pub unsafe fn from_raw(api: *const abi::Api, handle: abi::Handle) -> Option<Self> {
        let api = unsafe { api.as_ref() }?;
        (api.abi == abi::ABI_VERSION).then_some(Self { api, handle })
    }

    pub fn log(&self, level: i32, message: &str) {
        unsafe { (self.api.log)(self.handle, level, message.as_ptr(), message.len()) };
    }

    pub fn debug(&self, message: &str) {
        self.log(abi::LEVEL_DEBUG, message);
    }

    pub fn info(&self, message: &str) {
        self.log(abi::LEVEL_INFO, message);
    }

    pub fn warning(&self, message: &str) {
        self.log(abi::LEVEL_WARNING, message);
    }

    pub fn error(&self, message: &str) {
        self.log(abi::LEVEL_ERROR, message);
    }

    #[must_use]
    pub fn symbol(&self, name: &str) -> Option<*mut c_void> {
        let name = CString::new(name).ok()?;
        let p = unsafe { (self.api.symbol)(name.as_ptr()) };
        (!p.is_null()).then_some(p)
    }

    #[must_use]
    pub unsafe fn hook(&self, target: *mut c_void, detour: *mut c_void) -> Option<*mut c_void> {
        let trampoline = unsafe { (self.api.hook)(target, detour) };
        (!trampoline.is_null()).then_some(trampoline)
    }

    #[must_use]
    pub fn il2cpp(&self) -> Option<Il2Cpp> {
        Il2Cpp::load(self)
    }

    pub fn command(
        &self,
        name: &str,
        description: &str,
        handler: impl Fn(&Context, &[&str]) + Send + 'static,
    ) {
        COMMANDS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get_or_insert_with(HashMap::new)
            .insert(name.to_lowercase(), Box::new(handler));
        unsafe {
            (self.api.command)(
                self.handle,
                name.as_ptr(),
                name.len(),
                description.as_ptr(),
                description.len(),
                dispatch_command,
            );
        }
    }

    #[must_use]
    pub fn config_string(&self, key: &str) -> Option<String> {
        let mut buffer = vec![0u8; 256];
        loop {
            let len = unsafe {
                (self.api.config_get)(
                    self.handle,
                    key.as_ptr(),
                    key.len(),
                    buffer.as_mut_ptr(),
                    buffer.len(),
                )
            };
            let len = usize::try_from(len).ok()?;
            if len <= buffer.len() {
                buffer.truncate(len);
                return String::from_utf8(buffer).ok();
            }
            buffer.resize(len, 0);
        }
    }

    #[must_use]
    pub fn config_get<T: ConfigValue>(&self, key: &str, fallback: T) -> T {
        if let Some(value) = self.config_string(key).and_then(|s| T::parse(&s)) {
            return value;
        }
        self.config_set(key, &fallback);
        fallback
    }

    pub fn config_set<T: ConfigValue>(&self, key: &str, value: &T) {
        let text = value.render();
        unsafe {
            (self.api.config_set)(
                self.handle,
                key.as_ptr(),
                key.len(),
                text.as_ptr(),
                text.len(),
                T::KIND,
            );
        };
    }
}

pub trait ConfigValue: Sized {
    const KIND: i32;
    fn parse(raw: &str) -> Option<Self>;
    fn render(&self) -> String;
}

impl ConfigValue for String {
    const KIND: i32 = abi::KIND_STRING;
    fn parse(raw: &str) -> Option<Self> {
        Some(raw.to_owned())
    }
    fn render(&self) -> String {
        self.clone()
    }
}

impl ConfigValue for bool {
    const KIND: i32 = abi::KIND_BOOL;
    fn parse(raw: &str) -> Option<Self> {
        raw.parse().ok()
    }
    fn render(&self) -> String {
        self.to_string()
    }
}

impl ConfigValue for i64 {
    const KIND: i32 = abi::KIND_INT;
    fn parse(raw: &str) -> Option<Self> {
        raw.parse().ok()
    }
    fn render(&self) -> String {
        self.to_string()
    }
}

impl ConfigValue for f64 {
    const KIND: i32 = abi::KIND_FLOAT;
    fn parse(raw: &str) -> Option<Self> {
        raw.parse().ok()
    }
    fn render(&self) -> String {
        format!("{self:?}")
    }
}

unsafe extern "C" fn dispatch_command(
    name: *const u8,
    name_len: usize,
    args: *const u8,
    args_len: usize,
) {
    let _ = std::panic::catch_unwind(|| {
        let name = unsafe { str_from_raw(name, name_len) }.to_lowercase();
        let args = unsafe { str_from_raw(args, args_len) };
        let parts: Vec<&str> = args.split_whitespace().collect();
        let Some(ctx) = *CONTEXT.lock().unwrap_or_else(PoisonError::into_inner) else {
            return;
        };
        let commands = COMMANDS.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(handler) = commands.as_ref().and_then(|c| c.get(&name)) {
            handler(&ctx, &parts);
        }
    });
}

unsafe fn str_from_raw<'a>(ptr: *const u8, len: usize) -> &'a str {
    if ptr.is_null() || len == 0 {
        return "";
    }
    std::str::from_utf8(unsafe { std::slice::from_raw_parts(ptr, len) }).unwrap_or("")
}

pub trait Mod: Send + 'static {
    fn load(ctx: &Context) -> Self
    where
        Self: Sized;

    fn update(&mut self, _ctx: &Context) {}

    fn scene_loaded(&mut self, _ctx: &Context, _scene: &str) {}
}

#[doc(hidden)]
pub mod runtime {
    use super::{CONTEXT, Context, Mod, abi, str_from_raw};
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::{Mutex, PoisonError};

    pub unsafe fn load<M: Mod>(
        state: &Mutex<Option<M>>,
        api: *const abi::Api,
        handle: abi::Handle,
    ) -> bool {
        let Some(ctx) = (unsafe { Context::from_raw(api, handle) }) else {
            return false;
        };
        *CONTEXT.lock().unwrap_or_else(PoisonError::into_inner) = Some(ctx);
        if let Ok(m) = catch_unwind(|| M::load(&ctx)) {
            *state.lock().unwrap_or_else(PoisonError::into_inner) = Some(m);
            true
        } else {
            ctx.error("load panicked");
            false
        }
    }

    pub fn update<M: Mod>(state: &Mutex<Option<M>>) {
        with(state, super::Mod::update);
    }

    pub unsafe fn scene<M: Mod>(state: &Mutex<Option<M>>, name: *const u8, len: usize) {
        let name = unsafe { str_from_raw(name, len) }.to_owned();
        with(state, |m, ctx| m.scene_loaded(ctx, &name));
    }

    fn with<M: Mod>(state: &Mutex<Option<M>>, f: impl FnOnce(&mut M, &Context)) {
        let Some(ctx) = *CONTEXT.lock().unwrap_or_else(PoisonError::into_inner) else {
            return;
        };
        let mut guard = state.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(m) = guard.as_mut() else {
            return;
        };
        if catch_unwind(AssertUnwindSafe(|| f(m, &ctx))).is_err() {
            ctx.error("callback panicked, mod disabled");
            *guard = None;
        }
    }
}

#[macro_export]
macro_rules! export_mod {
    ($ty:ty, id = $id:literal, name = $name:literal, version = $version:literal $(, dependencies = $deps:literal)? $(,)?) => {
        static FERRITE_STATE: ::std::sync::Mutex<Option<$ty>> = ::std::sync::Mutex::new(None);
        static FERRITE_INFO: $crate::abi::ModInfo = $crate::abi::ModInfo {
            abi: $crate::abi::ABI_VERSION,
            id: concat!($id, "\0").as_ptr().cast(),
            name: concat!($name, "\0").as_ptr().cast(),
            version: concat!($version, "\0").as_ptr().cast(),
            dependencies: concat!($($deps,)? "\0").as_ptr().cast(),
        };

        #[unsafe(no_mangle)]
        pub extern "C" fn ferrite_mod_info() -> *const $crate::abi::ModInfo {
            &raw const FERRITE_INFO
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn ferrite_mod_load(api: *const $crate::abi::Api, handle: $crate::abi::Handle) -> bool {
            unsafe { $crate::runtime::load::<$ty>(&FERRITE_STATE, api, handle) }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn ferrite_mod_update() {
            $crate::runtime::update::<$ty>(&FERRITE_STATE);
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn ferrite_mod_scene(name: *const u8, len: usize) {
            unsafe { $crate::runtime::scene::<$ty>(&FERRITE_STATE, name, len) };
        }
    };
}
