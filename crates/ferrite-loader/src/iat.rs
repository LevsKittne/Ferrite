use crate::sys;
use std::ffi::c_void;
use windows_sys::Win32::Foundation::HMODULE;
use windows_sys::Win32::System::Memory::{PAGE_PROTECTION_FLAGS, PAGE_READWRITE, VirtualProtect};

const DOS_MAGIC: u16 = 0x5A4D;
const PE_MAGIC: u32 = 0x4550;
const PE32_PLUS: u16 = 0x20B;
const IMPORT_DESCRIPTOR_SIZE: usize = 20;
const ORDINAL_FLAG: u64 = 1 << 63;

unsafe fn read<T: Copy>(addr: usize) -> T {
    unsafe { std::ptr::read_unaligned(addr as *const T) }
}

pub unsafe fn hook(module: HMODULE, dll: &str, func: &str, replacement: usize) -> Option<usize> {
    let base = module as usize;
    if base == 0 || unsafe { read::<u16>(base) } != DOS_MAGIC {
        return None;
    }
    let nt = base + unsafe { read::<u32>(base + 0x3C) } as usize;
    if unsafe { read::<u32>(nt) } != PE_MAGIC {
        return None;
    }
    let optional = nt + 24;
    if unsafe { read::<u16>(optional) } != PE32_PLUS {
        return None;
    }
    let import_rva = unsafe { read::<u32>(optional + 120) } as usize;
    if import_rva == 0 {
        return None;
    }
    let mut desc = base + import_rva;
    loop {
        let name_rva = unsafe { read::<u32>(desc + 12) } as usize;
        if name_rva == 0 {
            return None;
        }
        let name = unsafe { sys::c_bytes((base + name_rva) as *const u8) };
        if name.eq_ignore_ascii_case(dll.as_bytes()) {
            let names = unsafe { read::<u32>(desc) } as usize;
            let slots = unsafe { read::<u32>(desc + 16) } as usize;
            if let Some(slot) = unsafe { find_slot(base, names, slots, func) } {
                return unsafe { patch(slot, replacement) };
            }
        }
        desc += IMPORT_DESCRIPTOR_SIZE;
    }
}

unsafe fn find_slot(base: usize, names: usize, slots: usize, func: &str) -> Option<*mut usize> {
    if names == 0 {
        return None;
    }
    (0..)
        .map_while(|i| {
            let thunk = unsafe { read::<u64>(base + names + i * 8) };
            (thunk != 0).then_some((i, thunk))
        })
        .filter(|&(_, thunk)| thunk & ORDINAL_FLAG == 0)
        .find(|&(_, thunk)| {
            let by_name = base + usize::try_from(thunk & 0x7FFF_FFFF).unwrap_or_default() + 2;
            (unsafe { sys::c_bytes(by_name as *const u8) }) == func.as_bytes()
        })
        .map(|(i, _)| (base + slots + i * 8) as *mut usize)
}

unsafe fn patch(slot: *mut usize, replacement: usize) -> Option<usize> {
    let mut old: PAGE_PROTECTION_FLAGS = 0;
    if unsafe { VirtualProtect(slot.cast::<c_void>(), 8, PAGE_READWRITE, &raw mut old) } == 0 {
        return None;
    }
    let previous = unsafe { slot.replace(replacement) };
    unsafe { VirtualProtect(slot.cast::<c_void>(), 8, old, &raw mut old) };
    Some(previous)
}
