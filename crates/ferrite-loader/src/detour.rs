use iced_x86::{
    BlockEncoder, BlockEncoderOptions, Decoder, DecoderOptions, FlowControl, Instruction,
    InstructionBlock,
};
use std::ffi::c_void;
use std::fmt;
use std::sync::Mutex;
use windows_sys::Win32::System::Diagnostics::Debug::FlushInstructionCache;
use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEM_FREE, MEM_RESERVE, MEMORY_BASIC_INFORMATION, PAGE_EXECUTE_READWRITE,
    PAGE_PROTECTION_FLAGS, VirtualAlloc, VirtualProtect, VirtualQuery,
};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

const JUMP_LEN: usize = 14;
const MAX_PROLOGUE: usize = 64;
const TRAMPOLINE_SIZE: usize = 256;
const REACH: usize = 0x4000_0000;
const GRANULARITY: usize = 0x1_0000;

static PATCH_LOCK: Mutex<()> = Mutex::new(());
static INSTALLED: Mutex<Vec<Installed>> = Mutex::new(Vec::new());

#[derive(Debug)]
struct Installed {
    target: usize,
    original: Vec<u8>,
    patch: Vec<u8>,
}

#[derive(Debug)]
pub enum DetourError {
    InvalidInstruction(usize),
    TooShort,
    Encode(String),
    NoMemoryNearTarget,
    Protect,
    NotHooked,
    Overwritten,
}

impl fmt::Display for DetourError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInstruction(at) => write!(f, "cannot decode instruction at offset {at}"),
            Self::TooShort => f.write_str("function is too short to hook"),
            Self::Encode(e) => write!(f, "cannot relocate prologue: {e}"),
            Self::NoMemoryNearTarget => f.write_str("no free memory within 2 GB of the target"),
            Self::Protect => f.write_str("cannot change page protection"),
            Self::NotHooked => f.write_str("no hook installed there"),
            Self::Overwritten => {
                f.write_str("hooked again by someone else, remove that hook first")
            }
        }
    }
}

fn absolute_jump(to: u64) -> [u8; JUMP_LEN] {
    let mut code = [0u8; JUMP_LEN];
    code[..6].copy_from_slice(&[0xFF, 0x25, 0, 0, 0, 0]);
    code[6..].copy_from_slice(&to.to_le_bytes());
    code
}

fn prologue(target: u64) -> Result<(Vec<Instruction>, usize), DetourError> {
    let bytes = unsafe { std::slice::from_raw_parts(target as *const u8, MAX_PROLOGUE) };
    let mut decoder = Decoder::with_ip(64, bytes, target, DecoderOptions::NONE);
    let mut instructions = Vec::new();
    let mut len = 0;
    while len < JUMP_LEN {
        let instruction = decoder.decode();
        if instruction.is_invalid() {
            return Err(DetourError::InvalidInstruction(len));
        }
        len += instruction.len();
        let ends = matches!(
            instruction.flow_control(),
            FlowControl::Return | FlowControl::UnconditionalBranch | FlowControl::Interrupt
        );
        instructions.push(instruction);
        if ends && len < JUMP_LEN {
            return Err(DetourError::TooShort);
        }
    }
    Ok((instructions, len))
}

