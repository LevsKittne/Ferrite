use core::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};
use windows::Win32::Foundation::{HMODULE, HWND};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION, D3D11CreateDeviceAndSwapChain, ID3D11Device,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_MODE_DESC, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    DXGI_SWAP_CHAIN_DESC, DXGI_SWAP_EFFECT_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGISwapChain,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, WINDOW_EX_STYLE, WS_OVERLAPPEDWINDOW,
};
use windows::core::{HRESULT, Interface, w};

type PresentFn = unsafe extern "system" fn(*mut c_void, u32, u32) -> HRESULT;

static ORIGINAL: AtomicUsize = AtomicUsize::new(0);

pub fn install() -> Result<(), String> {
    if ORIGINAL.load(Ordering::SeqCst) != 0 {
        return Ok(());
    }
    let present = unsafe { find_present()? };
    let trampoline =
        unsafe { crate::detour::install(present as *mut c_void, present_hook as *const c_void) }
            .map_err(|e| e.to_string())?;
    ORIGINAL.store(trampoline as usize, Ordering::SeqCst);
    Ok(())
}

unsafe fn find_present() -> Result<usize, String> {
    unsafe {
        let instance = GetModuleHandleW(None).map_err(|e| e.to_string())?;
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("ferrite-overlay-probe"),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            8,
            8,
            None,
            None,
            Some(instance.into()),
            None,
        )
        .map_err(|e| format!("probe window: {e}"))?;
        let result = probe(hwnd);
        let _ = DestroyWindow(hwnd);
        result
    }
}

unsafe fn probe(hwnd: HWND) -> Result<usize, String> {
    let desc = DXGI_SWAP_CHAIN_DESC {
        BufferDesc: DXGI_MODE_DESC {
            Width: 8,
            Height: 8,
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            ..Default::default()
        },
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 1,
        OutputWindow: hwnd,
        Windowed: true.into(),
        SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
        Flags: 0,
    };
    let mut swapchain: Option<IDXGISwapChain> = None;
    let mut device: Option<ID3D11Device> = None;
    unsafe {
        D3D11CreateDeviceAndSwapChain(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_FLAG(0),
            None,
            D3D11_SDK_VERSION,
            Some(&raw const desc),
            Some(&raw mut swapchain),
            Some(&raw mut device),
            None,
            None,
        )
        .map_err(|e| format!("probe device: {e}"))?;
        let sc = swapchain.ok_or("no probe swapchain")?;
        let vtable = *sc.as_raw().cast::<*const usize>();
        Ok(*vtable.add(8))
    }
}

unsafe extern "system" fn present_hook(this: *mut c_void, sync: u32, flags: u32) -> HRESULT {
    let _ = std::panic::catch_unwind(|| unsafe { super::frame(this) });
    let original: PresentFn =
        unsafe { std::mem::transmute::<usize, PresentFn>(ORIGINAL.load(Ordering::SeqCst)) };
    unsafe { original(this, sync, flags) }
}
