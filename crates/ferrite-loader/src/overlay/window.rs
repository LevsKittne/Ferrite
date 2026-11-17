use super::{VISIBLE, WANTS_KEYBOARD, input, ui};
use crate::log::{info, warning};
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HMODULE, HWND, LPARAM, LRESULT, POINT, RECT, TRUE, WPARAM};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, D3D11CreateDevice, ID3D11Device,
    ID3D11DeviceContext, ID3D11RenderTargetView, ID3D11Texture2D,
};
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice, IDCompositionDevice, IDCompositionTarget, IDCompositionVisual,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_UNKNOWN,
    DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory2, DXGI_CREATE_FACTORY_FLAGS, DXGI_PRESENT, DXGI_SCALING_STRETCH,
    DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_CHAIN_FLAG, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
    DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIDevice, IDXGIFactory2, IDXGISwapChain1,
};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, EnumWindows, GW_HWNDPREV, GWL_EXSTYLE,
    GetClassNameW, GetClientRect, GetWindow, GetWindowLongW, GetWindowThreadProcessId, HWND_TOP,
    HWND_TOPMOST, IsIconic, IsWindow, IsWindowVisible, MA_NOACTIVATE, MSG, PM_REMOVE, PeekMessageW,
    RegisterClassExW, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SetWindowPos, ShowWindow, TranslateMessage, WM_MOUSEACTIVATE, WNDCLASSEXW, WS_EX_NOACTIVATE,
    WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::BOOL;
use windows::core::{Interface, w};

static STARTED: AtomicBool = AtomicBool::new(false);
static GAME: AtomicIsize = AtomicIsize::new(0);

pub fn start() {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    let _ = std::thread::Builder::new()
        .name("ferrite-menu".into())
        .spawn(|| {
            if let Err(e) = std::panic::catch_unwind(run).unwrap_or(Err("panicked".into())) {
                warning!("menu window failed: {e}");
            }
        });
}

unsafe extern "system" fn enum_proc(hwnd: HWND, found: LPARAM) -> BOOL {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&raw mut pid)) };
    if pid != unsafe { GetCurrentProcessId() } || !unsafe { IsWindowVisible(hwnd) }.as_bool() {
        return TRUE;
    }
    let mut class = [0u16; 64];
    let len = unsafe { GetClassNameW(hwnd, &mut class) };
    let name = String::from_utf16_lossy(&class[..usize::try_from(len).unwrap_or(0)]);
    if name == "UnityWndClass" {
        unsafe { *(found.0 as *mut isize) = hwnd.0 as isize };
        return BOOL(0);
    }
    TRUE
}

fn find_game() -> Option<HWND> {
    let mut found: isize = 0;
    unsafe {
        let _ = EnumWindows(Some(enum_proc), LPARAM(&raw mut found as isize));
    }
    (found != 0).then_some(HWND(found as *mut _))
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_MOUSEACTIVATE {
        return LRESULT(isize::try_from(MA_NOACTIVATE).unwrap_or(3));
    }
    if std::panic::catch_unwind(|| input::handle(msg, wp.0, lp.0)).unwrap_or(false) {
        return LRESULT(0);
    }
    unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
}

struct Gpu {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    swapchain: IDXGISwapChain1,
    _dcomp: IDCompositionDevice,
    _target: IDCompositionTarget,
    _visual: IDCompositionVisual,
    size: (u32, u32),
}

fn gpu(hwnd: HWND, width: u32, height: u32) -> windows::core::Result<Gpu> {
    unsafe {
        let mut device: Option<ID3D11Device> = None;
        let mut context: Option<ID3D11DeviceContext> = None;
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&raw mut device),
            None,
            Some(&raw mut context),
        )?;
        let device = device.ok_or_else(windows::core::Error::empty)?;
        let context = context.ok_or_else(windows::core::Error::empty)?;
        let dxgi: IDXGIDevice = device.cast()?;
        let factory: IDXGIFactory2 = CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0))?;
        let desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: width,
            Height: height,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            Scaling: DXGI_SCALING_STRETCH,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
            ..Default::default()
        };
        let swapchain = factory.CreateSwapChainForComposition(&device, &raw const desc, None)?;
        let dcomp: IDCompositionDevice = DCompositionCreateDevice(&dxgi)?;
        let target = dcomp.CreateTargetForHwnd(hwnd, true)?;
        let visual = dcomp.CreateVisual()?;
        visual.SetContent(&swapchain)?;
        target.SetRoot(&visual)?;
        dcomp.Commit()?;
        Ok(Gpu {
            device,
            context,
            swapchain,
            _dcomp: dcomp,
            _target: target,
            _visual: visual,
            size: (width, height),
        })
    }
}

fn client_rect(game: HWND) -> Option<(i32, i32, u32, u32)> {
    unsafe {
        if !IsWindow(Some(game)).as_bool() || IsIconic(game).as_bool() {
            return None;
        }
        let mut rect = RECT::default();
        GetClientRect(game, &raw mut rect).ok()?;
        let mut origin = POINT::default();
        if !ClientToScreen(game, &raw mut origin).as_bool() {
            return None;
        }
        let w = u32::try_from(rect.right - rect.left).ok()?;
        let h = u32::try_from(rect.bottom - rect.top).ok()?;
        (w > 0 && h > 0).then_some((origin.x, origin.y, w, h))
    }
}

