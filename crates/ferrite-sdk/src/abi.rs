use std::ffi::{c_char, c_void};

pub const ABI_VERSION: u32 = 3;
pub const MIN_ABI_VERSION: u32 = 1;

pub const INFO_SYMBOL: &[u8] = b"ferrite_mod_info\0";
pub const LOAD_SYMBOL: &[u8] = b"ferrite_mod_load\0";
pub const UPDATE_SYMBOL: &[u8] = b"ferrite_mod_update\0";
pub const SCENE_SYMBOL: &[u8] = b"ferrite_mod_scene\0";
pub const UNLOAD_SYMBOL: &[u8] = b"ferrite_mod_unload\0";
pub const CONFIG_SYMBOL: &[u8] = b"ferrite_mod_config_changed\0";

#[repr(C)]
#[derive(Debug)]
pub struct ModInfo {
    pub abi: u32,
    pub id: *const c_char,
    pub name: *const c_char,
    pub version: *const c_char,
    pub dependencies: *const c_char,
}

unsafe impl Sync for ModInfo {}

pub type Handle = *const c_void;

pub type CommandFn =
    unsafe extern "C" fn(name: *const u8, name_len: usize, args: *const u8, args_len: usize);

#[repr(C)]
#[derive(Debug)]
pub struct Api {
    pub abi: u32,
    pub log: unsafe extern "C" fn(handle: Handle, level: i32, message: *const u8, len: usize),
    pub symbol: unsafe extern "C" fn(name: *const c_char) -> *mut c_void,
    pub hook: unsafe extern "C" fn(target: *mut c_void, detour: *mut c_void) -> *mut c_void,
    pub command: unsafe extern "C" fn(
        handle: Handle,
        name: *const u8,
        name_len: usize,
        description: *const u8,
        description_len: usize,
        callback: CommandFn,
    ),
    pub config_get: unsafe extern "C" fn(
        handle: Handle,
        key: *const u8,
        key_len: usize,
        buffer: *mut u8,
        capacity: usize,
    ) -> isize,
    pub config_set: unsafe extern "C" fn(
        handle: Handle,
        key: *const u8,
        key_len: usize,
        value: *const u8,
        value_len: usize,
        kind: i32,
    ),
    pub config_describe: unsafe extern "C" fn(
        handle: Handle,
        key: *const u8,
        key_len: usize,
        description: *const u8,
        description_len: usize,
    ),
    pub hook_owned: unsafe extern "C" fn(
        handle: Handle,
        target: *mut c_void,
        detour: *mut c_void,
    ) -> *mut c_void,
    pub unhook: unsafe extern "C" fn(handle: Handle, target: *mut c_void) -> bool,
    pub patch_pointer: unsafe extern "C" fn(
        handle: Handle,
        slot: *mut *mut c_void,
        value: *mut c_void,
    ) -> *mut c_void,
    pub execute: unsafe extern "C" fn(line: *const u8, len: usize),
    pub directory:
        unsafe extern "C" fn(handle: Handle, kind: i32, buffer: *mut u8, capacity: usize) -> isize,
    pub register_class: unsafe extern "C" fn(
        handle: Handle,
        class: *mut c_void,
        namespace: *const c_char,
        name: *const c_char,
    ) -> i64,
    pub stub: unsafe extern "C" fn(kind: i32) -> *mut c_void,
}

pub type InfoFn = unsafe extern "C" fn() -> *const ModInfo;
pub type LoadFn = unsafe extern "C" fn(api: *const Api, handle: Handle) -> bool;
pub type UpdateFn = unsafe extern "C" fn();
pub type SceneFn = unsafe extern "C" fn(name: *const u8, len: usize);
pub type UnloadFn = unsafe extern "C" fn();
pub type ConfigFn = unsafe extern "C" fn();

pub const LEVEL_DEBUG: i32 = 0;
pub const LEVEL_INFO: i32 = 1;
pub const LEVEL_WARNING: i32 = 2;
pub const LEVEL_ERROR: i32 = 3;

pub const KIND_STRING: i32 = 0;
pub const KIND_BOOL: i32 = 1;
pub const KIND_INT: i32 = 2;
pub const KIND_FLOAT: i32 = 3;

pub const DIR_ROOT: i32 = 0;
pub const DIR_GAME: i32 = 1;
pub const DIR_MOD: i32 = 2;
pub const DIR_CONFIG: i32 = 3;

pub const STUB_METHOD: i32 = 0;
pub const STUB_INVOKER: i32 = 1;
