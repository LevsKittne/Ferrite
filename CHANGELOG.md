# Changelog

## [0.2.1] - 2026-11-16

### Added
- The menu works with any renderer. Direct3D 11 games get it drawn inside the game; Direct3D 12, Vulkan and OpenGL games (and a paused game) get it in a transparent window over the game.

### Changed
- The menu window keeps its size and is resized only by hand.
- Rounder corners on the menu window and widgets.

## [0.2.0] - 2026-11-15

### Added
- In-game menu written in Rust (egui): mods with their settings, console with log and command input, loader options. F1 by default, `menu` command.
- Mono: Cecil patchers in `Ferrite\patchers` that patch game assemblies before the game loads them (`examples/HelloPatcher`).
- Mono: mods start before the first scene through a patched `UnityEngine.Application` static constructor. The patch is cached, later starts take about 3 ms.
- IL2CPP: native mods load right after `il2cpp_init`.
- Hot reload: C# mods are loaded from memory so the dll is never locked; native mods are loaded from shadow copies. `reload <id>` command and automatic reload with `hot_reload = true`.
- Mod unload callbacks: `OnUnload` (C#), `unload` (Rust). Harmony patches, commands, inline hooks and method pointer patches of a mod are removed by Ferrite on unload.
- Live configs: edit `config\<id>.toml` while the game runs and the mod gets `OnConfigChanged` / `config_changed`. Sections (`ui.scale`), descriptions written as comments, enums, int and float values.
- Console commands `config`, `loglevel`, `menu`, `reload`, and `dump` in IL2CPP games (all classes, field offsets and method RVAs).
- Dependency rules: minimum version `id>=1.2`, incompatibility `!id`, game filter `@Game`.
- Unity log capture in IL2CPP games.
- Log levels for console and file, previous log kept as `log.prev.txt`.
- Mono debugger option for dnSpy, Rider and Visual Studio.
- Rust SDK, ABI 2: `hook_method`, `unhook`, `replace_method`, `execute`, directory helpers, `config` with description, IL2CPP enumeration of images, classes, methods and fields, field read and write, objects, arrays, GC handles, internal calls. Mods built for ABI 1 still load.
- `Runtime.Find` and `Runtime.IsLoaded` for C# mods that use other mods.

### Fixed
- IL2CPP on Unity 6: per-frame callbacks ran only when Unity had queued tasks; Ferrite now uses several per-frame methods and runs once per frame.

## [0.1.0] - 2026-11-15

### Added
- Mod loader for Unity Mono games: C# mods with Harmony, dependency order, per-mod configs, console window with commands.
- IL2CPP support: native Rust mods built with `ferrite-sdk`, inline hooks on iced-x86.
- One-click installer in 12 languages.