fn keep_above(hwnd: HWND, game: HWND) {
    unsafe {
        let mut prev = GetWindow(game, GW_HWNDPREV).ok();
        if prev == Some(hwnd) {
            return;
        }
        let mut seen = prev;
        while let Some(w) = seen {
            if w == hwnd {
                return;
            }
            seen = GetWindow(w, GW_HWNDPREV).ok();
        }
        if prev.is_none() {
            let topmost = GetWindowLongW(game, GWL_EXSTYLE).cast_unsigned() & WS_EX_TOPMOST.0 != 0;
            prev = Some(if topmost { HWND_TOPMOST } else { HWND_TOP });
        }
        let _ = SetWindowPos(
            hwnd,
            prev,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

fn pump() {
    let mut msg = MSG::default();
    unsafe {
        while PeekMessageW(&raw mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&raw const msg);
            DispatchMessageW(&raw const msg);
        }
    }
}

#[allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]
fn run() -> Result<(), String> {
    let game = loop {
        if let Some(h) = find_game() {
            break h;
        }
        std::thread::sleep(Duration::from_millis(250));
    };
    GAME.store(game.0 as isize, Ordering::Relaxed);
    unsafe { input::install(game) };
    let instance = unsafe { GetModuleHandleW(None) }.map_err(|e| e.to_string())?;
    let class = WNDCLASSEXW {
        cbSize: u32::try_from(size_of::<WNDCLASSEXW>()).unwrap_or(0),
        lpfnWndProc: Some(window_proc),
        hInstance: instance.into(),
        lpszClassName: w!("FerriteMenu"),
        ..Default::default()
    };
    unsafe { RegisterClassExW(&raw const class) };
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            w!("FerriteMenu"),
            w!("Ferrite"),
            WS_POPUP,
            0,
            0,
            16,
            16,
            Some(game),
            None,
            Some(instance.into()),
            None,
        )
    }
    .map_err(|e| e.to_string())?;
    let mut gpu_state: Option<Gpu> = None;
    let mut renderer: Option<egui_directx11::Renderer> = None;
    let ctx = egui::Context::default();
    ui::style(&ctx);
    let start = Instant::now();
    let mut shown = false;
    let mut placed = (0, 0, 0, 0);
    let mut announced = false;
    loop {
        pump();
        let rect = client_rect(game);
        let want = VISIBLE.load(Ordering::Relaxed) && !super::d3d11_active() && rect.is_some();
        if !want {
            if shown {
                unsafe {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                }
                shown = false;
            }
            if !unsafe { IsWindow(Some(game)) }.as_bool() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(30));
            continue;
        }
        let Some((x, y, w, h)) = rect else { continue };
        if !announced {
            announced = true;
            info!("no Direct3D 11 frames from the game, drawing the menu in an overlay window");
        }
        if placed != (x, y, w, h) {
            placed = (x, y, w, h);
            unsafe {
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    x,
                    y,
                    i32::try_from(w).unwrap_or(1),
                    i32::try_from(h).unwrap_or(1),
                    SWP_NOACTIVATE,
                );
            }
        }
        if gpu_state.is_none() {
            let g = gpu(hwnd, w, h).map_err(|e| format!("Direct3D 11 / DirectComposition: {e}"))?;
            renderer = Some(egui_directx11::Renderer::new(&g.device).map_err(|e| e.to_string())?);
            gpu_state = Some(g);
        }
        let (Some(g), Some(r)) = (gpu_state.as_mut(), renderer.as_mut()) else {
            continue;
        };
        if g.size != (w, h) {
            unsafe {
                g.swapchain
                    .ResizeBuffers(0, w, h, DXGI_FORMAT_UNKNOWN, DXGI_SWAP_CHAIN_FLAG(0))
                    .map_err(|e| e.to_string())?;
            }
            g.size = (w, h);
        }
        if !shown {
            unsafe {
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }
            shown = true;
        }
        keep_above(hwnd, game);
        let Ok(back) = (unsafe { g.swapchain.GetBuffer::<ID3D11Texture2D>(0) }) else {
            continue;
        };
        let mut rtv: Option<ID3D11RenderTargetView> = None;
        if unsafe {
            g.device
                .CreateRenderTargetView(&back, None, Some(&raw mut rtv))
        }
        .is_err()
        {
            continue;
        }
        let Some(rtv) = rtv else { continue };
        unsafe { g.context.ClearRenderTargetView(&rtv, &[0.0; 4]) };
        let ppp = (h as f32 / 1080.0).clamp(1.0, 2.5);
        ctx.set_pixels_per_point(ppp);
        input::set_pixels_per_point(ppp);
        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(w as f32 / ppp, h as f32 / ppp),
            )),
            time: Some(start.elapsed().as_secs_f64()),
            events: input::take_events(),
            focused: true,
            ..Default::default()
        };
        let output = ctx.run_ui(raw_input, ui::draw_shared);
        WANTS_KEYBOARD.store(ctx.egui_wants_keyboard_input(), Ordering::Relaxed);
        let (out, _, _) = egui_directx11::split_output(output);
        let _ = r.render(&g.context, &rtv, &ctx, out);
        unsafe {
            super::OWN_PRESENT.with(|f| f.set(true));
            let _ = g.swapchain.Present(1, DXGI_PRESENT(0));
            super::OWN_PRESENT.with(|f| f.set(false));
        }
    }
}