fn free_regions(from: usize, to: usize) -> Vec<(usize, usize)> {
    let mut regions = Vec::new();
    let mut address = from.max(GRANULARITY);
    while address < to {
        let mut info: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
        if unsafe {
            VirtualQuery(
                address as *const c_void,
                &raw mut info,
                size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        } == 0
        {
            break;
        }
        let base = info.BaseAddress as usize;
        let end = base + info.RegionSize;
        if info.State == MEM_FREE {
            regions.push((base.max(from), end.min(to)));
        }
        address = end.max(address + GRANULARITY);
    }
    regions
}

fn reserve(at: usize) -> Option<*mut u8> {
    let p = unsafe {
        VirtualAlloc(
            at as *const c_void,
            TRAMPOLINE_SIZE,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        )
    };
    (!p.is_null()).then_some(p.cast())
}

fn allocate_near(target: usize) -> Option<*mut u8> {
    let align_up = |v: usize| (v + GRANULARITY - 1) & !(GRANULARITY - 1);
    let align_down = |v: usize| v & !(GRANULARITY - 1);
    let above = free_regions(target, target.saturating_add(REACH))
        .into_iter()
        .find_map(|(start, end)| {
            let at = align_up(start);
            (at + TRAMPOLINE_SIZE <= end).then(|| reserve(at)).flatten()
        });
    above.or_else(|| {
        free_regions(target.saturating_sub(REACH), target)
            .into_iter()
            .rev()
            .find_map(|(start, end)| {
                let at = align_down(end.saturating_sub(TRAMPOLINE_SIZE));
                (at >= start).then(|| reserve(at)).flatten()
            })
    })
}

unsafe fn write_code(at: *mut u8, code: &[u8]) -> Result<(), DetourError> {
    let mut old: PAGE_PROTECTION_FLAGS = 0;
    if unsafe { VirtualProtect(at.cast(), code.len(), PAGE_EXECUTE_READWRITE, &raw mut old) } == 0 {
        return Err(DetourError::Protect);
    }
    unsafe { std::ptr::copy_nonoverlapping(code.as_ptr(), at, code.len()) };
    unsafe {
        VirtualProtect(at.cast(), code.len(), old, &raw mut old);
        FlushInstructionCache(GetCurrentProcess(), at.cast(), code.len());
    }
    Ok(())
}

pub unsafe fn install(
    target: *mut c_void,
    detour: *const c_void,
) -> Result<*const c_void, DetourError> {
    let _guard = PATCH_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let target_ip = target as u64;
    let (instructions, len) = prologue(target_ip)?;
    let trampoline = allocate_near(target as usize).ok_or(DetourError::NoMemoryNearTarget)?;
    let block = InstructionBlock::new(&instructions, trampoline as u64);
    let mut code = BlockEncoder::encode(64, block, BlockEncoderOptions::NONE)
        .map_err(|e| DetourError::Encode(e.to_string()))?
        .code_buffer;
    code.extend_from_slice(&absolute_jump(target_ip + len as u64));
    if code.len() > TRAMPOLINE_SIZE {
        return Err(DetourError::Encode(
            "relocated prologue is too large".into(),
        ));
    }
    unsafe { std::ptr::copy_nonoverlapping(code.as_ptr(), trampoline, code.len()) };
    unsafe { FlushInstructionCache(GetCurrentProcess(), trampoline.cast(), code.len()) };
    let mut patch = absolute_jump(detour as u64).to_vec();
    patch.resize(len, 0x90);
    let original = unsafe { std::slice::from_raw_parts(target.cast::<u8>(), len) }.to_vec();
    unsafe { write_code(target.cast(), &patch) }?;
    INSTALLED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(Installed {
            target: target as usize,
            original,
            patch,
        });
    Ok(trampoline.cast_const().cast())
}

pub unsafe fn uninstall(target: *mut c_void) -> Result<(), DetourError> {
    let _guard = PATCH_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut installed = INSTALLED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(index) = installed.iter().rposition(|i| i.target == target as usize) else {
        return Err(DetourError::NotHooked);
    };
    let entry = &installed[index];
    let current = unsafe { std::slice::from_raw_parts(target.cast::<u8>(), entry.patch.len()) };
    if current != entry.patch.as_slice() {
        return Err(DetourError::Overwritten);
    }
    unsafe { write_code(target.cast(), &entry.original) }?;
    installed.remove(index);
    Ok(())
}

pub unsafe fn jump_targets(start: *const c_void, limit: usize) -> Vec<usize> {
    let bytes = unsafe { std::slice::from_raw_parts(start.cast::<u8>(), limit) };
    let mut decoder = Decoder::with_ip(64, bytes, start as u64, DecoderOptions::NONE);
    let mut targets = Vec::new();
    while decoder.can_decode() {
        let instruction = decoder.decode();
        if instruction.is_invalid() {
            break;
        }
        match instruction.flow_control() {
            FlowControl::Call => targets.push(instruction.near_branch_target()),
            FlowControl::UnconditionalBranch => {
                targets.push(instruction.near_branch_target());
                break;
            }
            FlowControl::Return | FlowControl::Interrupt | FlowControl::IndirectBranch => break,
            _ => {}
        }
    }
    targets
        .into_iter()
        .filter(|&t| t != 0)
        .filter_map(|t| usize::try_from(t).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{JUMP_LEN, absolute_jump, install, jump_targets, prologue, uninstall};
    use windows_sys::Win32::System::Memory::{
        MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE, VirtualAlloc,
    };

    extern "C" fn two() -> u64 {
        2
    }

    #[test]
    fn hook_and_unhook_live_code() {
        let code: [u8; 16] = [
            0x48, 0xC7, 0xC0, 0x01, 0x00, 0x00, 0x00, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
            0x90, 0xC3,
        ];
        let page = unsafe {
            VirtualAlloc(
                std::ptr::null(),
                4096,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_EXECUTE_READWRITE,
            )
        }
        .cast::<u8>();
        assert!(!page.is_null());
        unsafe { std::ptr::copy_nonoverlapping(code.as_ptr(), page, code.len()) };
        let target: extern "C" fn() -> u64 = unsafe { std::mem::transmute(page) };
        assert_eq!(target(), 1);
        let trampoline = unsafe { install(page.cast(), (two as *const ()).cast()) }.unwrap();
        let original: extern "C" fn() -> u64 = unsafe { std::mem::transmute(trampoline) };
        assert_eq!(target(), 2);
        assert_eq!(original(), 1);
        unsafe { uninstall(page.cast()) }.unwrap();
        assert_eq!(target(), 1);
        assert!(unsafe { uninstall(page.cast()) }.is_err());
    }

    #[test]
    fn absolute_jump_layout() {
        let code = absolute_jump(0x1122_3344_5566_7788);
        assert_eq!(code.len(), JUMP_LEN);
        assert_eq!(&code[..6], &[0xFF, 0x25, 0, 0, 0, 0]);
        assert_eq!(
            u64::from_le_bytes(code[6..].try_into().unwrap()),
            0x1122_3344_5566_7788
        );
    }

    #[test]
    fn prologue_with_rip_relative_cmp() {
        let code: [u8; 64] = {
            let mut c = [0x90u8; 64];
            c[..13].copy_from_slice(&[
                0x48, 0x83, 0xEC, 0x28, 0x80, 0x3D, 0x6E, 0x85, 0x91, 0x00, 0x00, 0x75, 0x13,
            ]);
            c
        };
        let (instructions, len) = prologue(code.as_ptr() as u64).unwrap();
        assert_eq!(instructions.len(), 4);
        assert_eq!(len, 14);
    }

    #[test]
    fn finds_call_and_tail_jump_targets() {
        let mut code = [0xCCu8; 64];
        code[..14].copy_from_slice(&[
            0x48, 0x83, 0xEC, 0x28, 0xE8, 0x10, 0x00, 0x00, 0x00, 0xE9, 0x20, 0x00, 0x00, 0x00,
        ]);
        let base = code.as_ptr() as usize;
        let targets = unsafe { jump_targets(code.as_ptr().cast(), code.len()) };
        assert_eq!(targets, vec![base + 9 + 0x10, base + 14 + 0x20]);
    }

    #[test]
    fn rejects_tiny_function() {
        let code = [0xC3u8; 64];
        assert!(prologue(code.as_ptr() as u64).is_err());
    }
}
