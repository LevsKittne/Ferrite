use crate::config::{self, Kind};
use crate::console;
use crate::deps::{self, Catalog, Dependency, Node};
use crate::log::{self, Level, error, info, warning};
use crate::sys::{self, Module};
use ferrite_sdk::abi::{
    self, Api, CommandFn, ConfigFn, Handle, InfoFn, LoadFn, ModInfo, SceneFn, UnloadFn, UpdateFn,
};
use std::collections::HashMap;
use std::ffi::{CStr, c_char, c_void};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::{Duration, Instant, SystemTime};

#[derive(Debug, Clone)]
struct Entry {
    id: String,
    name: String,
    version: String,
    dependencies: Vec<Dependency>,
    file: PathBuf,
    module: usize,
    abi: u32,
}

impl Node for Entry {
    fn id(&self) -> &str {
        &self.id
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn dependencies(&self) -> &[Dependency] {
        &self.dependencies
    }

    fn origin(&self) -> &Path {
        &self.file
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Callbacks {
    update: Option<UpdateFn>,
    scene: Option<SceneFn>,
    unload: Option<UnloadFn>,
    config: Option<ConfigFn>,
}

#[derive(Debug, Clone)]
struct Loaded {
    id: String,
    name: String,
    version: String,
    file: PathBuf,
    module: usize,
    stamp: Option<SystemTime>,
    callbacks: Callbacks,
    active: bool,
}

#[derive(Debug, Clone)]
struct Command {
    callback: CommandFn,
    description: String,
    owner: usize,
}

#[derive(Debug, Clone, Copy)]
struct PointerPatch {
    owner: usize,
    slot: usize,
    previous: usize,
    value: usize,
}

#[derive(Debug)]
struct Dirs {
    root: PathBuf,
    mods: PathBuf,
    cache: PathBuf,
    hot_reload: bool,
}

static MODS: Mutex<Vec<Loaded>> = Mutex::new(Vec::new());
static COMMANDS: Mutex<Option<HashMap<String, Command>>> = Mutex::new(None);
static HOOKS: Mutex<Vec<(usize, usize)>> = Mutex::new(Vec::new());
static POINTERS: Mutex<Vec<PointerPatch>> = Mutex::new(Vec::new());
static SYMBOLS: AtomicUsize = AtomicUsize::new(0);
static DIRS: OnceLock<Dirs> = OnceLock::new();
static SHADOW: AtomicUsize = AtomicUsize::new(0);
static LAST_CHECK: Mutex<Option<Instant>> = Mutex::new(None);

macro_rules! api_table {
    ($abi:expr) => {
        Api {
            abi: $abi,
            log: api_log,
            symbol: api_symbol,
            hook: api_hook,
            command: api_command,
            config_get: api_config_get,
            config_set: api_config_set,
            config_describe: api_config_describe,
            hook_owned: api_hook_owned,
            unhook: api_unhook,
            patch_pointer: api_patch_pointer,
            execute: api_execute,
            directory: api_directory,
        }
    };
}

static API: Api = api_table!(abi::ABI_VERSION);
static API_V1: Api = api_table!(1);

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn set_symbol_module(module: Module) {
    SYMBOLS.store(module as usize, Ordering::Release);
}

pub fn symbol(name: &str) -> Option<*mut c_void> {
    let module = SYMBOLS.load(Ordering::Acquire) as Module;
    let name = std::ffi::CString::new(name).ok()?;
    sys::symbol(module, &name)
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

fn stamp(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn open_library(file: &Path) -> Option<Module> {
    let hot = DIRS.get().is_some_and(|d| d.hot_reload);
    if let Some(dirs) = DIRS.get().filter(|_| hot) {
        let n = SHADOW.fetch_add(1, Ordering::Relaxed);
        let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned());
        let copy = dirs.cache.join(format!(
            "{}.{n}.{}",
            stem.unwrap_or_default(),
            sys::LIBRARY_EXTENSION
        ));
        std::fs::create_dir_all(&dirs.cache).ok()?;
        if let Err(e) = std::fs::copy(file, &copy) {
            warning!("cannot shadow-copy {}: {e}", file.display());
            return None;
        }
        sys::open_library(&copy, file.parent())
    } else {
        sys::open_library(file, None)
    }
}

fn inspect(file: &Path) -> Option<Entry> {
    let Some(module) = open_library(file) else {
        warning!("cannot load {}", file.display());
        return None;
    };
    let Some(info_fn) = export::<InfoFn>(module, abi::INFO_SYMBOL) else {
        warning!("skipping {}: no ferrite_mod_info export", file.display());
        sys::free_library(module);
        return None;
    };
    let info: &ModInfo = unsafe { info_fn().as_ref() }?;
    if !(abi::MIN_ABI_VERSION..=abi::ABI_VERSION).contains(&info.abi) {
        error!(
            "{} targets ABI {}, Ferrite supports {}..={}, skipped",
            file.display(),
            info.abi,
            abi::MIN_ABI_VERSION,
            abi::ABI_VERSION
        );
        sys::free_library(module);
        return None;
    }
    let id = unsafe { read_cstr(info.id) };
    if id.is_empty() {
        error!("{} has an empty mod id, skipped", file.display());
        sys::free_library(module);
        return None;
    }
    Some(Entry {
        name: unsafe { read_cstr(info.name) },
        version: unsafe { read_cstr(info.version) },
        dependencies: Dependency::parse_list(&unsafe { read_cstr(info.dependencies) }),
        id,
        file: file.to_path_buf(),
        module: module as usize,
        abi: info.abi,
    })
}

pub fn load_all(root: &Path, mods_dir: &Path, disabled: &[String], hot_reload: bool) {
    let cache = root.join("cache");
    if let Ok(entries) = std::fs::read_dir(&cache) {
        for e in entries.flatten() {
            let _ = std::fs::remove_file(e.path());
        }
    }
    let _ = DIRS.set(Dirs {
        root: root.to_path_buf(),
        mods: mods_dir.to_path_buf(),
        cache,
        hot_reload,
    });
    let started = Instant::now();
    let mut catalog = Catalog::new(disabled, &crate::game_name());
    let mut modules = Vec::new();
    for file in deps::files(mods_dir, sys::LIBRARY_EXTENSION) {
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
        sys::free_library(module as Module);
    }
    for entry in ordered {
        load(&entry, None);
    }
    report();
    let count = lock(&MODS).iter().filter(|m| m.active).count();
    info!(
        "{count} native mod(s) loaded in {} ms{}",
        started.elapsed().as_millis(),
        if hot_reload { ", hot reload on" } else { "" }
    );
}

fn export<T>(module: Module, name: &[u8]) -> Option<T> {
    let f = sys::symbol(module, CStr::from_bytes_with_nul(name).ok()?)?;
    Some(unsafe { std::mem::transmute_copy::<*mut c_void, T>(&f) })
}

fn load(entry: &Entry, slot: Option<usize>) -> bool {
    let module = entry.module as Module;
    let Some(load_fn) = export::<LoadFn>(module, abi::LOAD_SYMBOL) else {
        error!("{} has no ferrite_mod_load export, skipped", entry.id);
        sys::free_library(module);
        return false;
    };
    let record = Loaded {
        id: entry.id.clone(),
        name: entry.name.clone(),
        version: entry.version.clone(),
        file: entry.file.clone(),
        module: entry.module,
        stamp: stamp(&entry.file),
        callbacks: Callbacks::default(),
        active: false,
    };
    let handle = {
        let mut mods = lock(&MODS);
        if let Some(index) = slot.filter(|&i| i < mods.len()) {
            mods[index] = record;
            index + 1
        } else {
            mods.push(record);
            mods.len()
        }
    };
    info!(
        "found {} {} ({}) in {}",
        entry.name,
        entry.version,
        entry.id,
        entry.file.display()
    );
    let api = if entry.abi >= abi::ABI_VERSION {
        &raw const API
    } else {
        &raw const API_V1
    };
    if unsafe { load_fn(api, handle as Handle) } {
        let callbacks = Callbacks {
            update: export(module, abi::UPDATE_SYMBOL),
            scene: export(module, abi::SCENE_SYMBOL),
            unload: export(module, abi::UNLOAD_SYMBOL),
            config: export(module, abi::CONFIG_SYMBOL),
        };
        if let Some(m) = lock(&MODS).get_mut(handle - 1) {
            m.callbacks = callbacks;
            m.active = true;
        }
        info!("loaded {} {} ({})", entry.name, entry.version, entry.id);
        true
    } else {
        error!("{} failed to load", entry.id);
        release(handle);
        false
    }
}

fn release(handle: usize) {
    if let Some(c) = lock(&COMMANDS).as_mut() {
        c.retain(|_, cmd| cmd.owner != handle);
    }
    let hooks: Vec<usize> = {
        let mut all = lock(&HOOKS);
        let mine = all
            .iter()
            .filter(|(h, _)| *h == handle)
            .map(|(_, t)| *t)
            .collect();
        all.retain(|(h, _)| *h != handle);
        mine
    };
    for target in hooks.into_iter().rev() {
        if let Err(e) = unsafe { crate::detour::uninstall(target as *mut c_void) } {
            warning!("cannot remove hook at {target:#x}: {e}");
        }
    }
    let patches: Vec<PointerPatch> = {
        let mut all = lock(&POINTERS);
        let mine = all.iter().filter(|p| p.owner == handle).copied().collect();
        all.retain(|p| p.owner != handle);
        mine
    };
    for p in patches.into_iter().rev() {
        let slot = p.slot as *mut usize;
        if unsafe { slot.read_volatile() } == p.value {
            unsafe { slot.write_volatile(p.previous) };
        }
    }
    let module = lock(&MODS).get_mut(handle - 1).map(|m| {
        m.active = false;
        m.callbacks = Callbacks::default();
        std::mem::take(&mut m.module)
    });
    if let Some(module) = module.filter(|&m| m != 0) {
        sys::free_library(module as Module);
    }
}

fn unload(handle: usize) {
    let unload = lock(&MODS).get(handle - 1).and_then(|m| m.callbacks.unload);
    if let Some(f) = unload {
        unsafe { f() };
    }
    release(handle);
}

pub fn reload(id: &str) -> bool {
    let found = {
        let mods = lock(&MODS);
        mods.iter()
            .position(|m| m.id.eq_ignore_ascii_case(id) && m.module != 0)
            .map(|i| (i, mods[i].file.clone()))
    };
    let Some((index, file)) = found else {
        warning!("native mod {id} is not loaded");
        return false;
    };
    if !DIRS.get().is_some_and(|d| d.hot_reload) {
        warning!(
            "set hot_reload = true in [mods] of ferrite.toml to reload native mods, the dll is locked otherwise"
        );
    }
    unload(index + 1);
    let Some(entry) = inspect(&file) else {
        error!("cannot reload {id}: {} did not load", file.display());
        return false;
    };
    if !entry.id.eq_ignore_ascii_case(id) {
        error!("cannot reload {id}: the new build has id {}", entry.id);
        sys::free_library(entry.module as Module);
        return false;
    }
    let ok = load(&entry, Some(index));
    report();
    if ok {
        info!("reloaded {} {}", entry.name, entry.version);
    }
    ok
}

fn report() {
    let rows = lock(&MODS)
        .iter()
        .filter(|m| m.active)
        .map(|m| crate::overlay::ModRow {
            id: m.id.clone(),
            name: m.name.clone(),
            version: m.version.clone(),
            author: String::new(),
            kind: "Rust",
            faulted: false,
        })
        .collect();
    crate::overlay::set_mods("Rust", rows);
}

fn check_reloads() {
    if !DIRS.get().is_some_and(|d| d.hot_reload) {
        return;
    }
    {
        let mut last = lock(&LAST_CHECK);
        let now = Instant::now();
        if last.is_some_and(|t| now.duration_since(t) < Duration::from_secs(1)) {
            return;
        }
        *last = Some(now);
    }
    let changed: Vec<String> = lock(&MODS)
        .iter()
        .filter(|m| m.module != 0 && stamp(&m.file).is_some_and(|s| Some(s) != m.stamp))
        .map(|m| m.id.clone())
        .collect();
    for id in changed {
        std::thread::sleep(Duration::from_millis(200));
        info!("{id} changed on disk, reloading");
        reload(&id);
    }
}

fn notify_configs() {
    let changed = config::with_mod_configs(config::ModConfigs::poll).unwrap_or_default();
    for id in changed {
        let callback = lock(&MODS)
            .iter()
            .find(|m| m.active && m.id.eq_ignore_ascii_case(&id))
            .and_then(|m| m.callbacks.config);
        if let Some(f) = callback {
            info!("config of {id} changed");
            unsafe { f() };
        }
    }
}

pub fn tick() {
    poll_commands();
    notify_configs();
    check_reloads();
    let updates: Vec<UpdateFn> = lock(&MODS)
        .iter()
        .filter(|m| m.active)
        .filter_map(|m| m.callbacks.update)
        .collect();
    for update in updates {
        unsafe { update() };
    }
}

pub fn scene_loaded(name: &str) {
    let scenes: Vec<SceneFn> = lock(&MODS)
        .iter()
        .filter(|m| m.active)
        .filter_map(|m| m.callbacks.scene)
        .collect();
    for scene in scenes {
        unsafe { scene(name.as_ptr(), name.len()) };
    }
}

fn source(handle: Handle) -> Option<(String, String)> {
    let index = (handle as usize).checked_sub(1)?;
    lock(&MODS)
        .get(index)
        .map(|m| (m.id.clone(), m.name.clone()))
}

fn print(text: &str) {
    console::print(text);
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
                    .and_then(|c| c.get(key).map(|c| c.callback));
                if let Some(callback) = command {
                    unsafe { callback(name.as_ptr(), name.len(), args.as_ptr(), args.len()) };
                } else if !console::builtin(&line) {
                    log::write(
                        Level::Warning,
                        "Console",
                        &format!("unknown command {name}, type help"),
                    );
                }
            }
        }
    }
}

fn help() {
    let mut lines = vec![
        "help - Lists commands.".to_owned(),
        "mods - Lists loaded mods.".to_owned(),
    ];
    lines.extend(console::help());
    lines.extend(
        lock(&COMMANDS)
            .iter()
            .flatten()
            .map(|(name, c)| format!("{name} - {}", c.description)),
    );
    lines.sort();
    for line in lines {
        print(&line);
    }
}

fn mods() {
    let mods = lock(&MODS).clone();
    let active = mods.iter().filter(|m| m.active).count();
    print(&format!(
        "Ferrite {}, {active} native mod(s):",
        env!("CARGO_PKG_VERSION")
    ));
    for m in mods.iter().filter(|m| m.active) {
        print(&format!("  {} {} ({})", m.name, m.version, m.id));
    }
}

unsafe extern "C" fn api_log(handle: Handle, level: i32, message: *const u8, len: usize) {
    let source = source(handle).map_or_else(|| "Mod".to_owned(), |(_, name)| name);
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

unsafe extern "C" fn api_hook_owned(
    handle: Handle,
    target: *mut c_void,
    detour: *mut c_void,
) -> *mut c_void {
    let Some(trampoline) = (unsafe { hook(target, detour) }) else {
        return std::ptr::null_mut();
    };
    lock(&HOOKS).push((handle as usize, target as usize));
    trampoline
}

unsafe extern "C" fn api_unhook(handle: Handle, target: *mut c_void) -> bool {
    let owned = {
        let mut hooks = lock(&HOOKS);
        let index = hooks
            .iter()
            .rposition(|&(h, t)| h == handle as usize && t == target as usize);
        index.map(|i| hooks.remove(i)).is_some()
    };
    if !owned {
        return false;
    }
    match unsafe { crate::detour::uninstall(target) } {
        Ok(()) => true,
        Err(e) => {
            warning!("cannot remove hook at {target:p}: {e}");
            false
        }
    }
}

unsafe extern "C" fn api_patch_pointer(
    handle: Handle,
    slot: *mut *mut c_void,
    value: *mut c_void,
) -> *mut c_void {
    if slot.is_null() {
        return std::ptr::null_mut();
    }
    let previous = unsafe { slot.read_volatile() };
    unsafe { slot.write_volatile(value) };
    lock(&POINTERS).push(PointerPatch {
        owner: handle as usize,
        slot: slot as usize,
        previous: previous as usize,
        value: value as usize,
    });
    previous
}

unsafe extern "C" fn api_execute(line: *const u8, len: usize) {
    let line = unsafe { read_str(line, len) };
    if !line.trim().is_empty() {
        log::push_command(line);
    }
}

unsafe extern "C" fn api_directory(
    handle: Handle,
    kind: i32,
    buffer: *mut u8,
    capacity: usize,
) -> isize {
    let Some(dirs) = DIRS.get() else { return -1 };
    let path = match kind {
        abi::DIR_ROOT => dirs.root.clone(),
        abi::DIR_GAME => dirs
            .root
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default(),
        abi::DIR_CONFIG => dirs.root.join("config"),
        abi::DIR_MOD => lock(&MODS)
            .get((handle as usize).wrapping_sub(1))
            .and_then(|m| m.file.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| dirs.mods.clone()),
        _ => return -1,
    };
    let text = path.to_string_lossy();
    let bytes = text.as_bytes();
    if bytes.len() <= capacity && !buffer.is_null() {
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, bytes.len()) };
    }
    isize::try_from(bytes.len()).unwrap_or(-1)
}

