use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use windows_sys::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoTaskMemFree};
use windows_sys::Win32::UI::Shell::{
    BIF_NEWDIALOGSTYLE, BIF_RETURNONLYFSDIRS, BROWSEINFOW, SHBrowseForFolderW, SHGetPathFromIDListW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MESSAGEBOX_STYLE, MessageBoxW,
};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug)]
pub enum Game {
    Mono(PathBuf),
    Il2Cpp(PathBuf),
}

fn wide(s: impl AsRef<OsStr>) -> Vec<u16> {
    s.as_ref().encode_wide().chain(Some(0)).collect()
}

fn arg_value(name: &str) -> Option<String> {
    let prefix = format!("/{name}=");
    std::env::args().find_map(|a| {
        a.get(..prefix.len())
            .filter(|p| p.eq_ignore_ascii_case(&prefix))
            .map(|_| a[prefix.len()..].trim_matches('"').to_owned())
    })
}

pub fn silent() -> bool {
    std::env::args().any(|a| a.eq_ignore_ascii_case("/silent"))
}

fn message(title: &str, text: &str, style: MESSAGEBOX_STYLE) {
    if silent() {
        if style & MB_ICONERROR != 0 {
            std::process::exit(1);
        }
        return;
    }
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            wide(text).as_ptr(),
            wide(title).as_ptr(),
            style,
        )
    };
}

pub fn info(title: &str, text: &str) {
    message(title, text, MB_OK | MB_ICONINFORMATION);
}

pub fn error(title: &str, text: &str) {
    message(title, text, MB_OK | MB_ICONERROR);
}

pub fn classify(dir: &Path) -> Option<Game> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find_map(|exe| {
            if !exe
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("exe"))
            {
                return None;
            }
            let data = dir.join(format!("{}_Data", exe.file_stem()?.to_string_lossy()));
            if !data.is_dir() {
                return None;
            }
            if dir.join("GameAssembly.dll").is_file() || data.join("il2cpp_data").is_dir() {
                return Some(Game::Il2Cpp(dir.to_path_buf()));
            }
            let managed = data.join("Managed");
            (managed.join("UnityEngine.dll").is_file()
                || managed.join("UnityEngine.CoreModule.dll").is_file())
            .then(|| Game::Mono(dir.to_path_buf()))
        })
}

fn pick_folder(title: &str) -> Option<PathBuf> {
    if silent() {
        return None;
    }
    unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) };
    let title = wide(title);
    let mut name = [0u16; 260];
    let mut info = BROWSEINFOW {
        hwndOwner: std::ptr::null_mut(),
        pidlRoot: std::ptr::null_mut(),
        pszDisplayName: name.as_mut_ptr(),
        lpszTitle: title.as_ptr(),
        ulFlags: BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE,
        lpfn: None,
        lParam: 0,
        iImage: 0,
    };
    let list = unsafe { SHBrowseForFolderW(&raw mut info) };
    if list.is_null() {
        return None;
    }
    let mut path = vec![0u16; 32_768];
    let ok = unsafe { SHGetPathFromIDListW(list, path.as_mut_ptr()) };
    unsafe { CoTaskMemFree(list.cast_const().cast()) };
    if ok == 0 {
        return None;
    }
    let len = path.iter().position(|&c| c == 0).unwrap_or(path.len());
    Some(PathBuf::from(OsString::from_wide(&path[..len])))
}

pub fn find_game(pick_title: &str) -> Option<Game> {
    if let Some(dir) = arg_value("dir") {
        return classify(Path::new(&dir));
    }
    let exe = std::env::current_exe().ok()?;
    exe.ancestors()
        .skip(1)
        .find_map(classify)
        .or_else(|| pick_folder(pick_title).and_then(|d| classify(&d)))
}

pub fn write_all(base: &Path, files: &[(&str, &[u8])]) -> Result<(), String> {
    for (relative, data) in files {
        let path = base.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::write(&path, data).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

pub fn self_delete() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let _ = Command::new("cmd")
        .raw_arg(format!(
            "/c ping 127.0.0.1 -n 3 >nul & del /f /q \"{}\"",
            exe.display()
        ))
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
}
