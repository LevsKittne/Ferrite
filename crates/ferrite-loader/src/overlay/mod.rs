mod backup;
mod hook;
mod input;
mod ui;
mod window;

use crate::log::{info, warning};
use core::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::Instant;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_RENDER_TARGET_VIEW_DESC, D3D11_RENDER_TARGET_VIEW_DESC_0, D3D11_RTV_DIMENSION_TEXTURE2D,
    D3D11_TEX2D_RTV, D3D11_TEXTURE2D_DESC, ID3D11Device, ID3D11RenderTargetView, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT, DXGI_FORMAT_B8G8R8A8_TYPELESS, DXGI_FORMAT_B8G8R8A8_UNORM,
    DXGI_FORMAT_B8G8R8A8_UNORM_SRGB, DXGI_FORMAT_R8G8B8A8_TYPELESS, DXGI_FORMAT_R8G8B8A8_UNORM,
    DXGI_FORMAT_R8G8B8A8_UNORM_SRGB, DXGI_FORMAT_R10G10B10A2_TYPELESS,
    DXGI_FORMAT_R10G10B10A2_UNORM,
};
use windows::Win32::Graphics::Dxgi::IDXGISwapChain;
use windows::core::Interface;

pub static VISIBLE: AtomicBool = AtomicBool::new(false);
pub static WANTS_KEYBOARD: AtomicBool = AtomicBool::new(false);
static ATTACHED: AtomicBool = AtomicBool::new(false);
static LAST_D3D11: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static EPOCH: OnceLock<Instant> = OnceLock::new();

fn now_ms() -> u64 {
    u64::try_from(EPOCH.get_or_init(Instant::now).elapsed().as_millis()).unwrap_or(u64::MAX)
}

pub(super) fn d3d11_active() -> bool {
    let last = LAST_D3D11.load(Ordering::Relaxed);
    last != 0 && now_ms().saturating_sub(last) < 500
}

#[derive(Debug, Clone)]
pub struct ModRow {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub kind: &'static str,
    pub faulted: bool,
}

#[derive(Debug)]
struct Info {
    root: PathBuf,
    runtime: &'static str,
    game: String,
}

static INFO: OnceLock<Info> = OnceLock::new();
static MODS: Mutex<Vec<ModRow>> = Mutex::new(Vec::new());

struct State {
    ctx: egui::Context,
    renderer: egui_directx11::Renderer,
    start: Instant,
}

unsafe impl Send for State {}

static STATE: Mutex<Option<State>> = Mutex::new(None);

pub fn init(root: PathBuf, runtime: &'static str, menu_key: &str) {
    input::set_menu_key(menu_key);
    let _ = INFO.set(Info {
        root,
        runtime,
        game: crate::game_name(),
    });
    now_ms();
    if let Err(e) = hook::install() {
        warning!("Direct3D 11 menu hook unavailable, using the overlay window: {e}");
    }
    window::start();
    info!("menu ready, press {} in game", input::menu_key_name());
}

pub fn is_open() -> bool {
    VISIBLE.load(Ordering::Relaxed)
}

pub fn toggle() {
    VISIBLE.fetch_xor(true, Ordering::SeqCst);
}

pub fn set_mods(kind: &'static str, rows: Vec<ModRow>) {
    let mut mods = MODS.lock().unwrap_or_else(PoisonError::into_inner);
    mods.retain(|m| m.kind != kind);
    mods.extend(rows);
}

fn mods() -> Vec<ModRow> {
    MODS.lock().unwrap_or_else(PoisonError::into_inner).clone()
}

fn view_format(f: DXGI_FORMAT) -> Option<DXGI_FORMAT> {
    match f {
        DXGI_FORMAT_R8G8B8A8_TYPELESS | DXGI_FORMAT_R8G8B8A8_UNORM_SRGB => {
            Some(DXGI_FORMAT_R8G8B8A8_UNORM)
        }
        DXGI_FORMAT_B8G8R8A8_TYPELESS | DXGI_FORMAT_B8G8R8A8_UNORM_SRGB => {
            Some(DXGI_FORMAT_B8G8R8A8_UNORM)
        }
        DXGI_FORMAT_R10G10B10A2_TYPELESS => Some(DXGI_FORMAT_R10G10B10A2_UNORM),
        _ => None,
    }
}

