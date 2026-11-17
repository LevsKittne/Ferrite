using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Runtime.CompilerServices;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Ferrite {
    public static class Runtime {
        public const string Version = "0.2.1";

        static readonly List<Mod> mods = new List<Mod>();
        static readonly Dictionary<string, string[][]> external = new Dictionary<string, string[][]>();
        static readonly Logger log = new Logger("Ferrite");
        static bool started;
        static float nextReloadCheck;

        public static IReadOnlyList<Mod> Mods => mods;
        public static string RootDir => Bootstrap.RootDir;
        public static string CoreDir => Bootstrap.CoreDir;
        public static string ModsDir => Bootstrap.ModsDir;
        public static string ConfigDir => Bootstrap.ConfigDir;
        public static bool HotReload => Bootstrap.HotReload;

        public static Mod Find(string id) => mods.FirstOrDefault(m => string.Equals(m.Info.Id, id, StringComparison.OrdinalIgnoreCase));
        public static bool IsLoaded(string id) => Find(id) != null;

        internal static void Boot() {
            SceneManager.sceneLoaded += OnSceneLoaded;
        }

        public static void EarlyStart() {
            try {
                StartOnce("before the first scene");
            } catch (Exception e) {
                log.Error($"early start failed: {e}");
            }
        }

        static void OnSceneLoaded(Scene scene, LoadSceneMode mode) {
            try {
                StartOnce("on the first scene");
            } catch (Exception e) {
                log.Error($"start failed: {e}");
            }
            Each(m => m.OnSceneLoaded(scene, mode), nameof(Mod.OnSceneLoaded));
        }

        static void StartOnce(string when) {
            if (started) {
                return;
            }
            started = true;
            log.Info($"Unity {Application.unityVersion}, {Application.productName} {Application.version}, starting {when}");
            if (Native.Option("unity_log") == "true") {
                var unity = new Logger("Unity");
                Application.logMessageReceivedThreaded += (message, trace, type) => {
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
            StartCompat();
        }

        static void StartCompat() {
            var path = Path.Combine(CoreDir, "BepInEx.dll");
            if (!File.Exists(path)) {
                return;
            }
            try {
                Assembly.LoadFrom(path).GetType("BepInEx.Bootstrap.Chainloader", true).GetMethod("Start", Type.EmptyTypes).Invoke(null, null);
            } catch (Exception e) {
                log.Error($"BepInEx compatibility layer failed: {(e is TargetInvocationException && e.InnerException != null ? e.InnerException : e)}");
            }
        }

        public static void ReportExternal(string kind, IEnumerable<string[]> rows) {
            external[kind] = (rows ?? new string[0][]).ToArray();
            Report();
        }

        static void LoadMods() {
            var started = DateTime.Now;
            foreach (var type in Native.DiscoverMods() ?? new Type[0]) {
                var mod = Create(type);
                if (mod != null) {
                    mods.Add(mod);
                }
            }
            Report();
            log.Info($"{mods.Count} mod(s) loaded in {(DateTime.Now - started).TotalMilliseconds:0} ms");
        }

        static Mod Create(Type type) {
            var info = type.GetCustomAttribute<ModAttribute>();
            try {
                var mod = (Mod)Activator.CreateInstance(type);
                mod.Info = info;
                mod.Logger = new Logger(info.Name);
                mod.Config = new ModConfig(info.Id);
                mod.Location = type.Assembly.Location;
                mod.Stamp = Stamp(mod.Location);
                mod.OnLoad();
                log.Info($"loaded {info.Name} {info.Version} ({info.Id})");
                return mod;
            } catch (Exception e) {
                log.Error($"{info.Id} failed to load: {e}");
                return null;
            }
        }

        static DateTime Stamp(string path) {
            try {
                return string.IsNullOrEmpty(path) ? default : File.GetLastWriteTimeUtc(path);
            } catch (Exception) {
                return default;
            }
        }

        public static bool Reload(string id) {
            var old = Find(id);
            if (old == null) {
                log.Warning($"mod {id} is not loaded");
                return false;
            }
            var path = old.Location;
            if (string.IsNullOrEmpty(path) || !File.Exists(path)) {
                log.Error($"cannot reload {id}: file {path} not found");
                return false;
            }
            Type type;
            try {
                type = Reloader.Load(path, old.GetType().FullName);
            } catch (Exception e) {
                log.Error($"cannot reload {id}: {e.Message}");
                return false;
            }
            if (type == null) {
                log.Error($"cannot reload {id}: {old.GetType().FullName} not found in the new build");
                return false;
            }
            Unload(old);
            var index = mods.IndexOf(old);
            var mod = Create(type);
            if (mod == null) {
                mods.RemoveAt(index);
                return false;
            }
            mod.Location = path;
            mod.Stamp = Stamp(path);
            mods[index] = mod;
            Report();
            mod.ReloadedAt = Time.realtimeSinceStartup;
            log.Info($"reloaded {mod.Info.Name} {mod.Info.Version}");
            return true;
        }

        static void Unload(Mod mod) {
            try {
                mod.OnUnload();
            } catch (Exception e) {
                log.Error($"{mod.Info.Id} OnUnload threw: {e}");
            }
            mod.UnpatchAll();
            Commands.RemoveOwner(mod);
        }

        internal static void CheckReloads() {
            if (!HotReload || Time.realtimeSinceStartup < nextReloadCheck) {
                return;
            }
            nextReloadCheck = Time.realtimeSinceStartup + 1f;
            foreach (var mod in mods.ToArray()) {
                var stamp = Stamp(mod.Location);
                if (stamp != default && stamp != mod.Stamp && Time.realtimeSinceStartup - mod.ReloadedAt > 1f) {
                    mod.Stamp = stamp;
                    log.Info($"{Path.GetFileName(mod.Location)} changed, reloading {mod.Info.Id}");
                    Reload(mod.Info.Id);
                }
            }
        }

        internal static void PollConfigs() {
            var changed = Native.ConfigPoll();
            if (string.IsNullOrEmpty(changed)) {
                return;
            }
            foreach (var id in changed.Split('\n')) {
                var mod = Find(id);
                if (mod == null) {
                    continue;
                }
                log.Info($"config of {id} changed");
                try {
                    mod.Config.RaiseChanged();
                    mod.OnConfigChanged();
                } catch (Exception e) {
                    log.Error($"{id} OnConfigChanged threw: {e}");
                }
            }
        }

        internal static void Report() {
            var rows = mods.Select(m => new[] { m.Info.Id, m.Info.Name, m.Info.Version, m.Info.Author, m.Faulted ? "1" : "0", "C#" })
                .Concat(external.SelectMany(e => e.Value.Select(r => Enumerable.Range(0, 5).Select(i => r != null && i < r.Length ? r[i] : "").Concat(new[] { e.Key }).ToArray())))
                .Select(r => string.Join("\t", r.Select(Clean).ToArray()));
            Native.ReportMods(string.Join("\n", rows.ToArray()));
        }

        static string Clean(string f) => (f ?? "").Replace('\t', ' ').Replace('\n', ' ');

        internal static void Each(Action<Mod> action, string callback) {
            for (var i = 0; i < mods.Count; i++) {
                var mod = mods[i];
                if (mod.Faulted) {
                    continue;
                }
                var owner = Commands.CurrentOwner;
                Commands.CurrentOwner = mod;
                try {
                    action(mod);
                } catch (Exception e) {
                    mod.Faulted = true;
                    Report();
                    log.Error($"{mod.Info.Id} {callback} threw, callbacks disabled: {e}");
                } finally {
                    Commands.CurrentOwner = owner;
                }
            }
        }
    }

    sealed class Host : MonoBehaviour {
        void Update() {
            Commands.Poll();
            Cursors.Update();
            Runtime.PollConfigs();
            Runtime.CheckReloads();
            Runtime.Each(m => m.OnUpdate(), nameof(Mod.OnUpdate));
        }

        void LateUpdate() => Runtime.Each(m => m.OnLateUpdate(), nameof(Mod.OnLateUpdate));
        void FixedUpdate() => Runtime.Each(m => m.OnFixedUpdate(), nameof(Mod.OnFixedUpdate));
        void OnGUI() => Runtime.Each(m => m.OnGUI(), nameof(Mod.OnGUI));
        void OnApplicationQuit() => Runtime.Each(m => m.OnQuit(), nameof(Mod.OnQuit));
    }
}
