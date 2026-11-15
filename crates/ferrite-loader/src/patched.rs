use crate::mono::Ptr;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};

type DomainOpenFn = unsafe extern "C" fn(Ptr, *const c_char) -> Ptr;
type AssemblyOpenFn = unsafe extern "C" fn(*const c_char, *mut c_int) -> Ptr;
type ImageFromDataFn =
    unsafe extern "C" fn(*mut c_char, u32, c_int, *mut c_int, c_int, *const c_char) -> Ptr;

#[derive(Debug, Clone)]
struct Patched {
    file: String,
    assembly: usize,
    image: usize,
}

static ENTRIES: Mutex<Vec<Patched>> = Mutex::new(Vec::new());
static DOMAIN_OPEN: AtomicUsize = AtomicUsize::new(0);
static ASSEMBLY_OPEN: AtomicUsize = AtomicUsize::new(0);
static IMAGE_FROM_DATA: AtomicUsize = AtomicUsize::new(0);

fn key(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|f| f.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

pub fn register(path: &str, assembly: Ptr, image: Ptr) {
    let mut entries = ENTRIES.lock().unwrap_or_else(PoisonError::into_inner);
    let file = key(path);
    entries.retain(|e| e.file != file);
    entries.push(Patched {
        file,
        assembly: assembly as usize,
        image: image as usize,
    });
}

fn lookup(name: *const c_char) -> Option<Patched> {
    if name.is_null() {
        return None;
    }
    let file = key(&unsafe { CStr::from_ptr(name) }.to_string_lossy());
    ENTRIES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .find(|e| e.file == file)
        .cloned()
}

pub fn intercept(name: &[u8], real: *mut c_void) -> Option<*mut c_void> {
    let (slot, hook) = match name {
        b"mono_domain_assembly_open" => (&DOMAIN_OPEN, domain_open_hook as *mut c_void),
        b"mono_assembly_open" => (&ASSEMBLY_OPEN, assembly_open_hook as *mut c_void),
        b"mono_image_open_from_data_with_name" => {
            (&IMAGE_FROM_DATA, image_from_data_hook as *mut c_void)
        }
        _ => return None,
    };
    slot.store(real as usize, Ordering::SeqCst);
    Some(hook)
}

unsafe extern "C" fn domain_open_hook(domain: Ptr, name: *const c_char) -> Ptr {
    if let Some(p) = lookup(name) {
        return p.assembly as Ptr;
    }
    let original: DomainOpenFn =
        unsafe { std::mem::transmute::<usize, DomainOpenFn>(DOMAIN_OPEN.load(Ordering::SeqCst)) };
    unsafe { original(domain, name) }
}

unsafe extern "C" fn assembly_open_hook(name: *const c_char, status: *mut c_int) -> Ptr {
    if let Some(p) = lookup(name) {
        if !status.is_null() {
            unsafe { status.write(0) };
        }
        return p.assembly as Ptr;
    }
    let original: AssemblyOpenFn = unsafe {
        std::mem::transmute::<usize, AssemblyOpenFn>(ASSEMBLY_OPEN.load(Ordering::SeqCst))
    };
    unsafe { original(name, status) }
}

unsafe extern "C" fn image_from_data_hook(
    data: *mut c_char,
    len: u32,
    need_copy: c_int,
    status: *mut c_int,
    refonly: c_int,
    name: *const c_char,
) -> Ptr {
    if refonly == 0
        && let Some(p) = lookup(name)
        && let Some(rt) = crate::RUNTIME.get()
    {
        rt.mono.image_addref(p.image as Ptr);
        if !status.is_null() {
            unsafe { status.write(0) };
        }
        return p.image as Ptr;
    }
    let original: ImageFromDataFn = unsafe {
        std::mem::transmute::<usize, ImageFromDataFn>(IMAGE_FROM_DATA.load(Ordering::SeqCst))
    };
    unsafe { original(data, len, need_copy, status, refonly, name) }
}
