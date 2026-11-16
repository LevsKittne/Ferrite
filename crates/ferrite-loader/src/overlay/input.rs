use super::VISIBLE;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU16, AtomicU32, Ordering};
use windows::Win32::Foundation::{HGLOBAL, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, VIRTUAL_KEY, VK_A, VK_BACK, VK_C, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END,
    VK_ESCAPE, VK_F1, VK_HOME, VK_LEFT, VK_MENU, VK_RETURN, VK_RIGHT, VK_SHIFT, VK_TAB, VK_UP,
    VK_V, VK_X, VK_Z,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, GWLP_WNDPROC, HTCLIENT, IDC_ARROW, LoadCursorW, SetCursor, SetWindowLongPtrW,
    WM_CHAR, WM_INPUT, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_RBUTTONDBLCLK, WM_RBUTTONDOWN,
    WM_RBUTTONUP, WM_SETCURSOR, WM_SYSKEYDOWN, WM_SYSKEYUP, WNDPROC,
};

const CF_UNICODETEXT: u32 = 13;

static OLD_PROC: AtomicIsize = AtomicIsize::new(0);
static PPP: AtomicU32 = AtomicU32::new(0x3F80_0000);
static MENU_KEY: AtomicU16 = AtomicU16::new(VK_F1.0);
static SKIP_CHAR: AtomicBool = AtomicBool::new(false);
static EVENTS: Mutex<Vec<egui::Event>> = Mutex::new(Vec::new());

pub fn set_pixels_per_point(ppp: f32) {
    PPP.store(ppp.to_bits(), Ordering::Relaxed);
}

pub fn set_menu_key(name: &str) {
    if let Some(vk) = key_code(name) {
        MENU_KEY.store(vk, Ordering::Relaxed);
    }
}

pub fn menu_key_name() -> String {
    let vk = MENU_KEY.load(Ordering::Relaxed);
    NAMES
        .iter()
        .find(|(_, v)| *v == vk)
        .map_or_else(|| format!("key {vk}"), |(n, _)| (*n).to_owned())
}

pub(super) const NAMES: [(&str, u16); 22] = [
    ("F1", 0x70),
    ("F2", 0x71),
    ("F3", 0x72),
    ("F4", 0x73),
    ("F5", 0x74),
    ("F6", 0x75),
    ("F7", 0x76),
    ("F8", 0x77),
    ("F9", 0x78),
    ("F10", 0x79),
    ("F11", 0x7A),
    ("F12", 0x7B),
    ("Insert", 0x2D),
    ("Delete", 0x2E),
    ("Home", 0x24),
    ("End", 0x23),
    ("PageUp", 0x21),
    ("PageDown", 0x22),
    ("Pause", 0x13),
    ("ScrollLock", 0x91),
    ("BackQuote", 0xC0),
    ("RightShift", 0xA1),
];

fn key_code(name: &str) -> Option<u16> {
    NAMES
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name.trim()))
        .map(|(_, v)| *v)
}

pub unsafe fn install(hwnd: HWND) {
    if OLD_PROC.load(Ordering::SeqCst) != 0 {
        return;
    }
    let old = unsafe { SetWindowLongPtrW(hwnd, GWLP_WNDPROC, wndproc as *const () as isize) };
    OLD_PROC.store(old, Ordering::SeqCst);
}

pub fn take_events() -> Vec<egui::Event> {
    std::mem::take(
        &mut *EVENTS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    )
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let swallow = std::panic::catch_unwind(|| handle(msg, wparam.0, lparam.0)).unwrap_or(false);
    if swallow {
        return LRESULT(0);
    }
    let old: WNDPROC =
        unsafe { std::mem::transmute::<isize, WNDPROC>(OLD_PROC.load(Ordering::SeqCst)) };
    unsafe { CallWindowProcW(old, hwnd, msg, wparam, lparam) }
}

fn push(e: egui::Event) {
    EVENTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(e);
}

fn down(vk: VIRTUAL_KEY) -> bool {
    let state = unsafe { GetKeyState(i32::from(vk.0)) };
    state < 0
}

