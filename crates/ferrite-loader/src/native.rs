use crate::config::{self, Kind};
use crate::deps::{self, Catalog, Dependency, Node};
use crate::log::{self, Level, error, info, warning};
use crate::sys;
use ferrite_sdk::abi::{self, Api, CommandFn, Handle, InfoFn, LoadFn, ModInfo, SceneFn, UpdateFn};
use std::collections::HashMap;
use std::ffi::{CStr, c_char, c_void};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use windows_sys::Win32::Foundation::{FreeLibrary, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

#[derive(Debug, Clone)]
struct Entry {
    id: String,
    name: String,
    version: String,
    dependencies: Vec<Dependency>,
    file: PathBuf,
    module: usize,
}

impl Node for Entry {
    fn id(&self) -> &str {
        &self.id
    }

    fn dependencies(&self) -> &[Dependency] {
        &self.dependencies
    }

    fn origin(&self) -> &Path {
        &self.file
    }
}

#[derive(Debug, Clone)]
struct Meta {
    id: String,
    name: String,
    version: String,
}

#[derive(Debug, Clone, Copy)]
struct Callbacks {
    update: Option<UpdateFn>,
    scene: Option<SceneFn>,
}

#[derive(Debug, Clone, Copy)]
struct Command {
    callback: CommandFn,
}

static META: Mutex<Vec<Meta>> = Mutex::new(Vec::new());
static CALLBACKS: Mutex<Vec<Callbacks>> = Mutex::new(Vec::new());
static COMMANDS: Mutex<Option<HashMap<String, (Command, String)>>> = Mutex::new(None);
static SYMBOLS: AtomicUsize = AtomicUsize::new(0);

static API: Api = Api {
    abi: abi::ABI_VERSION,
    log: api_log,
    symbol: api_symbol,
    hook: api_hook,
    command: api_command,
    config_get: api_config_get,
    config_set: api_config_set,
};

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn set_symbol_module(module: HMODULE) {
    SYMBOLS.store(module as usize, Ordering::Release);
}

pub fn symbol(name: &str) -> Option<*mut c_void> {
    let module = SYMBOLS.load(Ordering::Acquire) as HMODULE;
    let name = std::ffi::CString::new(name).ok()?;
    unsafe { GetProcAddress(module, name.as_ptr().cast()) }.map(|f| f as *mut c_void)
}

pub unsafe fn hook(target: *mut c_void, detour: *mut c_void) -> Option<*mut c_void> {
    match unsafe { crate::detour::install(target, detour) } {
        Ok(trampoline) => Some(trampoline.cast_mut()),
        Err(e) => {
            error!("hook at {target:p} failed: {e}");
            None
        }
    }
}

unsafe fn read_cstr(p: *const c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
    }
}

unsafe fn read_str(ptr: *const u8, len: usize) -> String {
    if ptr.is_null() || len == 0 {
        return String::new();
    }
    String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(ptr, len) }).into_owned()
}

fn inspect(file: &Path) -> Option<Entry> {
    let module = unsafe { LoadLibraryW(sys::wide(file).as_ptr()) };
    if module.is_null() {
        warning!("cannot load {}", file.display());
        return None;
    }
    let Some(info_fn) = (unsafe { GetProcAddress(module, abi::INFO_SYMBOL.as_ptr()) }) else {
        warning!("skipping {}: no ferrite_mod_info export", file.display());
        unsafe { FreeLibrary(module) };
        return None;
    };
    let info_fn: InfoFn =
        unsafe { std::mem::transmute::<unsafe extern "system" fn() -> isize, InfoFn>(info_fn) };
    let info: &ModInfo = unsafe { info_fn().as_ref() }?;
    if info.abi != abi::ABI_VERSION {
        error!(
            "{} targets ABI {}, Ferrite uses {}, skipped",
            file.display(),
            info.abi,
            abi::ABI_VERSION
        );
        unsafe { FreeLibrary(module) };
        return None;
    }
    let id = unsafe { read_cstr(info.id) };
    if id.is_empty() {
        error!("{} has an empty mod id, skipped", file.display());
        unsafe { FreeLibrary(module) };
        return None;
    }
    Some(Entry {
        name: unsafe { read_cstr(info.name) },
        version: unsafe { read_cstr(info.version) },
        dependencies: Dependency::parse_list(&unsafe { read_cstr(info.dependencies) }),
        id,
        file: file.to_path_buf(),
        module: module as usize,
    })
}

pub fn load_all(mods_dir: &Path, disabled: &[String]) {
    let mut catalog = Catalog::new(disabled);
    let mut modules = Vec::new();
    for file in deps::dlls(mods_dir) {
        if let Some(entry) = inspect(&file) {
            modules.push(entry.module);
            catalog.add(entry);
        }
    }
    let ordered = catalog.ordered();
    for module in modules
        .into_iter()
        .filter(|m| !ordered.iter().any(|e| e.module == *m))
    {
        unsafe { FreeLibrary(module as HMODULE) };
    }
    for entry in ordered {
        load(&entry);
    }
    info!("{} native mod(s) loaded", lock(&CALLBACKS).len());
}

fn export<T>(module: HMODULE, name: &[u8]) -> Option<T> {
    let f = unsafe { GetProcAddress(module, name.as_ptr()) }?;
    Some(unsafe { std::mem::transmute_copy::<unsafe extern "system" fn() -> isize, T>(&f) })
}

