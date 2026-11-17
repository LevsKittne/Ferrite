use iced_x86::{
    BlockEncoder, BlockEncoderOptions, Decoder, DecoderOptions, FlowControl, Instruction,
    InstructionBlock,
};
use std::ffi::c_void;
use std::fmt;
use std::sync::Mutex;

#[cfg(target_pointer_width = "64")]
const BITNESS: u32 = 64;
#[cfg(target_pointer_width = "64")]
const JUMP_LEN: usize = 14;
#[cfg(target_pointer_width = "32")]
const BITNESS: u32 = 32;
#[cfg(target_pointer_width = "32")]
const JUMP_LEN: usize = 5;

const MAX_PROLOGUE: usize = 64;
const TRAMPOLINE_SIZE: usize = 256;

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

#[cfg(target_pointer_width = "64")]
fn jump(_from: u64, to: u64) -> [u8; JUMP_LEN] {
    let mut code = [0u8; JUMP_LEN];
    code[..6].copy_from_slice(&[0xFF, 0x25, 0, 0, 0, 0]);
    code[6..].copy_from_slice(&to.to_le_bytes());
    code
}

#[cfg(target_pointer_width = "32")]
fn jump(from: u64, to: u64) -> [u8; JUMP_LEN] {
    let mut code = [0u8; JUMP_LEN];
    code[0] = 0xE9;
    #[allow(clippy::cast_possible_truncation)]
    let rel = (to as u32).wrapping_sub((from as u32).wrapping_add(JUMP_LEN as u32));
    code[1..].copy_from_slice(&rel.to_le_bytes());
    code
}