unsafe extern "C" fn api_command(
    handle: Handle,
    name: *const u8,
    name_len: usize,
    description: *const u8,
    description_len: usize,
    callback: CommandFn,
) {
    let name = unsafe { read_str(name, name_len) }.to_lowercase();
    if matches!(name.as_str(), "help" | "mods") || name.is_empty() || console::is_reserved(&name) {
        error!("command name {name} is reserved");
        return;
    }
    let description = unsafe { read_str(description, description_len) };
    lock(&COMMANDS).get_or_insert_with(HashMap::new).insert(
        name,
        Command {
            callback,
            description,
            owner: handle as usize,
        },
    );
}

unsafe extern "C" fn api_config_get(
    handle: Handle,
    key: *const u8,
    key_len: usize,
    buffer: *mut u8,
    capacity: usize,
) -> isize {
    let Some((id, _)) = source(handle) else {
        return -1;
    };
    let key = unsafe { read_str(key, key_len) };
    let Some(value) = config::with_mod_configs(|c| c.get(&id, &key)).flatten() else {
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
    let Some((id, _)) = source(handle) else {
        return;
    };
    let (key, value) = unsafe { (read_str(key, key_len), read_str(value, value_len)) };
    if let Some(Err(e)) =
        config::with_mod_configs(|c| c.set(&id, &key, &value, Kind::from_raw(kind)))
    {
        error!("cannot save config for {id}: {e}");
    }
}

unsafe extern "C" fn api_config_describe(
    handle: Handle,
    key: *const u8,
    key_len: usize,
    description: *const u8,
    description_len: usize,
) {
    let Some((id, _)) = source(handle) else {
        return;
    };
    let (key, description) = unsafe {
        (
            read_str(key, key_len),
            read_str(description, description_len),
        )
    };
    if let Some(Err(e)) = config::with_mod_configs(|c| c.describe(&id, &key, &description)) {
        error!("cannot save config for {id}: {e}");
    }
}
