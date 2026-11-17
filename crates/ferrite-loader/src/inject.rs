use crate::log::{error, info, warning};
use crate::native;
use ferrite_sdk::abi::{self, Handle};
use std::collections::HashMap;
use std::ffi::{CStr, c_char, c_void};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

type Ptr = *mut c_void;
type FromTypeFn = unsafe extern "C" fn(*const u8, bool) -> Ptr;
type FromNameFn = unsafe extern "C" fn(Ptr, *const c_char, *const c_char) -> Ptr;

const TYPE_VALUETYPE: u8 = 0x11;
const TYPE_CLASS: u8 = 0x12;
const SCAN_LIMIT: usize = 64;

#[derive(Debug, Default)]
struct Registry {
    by_token: HashMap<i64, usize>,
    by_name: HashMap<(String, String), usize>,
    next: i64,
}

static REGISTRY: Mutex<Option<Registry>> = Mutex::new(None);
static HOOKS: OnceLock<Result<(), String>> = OnceLock::new();
static FROM_TYPE: AtomicUsize = AtomicUsize::new(0);
static FROM_NAME: AtomicUsize = AtomicUsize::new(0);

fn registry() -> MutexGuard<'static, Option<Registry>> {
    REGISTRY.lock().unwrap_or_else(PoisonError::into_inner)
}

fn single_target(export: &str) -> Result<Ptr, String> {
    let start = native::symbol(export).ok_or_else(|| format!("{export} is not exported"))?;
    let targets = unsafe { crate::detour::jump_targets(start, SCAN_LIMIT) };
    match targets.as_slice() {
        [target] => Ok(*target as Ptr),
        other => Err(format!(
            "{export} has {} call targets, expected one",
            other.len()
        )),
    }
}

unsafe fn install(export: &str, detour: Ptr, slot: &AtomicUsize) -> Result<(), String> {
    let target = single_target(export)?;
    let trampoline = unsafe { crate::detour::install(target, detour) }
        .map_err(|e| format!("cannot hook the function behind {export}: {e}"))?;
    slot.store(trampoline as usize, Ordering::Release);
    info!("hooked the function behind {export} at {target:p}");
    Ok(())
}

fn install_hooks() -> Result<(), String> {
    let from_type = ["il2cpp_class_from_il2cpp_type", "il2cpp_class_from_type"]
        .into_iter()
        .find(|e| native::symbol(e).is_some())
        .ok_or("il2cpp_class_from_il2cpp_type is not exported")?;
    unsafe {
        install(
            from_type,
            from_type_detour as *const () as Ptr,
            &FROM_TYPE,
        )?;
        install(
            "il2cpp_class_from_name",
            from_name_detour as *const () as Ptr,
            &FROM_NAME,
        )?;
    }
    Ok(())
}

fn lookup_token(token: i64) -> Ptr {
    registry()
        .as_ref()
        .and_then(|r| r.by_token.get(&token).copied())
        .unwrap_or(0) as Ptr
}

unsafe extern "C" fn from_type_detour(ty: *const u8, throw: bool) -> Ptr {
    if !ty.is_null() {
        let data = unsafe { ty.cast::<i64>().read_unaligned() };
        let kind = unsafe { ty.add(10).read() };
        if data < 0 && (kind == TYPE_CLASS || kind == TYPE_VALUETYPE) {
            let class = lookup_token(data);
            if !class.is_null() {
                return class;
            }
        }
    }
    let original: FromTypeFn =
        unsafe { std::mem::transmute::<usize, FromTypeFn>(FROM_TYPE.load(Ordering::Acquire)) };
    unsafe { original(ty, throw) }
}

unsafe fn text(p: *const c_char) -> Option<String> {
    (!p.is_null()).then(|| unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
}

unsafe extern "C" fn from_name_detour(
    image: Ptr,
    namespace: *const c_char,
    name: *const c_char,
) -> Ptr {
    let original: FromNameFn =
        unsafe { std::mem::transmute::<usize, FromNameFn>(FROM_NAME.load(Ordering::Acquire)) };
    let found = unsafe { original(image, namespace, name) };
    if !found.is_null() {
        return found;
    }
    let (Some(namespace), Some(name)) = (unsafe { text(namespace) }, unsafe { text(name) }) else {
        return found;
    };
    registry()
        .as_ref()
        .and_then(|r| r.by_name.get(&(namespace, name)).copied())
        .map_or(found, |c| c as Ptr)
}

pub unsafe extern "C" fn api_register_class(
    _handle: Handle,
    class: *mut c_void,
    namespace: *const c_char,
    name: *const c_char,
) -> i64 {
    if class.is_null() {
        return 0;
    }
    let (namespace, Some(name)) = (unsafe { text(namespace) }.unwrap_or_default(), unsafe {
        text(name)
    }) else {
        return 0;
    };
    if let Err(e) = HOOKS.get_or_init(install_hooks) {
        error!("class injection is unavailable: {e}");
        return 0;
    }
    let mut guard = registry();
    let registry = guard.get_or_insert_with(|| Registry {
        next: -2,
        ..Registry::default()
    });
    let token = registry.next;
    registry.next -= 1;
    registry.by_token.insert(token, class as usize);
    let full = if namespace.is_empty() {
        name.clone()
    } else {
        format!("{namespace}.{name}")
    };
    if registry
        .by_name
        .insert((namespace, name), class as usize)
        .is_some()
    {
        warning!("injected class {full} registered again, lookups by name now return the new one");
    }
    token
}

unsafe extern "C" fn noop_method() -> usize {
    0
}

unsafe extern "C" fn noop_invoker(_: Ptr, _: Ptr, _: Ptr, _: *mut Ptr, _: Ptr) {}

pub unsafe extern "C" fn api_stub(kind: i32) -> *mut c_void {
    match kind {
        abi::STUB_METHOD => noop_method as *const () as Ptr,
        abi::STUB_INVOKER => noop_invoker as *const () as Ptr,
        _ => std::ptr::null_mut(),
    }
}