fn prologue(target: u64) -> Result<(Vec<Instruction>, usize), DetourError> {
    let bytes = unsafe { std::slice::from_raw_parts(target as *const u8, MAX_PROLOGUE) };
    let mut decoder = Decoder::with_ip(BITNESS, bytes, target, DecoderOptions::NONE);
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

#[cfg(windows)]
mod memory {
    use super::{DetourError, TRAMPOLINE_SIZE};
    use std::ffi::c_void;
    use windows_sys::Win32::System::Diagnostics::Debug::FlushInstructionCache;
    use windows_sys::Win32::System::Memory::{
        MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS, VirtualAlloc,
        VirtualProtect,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    pub fn reserve(at: usize) -> Option<*mut u8> {
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

    #[cfg(target_pointer_width = "64")]
    pub fn allocate_near(target: usize) -> Option<*mut u8> {
        use windows_sys::Win32::System::Memory::{
            MEM_FREE, MEMORY_BASIC_INFORMATION, VirtualQuery,
        };
        const REACH: usize = 0x4000_0000;
        const GRANULARITY: usize = 0x1_0000;
        let free_regions = |from: usize, to: usize| {
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
        };
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

    pub fn flush(at: *mut u8, len: usize) {
        unsafe { FlushInstructionCache(GetCurrentProcess(), at.cast(), len) };
    }

    pub unsafe fn write_code(at: *mut u8, code: &[u8]) -> Result<(), DetourError> {
        let mut old: PAGE_PROTECTION_FLAGS = 0;
        if unsafe { VirtualProtect(at.cast(), code.len(), PAGE_EXECUTE_READWRITE, &raw mut old) }
            == 0
        {
            return Err(DetourError::Protect);
        }
        unsafe { std::ptr::copy_nonoverlapping(code.as_ptr(), at, code.len()) };
        unsafe { VirtualProtect(at.cast(), code.len(), old, &raw mut old) };
        flush(at, code.len());
        Ok(())
    }
}

#[cfg(unix)]
mod memory {
    use super::{DetourError, TRAMPOLINE_SIZE};

    const RWX: i32 = libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC;

    fn page_size() -> usize {
        usize::try_from(unsafe { libc::sysconf(libc::_SC_PAGESIZE) }).unwrap_or(4096)
    }

    pub fn reserve(at: usize) -> Option<*mut u8> {
        let p = unsafe {
            libc::mmap(
                at as *mut libc::c_void,
                TRAMPOLINE_SIZE,
                RWX,
                libc::MAP_PRIVATE | libc::MAP_ANON,
                -1,
                0,
            )
        };
        (p != libc::MAP_FAILED).then_some(p.cast())
    }

    #[cfg(target_pointer_width = "64")]
    pub fn allocate_near(target: usize) -> Option<*mut u8> {
        const REACH: usize = 0x4000_0000;
        const GRANULARITY: usize = 0x1_0000;
        let base = target & !(GRANULARITY - 1);
        for step in 1..REACH / GRANULARITY {
            for at in [
                base.checked_add(step * GRANULARITY),
                base.checked_sub(step * GRANULARITY),
            ]
            .into_iter()
            .flatten()
            {
                let Some(p) = reserve(at) else { continue };
                if (p as usize).abs_diff(target) < REACH {
                    return Some(p);
                }
                unsafe { libc::munmap(p.cast(), TRAMPOLINE_SIZE) };
            }
        }
        None
    }

    pub fn flush(_at: *mut u8, _len: usize) {}

    pub unsafe fn write_code(at: *mut u8, code: &[u8]) -> Result<(), DetourError> {
        let page = page_size();
        let start = at as usize & !(page - 1);
        let len = at as usize + code.len() - start;
        if unsafe { libc::mprotect(start as *mut libc::c_void, len, RWX) } != 0 {
            return Err(DetourError::Protect);
        }
        unsafe { std::ptr::copy_nonoverlapping(code.as_ptr(), at, code.len()) };
        unsafe {
            libc::mprotect(
                start as *mut libc::c_void,
                len,
                libc::PROT_READ | libc::PROT_EXEC,
            )
        };
        Ok(())
    }
}

#[cfg(target_pointer_width = "64")]
fn allocate_trampoline(target: usize) -> Option<*mut u8> {
    memory::allocate_near(target)
}

#[cfg(target_pointer_width = "32")]
fn allocate_trampoline(_target: usize) -> Option<*mut u8> {
    memory::reserve(0)
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
    let trampoline = allocate_trampoline(target as usize).ok_or(DetourError::NoMemoryNearTarget)?;
    let block = InstructionBlock::new(&instructions, trampoline as u64);
    let mut code = BlockEncoder::encode(BITNESS, block, BlockEncoderOptions::NONE)
        .map_err(|e| DetourError::Encode(e.to_string()))?
        .code_buffer;
    let back = jump(trampoline as u64 + code.len() as u64, target_ip + len as u64);
    code.extend_from_slice(&back);
    if code.len() > TRAMPOLINE_SIZE {
        return Err(DetourError::Encode(
            "relocated prologue is too large".into(),
        ));
    }
    unsafe { std::ptr::copy_nonoverlapping(code.as_ptr(), trampoline, code.len()) };
    memory::flush(trampoline, code.len());
    let mut patch = jump(target_ip, detour as u64).to_vec();
    patch.resize(len, 0x90);
    let original = unsafe { std::slice::from_raw_parts(target.cast::<u8>(), len) }.to_vec();
    unsafe { memory::write_code(target.cast(), &patch) }?;
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
    unsafe { memory::write_code(target.cast(), &entry.original) }?;
    installed.remove(index);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{JUMP_LEN, install, jump, memory, prologue, uninstall};

    extern "C" fn two() -> usize {
        2
    }

    #[cfg(target_pointer_width = "64")]
    const RETURN_ONE: [u8; 16] = [
        0x48, 0xC7, 0xC0, 0x01, 0x00, 0x00, 0x00, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
        0xC3,
    ];

    #[cfg(target_pointer_width = "32")]
    const RETURN_ONE: [u8; 16] = [
        0x55, 0x89, 0xE5, 0xB8, 0x01, 0x00, 0x00, 0x00, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x5D,
        0xC3,
    ];

    #[test]
    fn hook_and_unhook_live_code() {
        let page = memory::reserve(0).unwrap();
        unsafe { std::ptr::copy_nonoverlapping(RETURN_ONE.as_ptr(), page, RETURN_ONE.len()) };
        let target: extern "C" fn() -> usize = unsafe { std::mem::transmute(page) };
        assert_eq!(target(), 1);
        let trampoline = unsafe { install(page.cast(), (two as *const ()).cast()) }.unwrap();
        let original: extern "C" fn() -> usize = unsafe { std::mem::transmute(trampoline) };
        assert_eq!(target(), 2);
        assert_eq!(original(), 1);
        unsafe { uninstall(page.cast()) }.unwrap();
        assert_eq!(target(), 1);
        assert!(unsafe { uninstall(page.cast()) }.is_err());
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn absolute_jump_layout() {
        let code = jump(0, 0x1122_3344_5566_7788);
        assert_eq!(code.len(), JUMP_LEN);
        assert_eq!(&code[..6], &[0xFF, 0x25, 0, 0, 0, 0]);
        assert_eq!(
            u64::from_le_bytes(code[6..].try_into().unwrap()),
            0x1122_3344_5566_7788
        );
    }

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn relative_jump_layout() {
        assert_eq!(jump(0x1000, 0x2000), [0xE9, 0xFB, 0x0F, 0x00, 0x00]);
        assert_eq!(jump(0x2000, 0x1000), [0xE9, 0xFB, 0xEF, 0xFF, 0xFF]);
        assert_eq!(JUMP_LEN, 5);
    }

    #[cfg(target_pointer_width = "64")]
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

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn prologue_with_frame_setup() {
        let code: [u8; 64] = {
            let mut c = [0x90u8; 64];
            c[..6].copy_from_slice(&[0x55, 0x8B, 0xEC, 0x83, 0xEC, 0x08]);
            c
        };
        let (instructions, len) = prologue(code.as_ptr() as u64).unwrap();
        assert_eq!(instructions.len(), 3);
        assert_eq!(len, 6);
    }

    #[test]
    fn rejects_tiny_function() {
        let code = [0xC3u8; 64];
        assert!(prologue(code.as_ptr() as u64).is_err());
    }
}