thread_local! {
    pub(super) static OWN_PRESENT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[allow(clippy::cast_precision_loss)]
unsafe fn frame(raw: *mut c_void) {
    if OWN_PRESENT.with(std::cell::Cell::get) {
        return;
    }
    let Some(sc) = (unsafe { IDXGISwapChain::from_raw_borrowed(&raw) }) else {
        return;
    };
    let Ok(device) = (unsafe { sc.GetDevice::<ID3D11Device>() }) else {
        return;
    };
    LAST_D3D11.store(now_ms().max(1), Ordering::Relaxed);
    let mut guard = STATE.lock().unwrap_or_else(PoisonError::into_inner);
    if guard.is_none() && !VISIBLE.load(Ordering::Relaxed) && ATTACHED.load(Ordering::Relaxed) {
        return;
    }
    let Ok(context) = (unsafe { device.GetImmediateContext() }) else {
        return;
    };
    let Ok(desc) = (unsafe { sc.GetDesc() }) else {
        return;
    };
    if !ATTACHED.swap(true, Ordering::SeqCst) {
        unsafe { input::install(desc.OutputWindow) };
        info!("menu drawn by the Direct3D 11 hook");
    }
    if !VISIBLE.load(Ordering::Relaxed) {
        return;
    }
    let Ok(back) = (unsafe { sc.GetBuffer::<ID3D11Texture2D>(0) }) else {
        return;
    };
    let mut tex = D3D11_TEXTURE2D_DESC::default();
    unsafe { back.GetDesc(&raw mut tex) };
    let mut rtv: Option<ID3D11RenderTargetView> = None;
    let made = match view_format(tex.Format) {
        Some(f) => {
            let vd = D3D11_RENDER_TARGET_VIEW_DESC {
                Format: f,
                ViewDimension: D3D11_RTV_DIMENSION_TEXTURE2D,
                Anonymous: D3D11_RENDER_TARGET_VIEW_DESC_0 {
                    Texture2D: D3D11_TEX2D_RTV { MipSlice: 0 },
                },
            };
            unsafe { device.CreateRenderTargetView(&back, Some(&raw const vd), Some(&raw mut rtv)) }
        }
        None => unsafe { device.CreateRenderTargetView(&back, None, Some(&raw mut rtv)) },
    };
    let (Ok(()), Some(rtv)) = (made, rtv) else {
        return;
    };
    if guard.is_none() {
        let Ok(renderer) = egui_directx11::Renderer::new(&device) else {
            return;
        };
        let ctx = egui::Context::default();
        ui::style(&ctx);
        *guard = Some(State {
            ctx,
            renderer,
            start: Instant::now(),
        });
    }
    let Some(st) = guard.as_mut() else { return };
    let ppp = (tex.Height as f32 / 1080.0).clamp(1.0, 2.5);
    st.ctx.set_pixels_per_point(ppp);
    input::set_pixels_per_point(ppp);
    let raw_input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(tex.Width as f32 / ppp, tex.Height as f32 / ppp),
        )),
        time: Some(st.start.elapsed().as_secs_f64()),
        events: input::take_events(),
        focused: true,
        ..Default::default()
    };
    let output = st.ctx.run_ui(raw_input, ui::draw_shared);
    WANTS_KEYBOARD.store(st.ctx.egui_wants_keyboard_input(), Ordering::Relaxed);
    let (renderer_output, _, _) = egui_directx11::split_output(output);
    let saved = unsafe { backup::Backup::capture(&context) };
    let _ = st.renderer.render(&context, &rtv, &st.ctx, renderer_output);
    unsafe { saved.restore(&context) };
}