fn modifiers() -> egui::Modifiers {
    let ctrl = down(VK_CONTROL);
    egui::Modifiers {
        alt: down(VK_MENU),
        ctrl,
        shift: down(VK_SHIFT),
        mac_cmd: false,
        command: ctrl,
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
fn pos(lparam: isize) -> egui::Pos2 {
    let ppp = f32::from_bits(PPP.load(Ordering::Relaxed));
    let x = f32::from((lparam & 0xFFFF) as u16 as i16);
    let y = f32::from(((lparam >> 16) & 0xFFFF) as u16 as i16);
    egui::pos2(x / ppp, y / ppp)
}

fn key(vk: u16) -> Option<egui::Key> {
    use egui::Key;
    Some(match VIRTUAL_KEY(vk) {
        VK_RETURN => Key::Enter,
        VK_ESCAPE => Key::Escape,
        VK_BACK => Key::Backspace,
        VK_TAB => Key::Tab,
        VK_DELETE => Key::Delete,
        VK_LEFT => Key::ArrowLeft,
        VK_RIGHT => Key::ArrowRight,
        VK_UP => Key::ArrowUp,
        VK_DOWN => Key::ArrowDown,
        VK_HOME => Key::Home,
        VK_END => Key::End,
        VK_A => Key::A,
        VK_C => Key::C,
        VK_V => Key::V,
        VK_X => Key::X,
        VK_Z => Key::Z,
        _ => return None,
    })
}

fn clipboard() -> Option<String> {
    unsafe {
        OpenClipboard(None).ok()?;
        let text = GetClipboardData(CF_UNICODETEXT).ok().and_then(|h| {
            let mem = HGLOBAL(h.0);
            let p = GlobalLock(mem).cast::<u16>().cast_const();
            if p.is_null() {
                return None;
            }
            let mut len = 0;
            while *p.add(len) != 0 {
                len += 1;
            }
            let s = String::from_utf16_lossy(std::slice::from_raw_parts(p, len));
            let _ = GlobalUnlock(mem);
            Some(s)
        });
        let _ = CloseClipboard();
        text
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
pub(super) fn handle(msg: u32, wparam: usize, lparam: isize) -> bool {
    if msg == WM_CHAR && SKIP_CHAR.swap(false, Ordering::SeqCst) {
        return true;
    }
    let vk = wparam as u16;
    if (msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN)
        && vk == MENU_KEY.load(Ordering::Relaxed)
        && (lparam >> 30) & 1 == 0
    {
        VISIBLE.fetch_xor(true, Ordering::SeqCst);
        SKIP_CHAR.store(true, Ordering::SeqCst);
        return true;
    }
    if !VISIBLE.load(Ordering::Relaxed) {
        return false;
    }
    let button = |b: egui::PointerButton, pressed: bool| {
        push(egui::Event::PointerButton {
            pos: pos(lparam),
            button: b,
            pressed,
            modifiers: modifiers(),
        });
        true
    };
    match msg {
        WM_SETCURSOR if (lparam & 0xFFFF) as u32 == HTCLIENT => {
            if let Ok(arrow) = unsafe { LoadCursorW(None, IDC_ARROW) } {
                unsafe { SetCursor(Some(arrow)) };
            }
            true
        }
        WM_MOUSEMOVE => {
            push(egui::Event::PointerMoved(pos(lparam)));
            true
        }
        WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => button(egui::PointerButton::Primary, true),
        WM_LBUTTONUP => button(egui::PointerButton::Primary, false),
        WM_RBUTTONDOWN | WM_RBUTTONDBLCLK => button(egui::PointerButton::Secondary, true),
        WM_RBUTTONUP => button(egui::PointerButton::Secondary, false),
        WM_MBUTTONDOWN => button(egui::PointerButton::Middle, true),
        WM_MBUTTONUP => button(egui::PointerButton::Middle, false),
        WM_MOUSEWHEEL => {
            let delta = f32::from((wparam >> 16) as u16 as i16) / 120.0;
            push(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, delta),
                phase: egui::TouchPhase::Move,
                modifiers: modifiers(),
            });
            true
        }
        WM_CHAR => {
            if let Some(c) = char::from_u32(wparam as u32).filter(|c| !c.is_control()) {
                push(egui::Event::Text(c.to_string()));
            }
            true
        }
        WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP => {
            let pressed = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
            let m = modifiers();
            if pressed && m.ctrl && vk == VK_V.0 {
                if let Some(text) = clipboard() {
                    push(egui::Event::Paste(text));
                }
                return true;
            }
            if let Some(k) = key(vk) {
                push(egui::Event::Key {
                    key: k,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: m,
                });
                if pressed
                    && k == egui::Key::Escape
                    && !super::WANTS_KEYBOARD.load(Ordering::Relaxed)
                {
                    VISIBLE.store(false, Ordering::SeqCst);
                }
            }
            true
        }
        WM_INPUT => true,
        _ => false,
    }
}
