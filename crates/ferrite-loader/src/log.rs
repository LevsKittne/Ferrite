use crate::sys;
use std::collections::VecDeque;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Console::{
    AllocConsole, CONSOLE_MODE, ENABLE_EXTENDED_FLAGS, ENABLE_QUICK_EDIT_MODE, GetConsoleMode,
    ReadConsoleW, SetConsoleMode, SetConsoleOutputCP, SetConsoleTextAttribute, SetConsoleTitleW,
    WriteConsoleW,
};
use windows_sys::Win32::System::SystemInformation::GetLocalTime;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Debug,
    Info,
    Warning,
    Error,
}

impl Level {
    pub fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Debug,
            2 => Self::Warning,
            3 => Self::Error,
            _ => Self::Info,
        }
    }

    fn color(self) -> u16 {
        match self {
            Self::Debug => 0x08,
            Self::Info => 0x07,
            Self::Warning => 0x0E,
            Self::Error => 0x0C,
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(match self {
            Self::Debug => "Debug",
            Self::Info => "Info",
            Self::Warning => "Warning",
            Self::Error => "Error",
        })
    }
}

#[derive(Debug, Default)]
struct Sink {
    file: Option<File>,
    file_level: Option<Level>,
    console_level: Option<Level>,
    console: Option<Console>,
    backlog: Option<Vec<(Level, String, String)>>,
}

#[derive(Debug, Clone, Copy)]
struct Console(HANDLE);

unsafe impl Send for Console {}

static SINK: Mutex<Option<Sink>> = Mutex::new(None);
static INPUT: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());
static RECENT: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());
static SEQUENCE: AtomicI64 = AtomicI64::new(0);
const RECENT_LINES: usize = 400;

pub fn sequence() -> i64 {
    SEQUENCE.load(Ordering::Relaxed)
}

pub fn recent() -> String {
    let lines = lock(&RECENT);
    let mut out = String::new();
    for line in lines.iter() {
        out.push_str(line);
        out.push('\n');
    }
    out
}

pub fn set_console_level(level: Level) {
    if let Some(sink) = lock(&SINK).as_mut() {
        sink.console_level = Some(level);
    }
}

pub fn push_command(line: String) {
    lock(&INPUT).push_back(line);
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn init(path: &Path, buffer_for_console: bool, levels: &crate::config::LogSection) {
    if levels.keep_previous && path.exists() {
        let _ = std::fs::rename(path, path.with_extension("prev.txt"));
    }
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .ok();
    *lock(&SINK) = Some(Sink {
        file,
        file_level: Some(levels.file_level),
        console_level: Some(levels.console_level),
        console: None,
        backlog: buffer_for_console.then(Vec::new),
    });
}

pub fn open_console(title: &str) {
    unsafe {
        AllocConsole();
        SetConsoleOutputCP(65001);
        SetConsoleTitleW(sys::wide(title).as_ptr());
    }
    let input = open("CONIN$");
    if let Some(input) = input {
        let mut mode: CONSOLE_MODE = 0;
        unsafe {
            GetConsoleMode(input.0, &raw mut mode);
            SetConsoleMode(
                input.0,
                (mode & !ENABLE_QUICK_EDIT_MODE) | ENABLE_EXTENDED_FLAGS,
            );
        }
        std::thread::Builder::new()
            .name("ferrite-console".into())
            .spawn(move || read_lines(input))
            .ok();
    }
    let mut guard = lock(&SINK);
    let Some(sink) = guard.as_mut() else { return };
    sink.console = open("CONOUT$");
    let backlog = sink.backlog.take().unwrap_or_default();
    if let Some(console) = sink.console {
        for (level, source, message) in backlog {
            print(console, level, &source, &message);
        }
    }
}

fn open(name: &str) -> Option<Console> {
    let handle = unsafe {
        CreateFileW(
            sys::wide(name).as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        )
    };
    (handle != INVALID_HANDLE_VALUE).then_some(Console(handle))
}

fn read_lines(input: Console) {
    let mut buf = [0u16; 512];
    let mut line = Vec::new();
    loop {
        let mut read = 0u32;
        let ok = unsafe {
            ReadConsoleW(
                input.0,
                buf.as_mut_ptr().cast(),
                512,
                &raw mut read,
                std::ptr::null(),
            )
        };
        if ok == 0 || read == 0 {
            return;
        }
        for &unit in &buf[..read as usize] {
            match unit {
                10 => {
                    let text = String::from_utf16_lossy(&line).trim().to_owned();
                    line.clear();
                    if !text.is_empty() {
                        lock(&INPUT).push_back(text);
                    }
                }
                13 => {}
                _ => line.push(unit),
            }
        }
    }
}

pub fn next_command() -> Option<String> {
    lock(&INPUT).pop_front()
}

pub fn write(level: Level, source: &str, message: &str) {
    let mut guard = lock(&SINK);
    let Some(sink) = guard.as_mut() else { return };
    let passes = |min: Option<Level>| min.is_none_or(|m| level >= m);
    let (to_file, to_console) = (passes(sink.file_level), passes(sink.console_level));
    if let Some(file) = sink.file.as_mut().filter(|_| to_file) {
        let _ = write!(file, "[{}] [{source}] [{level}] {message}\r\n", timestamp());
    }
    if !to_console {
        return;
    }
    {
        let mut recent = lock(&RECENT);
        for line in message.lines() {
            if recent.len() >= RECENT_LINES {
                recent.pop_front();
            }
            recent.push_back(format!("[{level}] [{source}] {line}"));
        }
        SEQUENCE.fetch_add(1, Ordering::Relaxed);
    }
    if let Some(console) = sink.console {
        print(console, level, source, message);
    } else if let Some(backlog) = sink.backlog.as_mut() {
        backlog.push((level, source.to_owned(), message.to_owned()));
    }
}

fn timestamp() -> String {
    let t = unsafe {
        let mut t = std::mem::zeroed();
        GetLocalTime(&raw mut t);
        t
    };
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    )
}

fn print(console: Console, level: Level, source: &str, message: &str) {
    let text = format!(
        "[{level:<7}:{source:>12}] {}\r\n",
        message.replace("\r\n", "\n").replace('\n', "\r\n")
    );
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut written = 0u32;
    unsafe {
        SetConsoleTextAttribute(console.0, level.color());
        WriteConsoleW(
            console.0,
            units.as_ptr().cast(),
            u32::try_from(units.len()).unwrap_or(u32::MAX),
            &raw mut written,
            std::ptr::null(),
        );
        SetConsoleTextAttribute(console.0, Level::Info.color());
    }
}

pub fn native(level: Level, args: fmt::Arguments<'_>) {
    write(level, "Ferrite", &args.to_string());
}

macro_rules! debug {
    ($($t:tt)*) => { $crate::log::native($crate::log::Level::Debug, format_args!($($t)*)) };
}

macro_rules! info {
    ($($t:tt)*) => { $crate::log::native($crate::log::Level::Info, format_args!($($t)*)) };
}

macro_rules! warning {
    ($($t:tt)*) => { $crate::log::native($crate::log::Level::Warning, format_args!($($t)*)) };
}

macro_rules! error {
    ($($t:tt)*) => { $crate::log::native($crate::log::Level::Error, format_args!($($t)*)) };
}

pub(crate) use {debug, error, info, warning};
