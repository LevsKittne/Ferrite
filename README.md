# Ferrite

Lightweight mod loader for **Unity games**, Mono and IL2CPP, with a Rust core.

- Works with any Unity game, no per-game setup. Mono tested on Unity 2020.3, 2022.3 and 6000.3, IL2CPP on 2022.3.
- Rust does the heavy lifting: bootstrap, mod discovery, dependency order, configs, logging, console.
- Mono games: mods are plain C# libraries with Harmony.
- IL2CPP games: mods are Rust libraries built with `ferrite-sdk` (game classes, method calls, inline hooks).
- Console window with commands you can type while the game runs.
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
  config\             per-mod configs (<mod id>.toml)
  ferrite.toml        loader settings
  log.txt
```

## ferrite.toml

```toml
[loader]
enabled = true
console = true
unity_log = true

[mods]
disabled = ["some-mod-id"]
```

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

- Callbacks: `OnLoad`, `OnSceneLoaded`, `OnUpdate`, `OnLateUpdate`, `OnFixedUpdate`, `OnGUI`, `OnQuit`. A mod that throws in a callback is disabled, the game keeps running.
- `Config.Get(key, default)` / `Config.Set(key, value)` for string, bool, long and double, stored in `Ferrite\config\<id>.toml`.
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

- Callbacks: `load`, `update` (every frame), `scene_loaded`. Panics are caught, a panicking mod is disabled.
- `ctx.il2cpp()`: find classes, methods and fields, `invoke`, create and read strings, `Method::pointer`, `Method::replace_pointer`.
- `ctx.hook(target, detour)` installs an inline hook and returns the trampoline to the original. Prologues are relocated with iced-x86, RIP-relative instructions included.
- `ctx.symbol("il2cpp_...")` gives any IL2CPP export.
- Config, commands and logging work the same as for C# mods.
- Methods the game never uses may be stripped by IL2CPP and not exist at runtime.

## How it works

`version.dll` is a proxy for the Windows `version.dll` that Unity loads at start. It hooks `mono_jit_init_version`, and once the Mono domain exists it opens `Ferrite.dll`, registers internal calls and starts the managed runtime. On the first scene the C# side asks the Rust core for mods: Rust scans `Ferrite\mods`, finds every class derived from `Ferrite.Mod` through the Mono embedding API, reads `[Mod]`, skips disabled and duplicate ids, sorts by dependencies and returns the types in load order. Logging, the console and configs stay on the Rust side and are reached through internal calls.

In IL2CPP games it hooks `il2cpp_init` instead. After the runtime starts it swaps the method pointers of `SceneManager.Internal_SceneLoaded` and `UnitySynchronizationContext.ExecuteTasks` (both called from native code every scene and every frame), loads native mods from `Ferrite\mods` on the first scene and passes them a stable C ABI table (`ferrite_sdk::abi`).

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
