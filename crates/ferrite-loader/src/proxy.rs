use crate::sys;
use std::sync::atomic::{AtomicUsize, Ordering};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

macro_rules! forward {
    ($($name:ident),* $(,)?) => {
        #[allow(non_upper_case_globals)]
        mod slots {
            use std::sync::atomic::AtomicUsize;
            $(pub static $name: AtomicUsize = AtomicUsize::new(0);)*
        }

        $(
            #[allow(non_snake_case)]
            #[unsafe(no_mangle)]
            #[unsafe(naked)]
            pub unsafe extern "C" fn $name() {
                #[cfg(target_arch = "x86_64")]
                core::arch::naked_asm!("jmp qword ptr [rip + {0}]", sym slots::$name);
                #[cfg(target_arch = "x86")]
                core::arch::naked_asm!("jmp dword ptr [{0}]", sym slots::$name);
            }
        )*

        const SLOTS: &[(&str, &AtomicUsize)] = &[$((concat!(stringify!($name), "\0"), &slots::$name)),*];
    };
}

forward!(
    GetFileVersionInfoA,
    GetFileVersionInfoByHandle,
    GetFileVersionInfoExA,
    GetFileVersionInfoExW,
    GetFileVersionInfoSizeA,
    GetFileVersionInfoSizeExA,
    GetFileVersionInfoSizeExW,
    GetFileVersionInfoSizeW,
    GetFileVersionInfoW,
    VerFindFileA,
    VerFindFileW,
    VerInstallFileA,
    VerInstallFileW,
    VerLanguageNameA,
    VerLanguageNameW,
    VerQueryValueA,
    VerQueryValueW,
);

#[derive(Debug)]
pub struct ProxyError(String);

impl std::fmt::Display for ProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

pub fn init() -> Result<(), ProxyError> {
    let path = sys::system_dir().join("version.dll");
    let module = unsafe { LoadLibraryW(sys::wide(&path).as_ptr()) };
    if module.is_null() {
        return Err(ProxyError(format!("cannot load {}", path.display())));
    }
    let mut missing = 0;
    for (name, slot) in SLOTS {
        match unsafe { GetProcAddress(module, name.as_ptr()) } {
            Some(f) => slot.store(f as usize, Ordering::Release),
            None => missing += 1,
        }
    }
    if missing == 0 {
        Ok(())
    } else {
        Err(ProxyError(format!(
            "{missing} exports missing in {}",
            path.display()
        )))
    }
}
