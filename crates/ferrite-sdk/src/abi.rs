use std::ffi::{c_char, c_void};

pub const ABI_VERSION: u32 = 1;

pub const INFO_SYMBOL: &[u8] = b"ferrite_mod_info\0";
pub const LOAD_SYMBOL: &[u8] = b"ferrite_mod_load\0";
pub const UPDATE_SYMBOL: &[u8] = b"ferrite_mod_update\0";
pub const SCENE_SYMBOL: &[u8] = b"ferrite_mod_scene\0";

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
}

pub type InfoFn = unsafe extern "C" fn() -> *const ModInfo;
pub type LoadFn = unsafe extern "C" fn(api: *const Api, handle: Handle) -> bool;
pub type UpdateFn = unsafe extern "C" fn();
pub type SceneFn = unsafe extern "C" fn(name: *const u8, len: usize);

pub const LEVEL_DEBUG: i32 = 0;
pub const LEVEL_INFO: i32 = 1;
pub const LEVEL_WARNING: i32 = 2;
pub const LEVEL_ERROR: i32 = 3;

pub const KIND_STRING: i32 = 0;
pub const KIND_BOOL: i32 = 1;
pub const KIND_INT: i32 = 2;
pub const KIND_FLOAT: i32 = 3;
