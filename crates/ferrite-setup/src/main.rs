#![windows_subsystem = "windows"]

mod i18n;
mod install;

use i18n::{fill, text};
use install::{Game, error, find_game, info, self_delete, write_all};

const TITLE: &str = "Ferrite Setup";

macro_rules! payload {
    ($($relative:literal),* $(,)?) => {
        &[$(($relative, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../dist/", $relative)) as &[u8])),*]
    };
}

const FILES: &[(&str, &[u8])] = payload! {
    "version.dll",
    "Ferrite/core/Ferrite.dll",
    "Ferrite/core/0Harmony.dll",
    "Ferrite/core/Mono.Cecil.dll",
    "Ferrite/core/Mono.Cecil.Mdb.dll",
    "Ferrite/core/Mono.Cecil.Pdb.dll",
    "Ferrite/core/Mono.Cecil.Rocks.dll",
    "Ferrite/core/MonoMod.RuntimeDetour.dll",
    "Ferrite/core/MonoMod.Utils.dll",
};

fn main() {
    let t = text();
    let Some(Game::Mono(game) | Game::Il2Cpp(game)) = find_game(t.pick) else {
        error(TITLE, t.not_found);
        return;
    };
    let update = game
        .join("Ferrite")
        .join("core")
        .join("Ferrite.dll")
        .is_file();
    let result = write_all(&game, FILES).and_then(|()| {
        std::fs::create_dir_all(game.join("Ferrite").join("mods")).map_err(|e| e.to_string())
    });
    if let Err(e) = result {
        error(TITLE, &fill(t.write_failed, &[&e]));
        return;
    }
    let state = if update { t.updated } else { t.installed };
    info(
        TITLE,
        &fill(t.loader_done, &[state, &game.display().to_string()]),
    );
    self_delete();
}
