# Ferrite

Lightweight mod loader for **Unity games**, Mono and IL2CPP, with a Rust core.

- Works with any Unity game, no per-game setup. Mono tested on Unity 2020.3, 2022.3 and 6000.3, IL2CPP on 2022.3.
- Rust does the heavy lifting: bootstrap, mod discovery, dependency order, configs, logging, console.
- Mono games: mods are plain C# libraries with Harmony.
- IL2CPP games: mods are Rust libraries built with `ferrite-sdk` (game classes, method calls, inline hooks).
- Console window with commands you can type while the game runs.
- In-game menu written in Rust (egui): mods, live settings, console, loader options. F1 by default. Works with any renderer: drawn inside Direct3D 11 games, in a transparent overlay window for Direct3D 12, Vulkan and OpenGL.
- Mods start before the first scene, configs and mods reload while the game runs.
- One-click installer in 12 languages.

## Install

1. Download `Ferrite-Setup.exe` from [Releases](https://github.com/LevsKittne/Ferrite/releases).
2. Put it next to the game's `.exe` and run it, or run it anywhere and pick the game folder.
3. Put mods into `Ferrite\mods`.

Silent install: `Ferrite-Setup.exe /silent "/dir=C:\path\to\game"`.

Uninstall: delete `version.dll` and the `Ferrite` folder from the game folder. Start the game with `--no-mods` to skip mods once.

```
version.dll           native loader (Rust)
Ferrite\
  core\               Ferrite.dll (C# API for Mono games), Harmony, MonoMod, Mono.Cecil
  mods\               mods (*.dll, subfolders allowed)
  patchers\           Cecil patchers (Mono)
  config\             per-mod configs (<mod id>.toml)
  cache\              patched assemblies, shadow copies of native mods
  dump\               output of dump and dump_assemblies
  ferrite.toml        loader settings
  log.txt
```

## ferrite.toml

```toml
[loader]
enabled = true
console = true
unity_log = true
entrypoint = "early"      # "early": before the first scene, "scene": on it
menu = true
menu_key = "F1"

[log]
console_level = "info"
file_level = "debug"
keep_previous = true      # previous log goes to log.prev.txt

[mods]
disabled = ["some-mod-id"]
hot_reload = false        # reload mods when their dll changes

[patchers]
dump_assemblies = false

[debug]
enabled = false           # Mono debugger for dnSpy / Rider / VS
address = "127.0.0.1:55555"
suspend = false
```

Every option can also be changed in the menu (Loader tab).

## Console and menu

Built-in commands: `help`, `mods`, `config [mod] [key] [value]`, `reload <mod id>`, `loglevel <level>`, `menu`, and `dump [filter]` in IL2CPP games (all classes, fields with offsets and methods with RVAs into `Ferritedumpil2cpp.cs`).

Configs are TOML with comments. Edit a file while the game runs and the mod gets `OnConfigChanged` / `config_changed`; the menu and the `config` command do the same.

## Dependencies

`Dependencies` entries: `id`, `id?` (optional), `id>=1.2` (minimum version), `!id` (incompatible), `@Game` (only for this exe). C# mods can also use `Incompatibilities` and `Processes`.

## Writing a mod

Create a `net472` class library that references `Ferrite.dll` and `0Harmony.dll`. See `examples/HelloMod`.

```csharp
using Ferrite;

[Mod("hello", "Hello Mod", "1.0.0", Author = "you", Dependencies = new[] { "other-mod", "optional-mod?" })]
public sealed class HelloMod : Mod {
    public override void OnLoad() {
        var greeting = Config.Get("greeting", "Hello!");
        Commands.Register("hello", "Prints the greeting.", args => Commands.Print(greeting));
        Harmony.PatchAll(typeof(HelloMod).Assembly);
        Logger.Info(greeting);
    }
}
```

- Callbacks: `OnLoad`, `OnSceneLoaded`, `OnUpdate`, `OnLateUpdate`, `OnFixedUpdate`, `OnGUI`, `OnQuit`, `OnConfigChanged`, `OnUnload`. A mod that throws in a callback is disabled, the game keeps running.
- `Config.Get(key, default, description)` / `Config.Set(key, value)` for string, bool, int, long, float, double and enums (`GetEnum`), stored in `Ferrite\config\<id>.toml`. Keys like `ui.scale` become TOML sections, descriptions become comments.
- Mods are loaded from memory, so the dll is never locked: rebuild it and run `reload <id>` (or turn on `hot_reload`). Harmony patches and commands of the old copy are removed.
- `Runtime.Find(id)` / `Runtime.IsLoaded(id)` give access to other mods.
- Patchers: classes derived from `Ferrite.Patcher` in `Ferrite\patchers` patch game assemblies with Mono.Cecil before the game loads them (see `examples/HelloPatcher`). Patchers must not reference game assemblies.
- `Commands.Register(name, description, handler, usage)`, typed in the Ferrite console. Built-in: `help`, `mods`.
- `Harmony` is a per-mod Harmony instance with the mod id.
- Dependencies ending with `?` are optional.

## IL2CPP mods (Rust)

In IL2CPP games the C# code is compiled to native code, so mods are native libraries. Create a `cdylib` crate that depends on `ferrite-sdk`. See `examples/hello-native`.

```rust
use ferrite_sdk::il2cpp::Object;
use ferrite_sdk::{Context, Mod, export_mod};

struct Hello;

impl Mod for Hello {
    fn load(ctx: &Context) -> Self {
        let greeting: String = ctx.config_get("greeting", "Hello!".to_owned());
        ctx.command("hello", "Prints the greeting.", |ctx, _| {
            ctx.info(&ctx.config_get("greeting", "Hello!".to_owned()));
        });
        if let Some(il2cpp) = ctx.il2cpp()
            && let Some(log) = il2cpp.find_method("UnityEngine", "Debug", "Log", 1)
        {
            let mut args = [il2cpp.string(&greeting).0];
            let _ = unsafe { il2cpp.invoke(log, Object(std::ptr::null_mut()), &mut args) };
        }
        Self
    }

    fn update(&mut self, _ctx: &Context) {}
}

export_mod!(Hello, id = "hello", name = "Hello", version = "1.0.0", dependencies = "other-mod, optional-mod?");
```

- Callbacks: `load`, `update` (every frame), `scene_loaded`, `config_changed`, `unload`. Panics are caught, a panicking mod is disabled.
- `ctx.il2cpp()`: find classes, methods and fields, `invoke`, create and read strings, `Method::pointer`, `Method::replace_pointer`.
- `ctx.hook(target, detour)` installs an inline hook and returns the trampoline to the original, `ctx.hook_method(ns, class, method, argc, detour)` does it by name, `ctx.unhook`, `ctx.replace_method`. Prologues are relocated with iced-x86, RIP-relative instructions included. Hooks and pointer patches are tracked per mod and removed on unload.
- With `hot_reload` native mods are loaded from shadow copies in `Ferritecache`, so a rebuilt dll is picked up while the game runs.
- `ctx.il2cpp()` also enumerates images, classes, methods and fields, reads and writes fields, creates objects and arrays, pins objects (`pin`) and resolves internal calls (`icall`).
- `ctx.config(key, default, description)`, `ctx.execute(command)`, `ctx.root_dir()` / `game_dir()` / `mod_dir()` / `config_dir()`.
- ABI 2; mods built for ABI 1 still load.
- `ctx.symbol("il2cpp_...")` gives any IL2CPP export.
- Config, commands and logging work the same as for C# mods.
- Methods the game never uses may be stripped by IL2CPP and not exist at runtime.

## How it works

`version.dll` is a proxy for the Windows `version.dll` that Unity loads at start. It hooks `mono_jit_init_version`, and once the Mono domain exists it opens `Ferrite.dll`, registers internal calls and starts the managed runtime. Before anything else the preloader runs Cecil patchers and patches `UnityEngine.Application`'s static constructor to start mods before the first scene; patched assemblies are loaded into Mono's default context and Unity's own `mono_domain_assembly_open` / `mono_image_open_from_data_with_name` calls are redirected to them. The unpatched entrypoint is cached, so later starts skip Cecil. Then the C# side asks the Rust core for mods: Rust scans `Ferrite\mods`, finds every class derived from `Ferrite.Mod` through the Mono embedding API, reads `[Mod]`, skips disabled and duplicate ids, sorts by dependencies and returns the types in load order. Logging, the console and configs stay on the Rust side and are reached through internal calls.

In IL2CPP games it hooks `il2cpp_init` instead. After the runtime starts it swaps the method pointers of `SceneManager.Internal_SceneLoaded`, `Application.CallLogCallback` and several per-frame methods (`UnitySynchronizationContext.ExecuteTasks`, `BeforeRenderHelper.Invoke`, `Canvas.SendWillRenderCanvases`, deduplicated by `Time.frameCount`), loads native mods from `Ferrite\mods` right after `il2cpp_init` and passes them a stable C ABI table (`ferrite_sdk::abi`).

The menu hooks `IDXGISwapChain::Present` with the same inline hook engine, draws egui with egui-directx11 and subclasses the game window for input. When the game sends no Direct3D 11 frames (other renderers, or the game is paused), the menu is drawn in a borderless window over the game's client area with its own Direct3D 11 device and a DirectComposition swap chain with per-pixel alpha; that window does not show over exclusive fullscreen. Game input is blocked and the cursor unlocked while it is open.

## Building

Requires Rust (stable, MSVC), the .NET SDK and any Unity Mono game for the Unity reference assemblies. `tools\send-command.ps1` sends a console command to a running game without focusing its window.

```powershell
.\build.ps1 -GameDir "C:\...\Some Unity Game"
.\build.ps1 -Install "C:\...\Some Unity Game"
cargo clippy --workspace --all-targets
cargo test --workspace
cargo fmt --check
```

Output: `dist\`, installer in `dist\setup\`.

## License

MIT, see [LICENSE](LICENSE). Bundled libraries in `libs\`: Harmony, MonoMod, Mono.Cecil (MIT).
