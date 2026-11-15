# Ferrite

Lightweight mod loader for **Unity games on Mono**, with a Rust core.

- Works with any Unity Mono game, no per-game setup. Tested on Unity 2020.3, 2022.3 and 6000.3.
- Rust does the heavy lifting: bootstrap, mod discovery, dependency order, configs, logging, console.
- Mods are plain C# libraries with Harmony, the same as everywhere else.
- Console window with commands you can type while the game runs.
- One-click installer in 12 languages.

IL2CPP games are not supported.

## Install

1. Download `Ferrite-Setup.exe` from [Releases](https://github.com/LevsKittne/Ferrite/releases).
2. Put it next to the game's `.exe` and run it, or run it anywhere and pick the game folder.
3. Put mods into `Ferrite\mods`.

Silent install: `Ferrite-Setup.exe /silent "/dir=C:\path\to\game"`.

Uninstall: delete `version.dll` and the `Ferrite` folder from the game folder. Start the game with `--no-mods` to skip mods once.

```
version.dll           native loader (Rust)
Ferrite\
  core\               Ferrite.dll (C# API), Harmony, MonoMod, Mono.Cecil
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

## How it works

`version.dll` is a proxy for the Windows `version.dll` that Unity loads at start. It hooks `mono_jit_init_version`, and once the Mono domain exists it opens `Ferrite.dll`, registers internal calls and starts the managed runtime. On the first scene the C# side asks the Rust core for mods: Rust scans `Ferrite\mods`, finds every class derived from `Ferrite.Mod` through the Mono embedding API, reads `[Mod]`, skips disabled and duplicate ids, sorts by dependencies and returns the types in load order. Logging, the console and configs stay on the Rust side and are reached through internal calls.

## Building

Requires Rust (stable, MSVC), the .NET SDK and any Unity Mono game for the Unity reference assemblies.

```powershell
.\build.ps1 -GameDir "C:\...\Some Unity Game"
.\build.ps1 -Install "C:\...\Some Unity Game"
cargo clippy --workspace
cargo fmt --check
```

Output: `dist\`, installer in `dist\setup\`.

## License

MIT, see [LICENSE](LICENSE). Bundled libraries in `libs\`: Harmony, MonoMod, Mono.Cecil (MIT).