fn load(entry: &Entry) {
    let module = entry.module as HMODULE;
    let Some(load_fn) = export::<LoadFn>(module, abi::LOAD_SYMBOL) else {
        error!("{} has no ferrite_mod_load export, skipped", entry.id);
        return;
    };
    let handle = {
        let mut meta = lock(&META);
        meta.push(Meta {
            id: entry.id.clone(),
            name: entry.name.clone(),
            version: entry.version.clone(),
        });
        meta.len()
    };
    info!(
        "found {} {} ({}) in {}",
        entry.name,
        entry.version,
        entry.id,
        entry.file.display()
    );
    if unsafe { load_fn(&raw const API, handle as Handle) } {
        let callbacks = Callbacks {
            update: export(module, abi::UPDATE_SYMBOL),
            scene: export(module, abi::SCENE_SYMBOL),
        };
        let mut all = lock(&CALLBACKS);
        all.resize(
            handle,
            Callbacks {
                update: None,
                scene: None,
            },
        );
        all[handle - 1] = callbacks;
        info!("loaded {} {} ({})", entry.name, entry.version, entry.id);
    } else {
        error!("{} failed to load", entry.id);
    }
}

pub fn tick() {
    poll_commands();
    let updates: Vec<UpdateFn> = lock(&CALLBACKS).iter().filter_map(|c| c.update).collect();
    for update in updates {
        unsafe { update() };
    }
}

pub fn scene_loaded(name: &str) {
    let scenes: Vec<SceneFn> = lock(&CALLBACKS).iter().filter_map(|c| c.scene).collect();
    for scene in scenes {
        unsafe { scene(name.as_ptr(), name.len()) };
    }
}

fn meta(handle: Handle) -> Option<Meta> {
    let index = (handle as usize).checked_sub(1)?;
    lock(&META).get(index).cloned()
}

fn print(text: &str) {
    log::write(Level::Info, "Console", text);
}

fn poll_commands() {
    while let Some(line) = log::next_command() {
        print(&format!("> {line}"));
        let (name, args) = line.split_once(' ').unwrap_or((&line, ""));
        match name.to_lowercase().as_str() {
            "help" => help(),
            "mods" => mods(),
            key => {
                let command = lock(&COMMANDS)
                    .as_ref()
                    .and_then(|c| c.get(key).map(|(c, _)| *c));
                match command {
                    Some(c) => unsafe {
                        (c.callback)(name.as_ptr(), name.len(), args.as_ptr(), args.len());
                    },
                    None => log::write(
                        Level::Warning,
                        "Console",
                        &format!("unknown command {name}, type help"),
                    ),
                }
            }
        }
    }
}

fn help() {
    print("help - Lists commands.");
    print("mods - Lists loaded mods.");
    let mut lines: Vec<String> = lock(&COMMANDS)
        .iter()
        .flatten()
        .map(|(name, (_, description))| format!("{name} - {description}"))
        .collect();
    lines.sort();
    for line in lines {
        print(&line);
    }
}

fn mods() {
    let loaded = lock(&CALLBACKS).len();
    print(&format!(
        "Ferrite {}, {loaded} native mod(s):",
        env!("CARGO_PKG_VERSION")
    ));
    for m in lock(&META).iter().take(loaded) {
        print(&format!("  {} {} ({})", m.name, m.version, m.id));
    }
}

unsafe extern "C" fn api_log(handle: Handle, level: i32, message: *const u8, len: usize) {
    let source = meta(handle).map_or_else(|| "Mod".to_owned(), |m| m.name);
    log::write(Level::from_raw(level), &source, &unsafe {
        read_str(message, len)
    });
}

unsafe extern "C" fn api_symbol(name: *const c_char) -> *mut c_void {
    symbol(&unsafe { read_cstr(name) }).unwrap_or(std::ptr::null_mut())
}

unsafe extern "C" fn api_hook(target: *mut c_void, detour: *mut c_void) -> *mut c_void {
    unsafe { hook(target, detour) }.unwrap_or(std::ptr::null_mut())
}

unsafe extern "C" fn api_command(
    _handle: Handle,
    name: *const u8,
    name_len: usize,
    description: *const u8,
    description_len: usize,
    callback: CommandFn,
) {
    let name = unsafe { read_str(name, name_len) }.to_lowercase();
    if matches!(name.as_str(), "help" | "mods") || name.is_empty() {
        error!("command name {name} is reserved");
        return;
    }
    let description = unsafe { read_str(description, description_len) };
    lock(&COMMANDS)
        .get_or_insert_with(HashMap::new)
        .insert(name, (Command { callback }, description));
}

unsafe extern "C" fn api_config_get(
    handle: Handle,
    key: *const u8,
    key_len: usize,
    buffer: *mut u8,
    capacity: usize,
) -> isize {
    let Some(m) = meta(handle) else { return -1 };
    let key = unsafe { read_str(key, key_len) };
    let Some(value) = config::with_mod_configs(|c| c.get(&m.id, &key)).flatten() else {
        return -1;
    };
    let bytes = value.as_bytes();
    if bytes.len() <= capacity && !buffer.is_null() {
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, bytes.len()) };
    }
    isize::try_from(bytes.len()).unwrap_or(-1)
}

unsafe extern "C" fn api_config_set(
    handle: Handle,
    key: *const u8,
    key_len: usize,
    value: *const u8,
    value_len: usize,
    kind: i32,
) {
    let Some(m) = meta(handle) else { return };
    let (key, value) = unsafe { (read_str(key, key_len), read_str(value, value_len)) };
    if let Some(Err(e)) =
        config::with_mod_configs(|c| c.set(&m.id, &key, &value, Kind::from_raw(kind)))
    {
        error!("cannot save config for {}: {e}", m.id);
    }
}
