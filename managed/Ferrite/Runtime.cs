using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Ferrite {
    public static class Runtime {
        public const string Version = "0.1.0";

        static readonly List<Mod> mods = new List<Mod>();
        static readonly Logger log = new Logger("Ferrite");
        static bool started;

        public static IReadOnlyList<Mod> Mods => mods;
        public static string RootDir { get; private set; }
        public static string CoreDir { get; private set; }
        public static string ModsDir { get; private set; }

        public static void Boot() {
            RootDir = Native.RootDir();
            CoreDir = Path.Combine(RootDir, "core");
            ModsDir = Path.Combine(RootDir, "mods");
            Directory.CreateDirectory(ModsDir);
            AppDomain.CurrentDomain.AssemblyResolve += Resolve;
            Console.SetOut(new LoggerWriter(new Logger("Console")));
            SceneManager.sceneLoaded += OnSceneLoaded;
        }

        static Assembly Resolve(object sender, ResolveEventArgs args) {
            var name = new AssemblyName(args.Name).Name;
            foreach (var asm in AppDomain.CurrentDomain.GetAssemblies()) {
                if (asm.GetName().Name == name) {
                    return asm;
                }
            }
            var core = Path.Combine(CoreDir, name + ".dll");
            if (File.Exists(core)) {
                return Assembly.LoadFrom(core);
            }
            foreach (var file in Directory.GetFiles(ModsDir, name + ".dll", SearchOption.AllDirectories)) {
                return Assembly.LoadFrom(file);
            }
            return null;
        }

        static void OnSceneLoaded(Scene scene, LoadSceneMode mode) {
            if (!started) {
                started = true;
                try {
                    Start();
                } catch (Exception e) {
                    log.Error($"start failed: {e}");
                }
            }
            Each(m => m.OnSceneLoaded(scene, mode), nameof(Mod.OnSceneLoaded));
        }

        static void Start() {
            log.Info($"Unity {Application.unityVersion}, {Application.productName} {Application.version}");
            if (Native.Option("unity_log") == "true") {
                var unity = new Logger("Unity");
                Application.logMessageReceived += (message, trace, type) => {
                    var level = type == LogType.Log ? LogLevel.Info : type == LogType.Warning ? LogLevel.Warning : LogLevel.Error;
                    unity.Write(level, level == LogLevel.Error && !string.IsNullOrEmpty(trace) ? message + "\n" + trace.TrimEnd() : message);
                };
            }
            var host = new GameObject("Ferrite");
            host.hideFlags = HideFlags.HideAndDontSave;
            UnityEngine.Object.DontDestroyOnLoad(host);
            host.AddComponent<Host>();
            Commands.RegisterBuiltins();
            LoadMods();
        }

        static void LoadMods() {
            foreach (var type in Native.DiscoverMods() ?? new Type[0]) {
                var info = type.GetCustomAttribute<ModAttribute>();
                Mod mod;
                try {
                    mod = (Mod)Activator.CreateInstance(type);
                    mod.Info = info;
                    mod.Logger = new Logger(info.Name);
                    mod.Config = new ModConfig(info.Id);
                    mod.Location = type.Assembly.Location;
                    mod.OnLoad();
                } catch (Exception e) {
                    log.Error($"{info.Id} failed to load: {e}");
                    continue;
                }
                mods.Add(mod);
                log.Info($"loaded {info.Name} {info.Version} ({info.Id})");
            }
            log.Info($"{mods.Count} mod(s) loaded");
        }

        internal static void Each(Action<Mod> action, string callback) {
            for (var i = 0; i < mods.Count; i++) {
                var mod = mods[i];
                if (mod.Faulted) {
                    continue;
                }
                try {
                    action(mod);
                } catch (Exception e) {
                    mod.Faulted = true;
                    log.Error($"{mod.Info.Id} {callback} threw, callbacks disabled: {e}");
                }
            }
        }
    }

    sealed class Host : MonoBehaviour {
        void Update() {
            Commands.Poll();
            Runtime.Each(m => m.OnUpdate(), nameof(Mod.OnUpdate));
        }

        void LateUpdate() => Runtime.Each(m => m.OnLateUpdate(), nameof(Mod.OnLateUpdate));
        void FixedUpdate() => Runtime.Each(m => m.OnFixedUpdate(), nameof(Mod.OnFixedUpdate));
        void OnGUI() => Runtime.Each(m => m.OnGUI(), nameof(Mod.OnGUI));
        void OnApplicationQuit() => Runtime.Each(m => m.OnQuit(), nameof(Mod.OnQuit));
    }
}
