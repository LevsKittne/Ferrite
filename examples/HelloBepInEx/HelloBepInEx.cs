using System.Linq;
using BepInEx;
using BepInEx.Bootstrap;
using BepInEx.Configuration;
using HarmonyLib;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace HelloBepInEx {
    [BepInPlugin(Guid, "Hello BepInEx", "1.0.0")]
    [BepInDependency("com.example.optional", BepInDependency.DependencyFlags.SoftDependency)]
    public sealed class HelloPlugin : BaseUnityPlugin {
        public const string Guid = "ferrite.hello.bepinex";

        internal static HelloPlugin Instance;

        ConfigEntry<string> greeting;
        ConfigEntry<int> interval;
        ConfigEntry<KeyboardShortcut> key;
        int frames;
        float next;

        void Awake() {
            Instance = this;
            greeting = Config.Bind("General", "Greeting", "Hello from a BepInEx plugin!", "Logged on start.");
            interval = Config.Bind("General", "Interval", 10, new ConfigDescription("Seconds between update logs.", new AcceptableValueRange<int>(1, 60)));
            key = Config.Bind("Input", "Key", new KeyboardShortcut(KeyCode.F6), "Logs a line when pressed.");
            interval.SettingChanged += (s, e) => Logger.LogInfo($"interval changed to {interval.Value}");
            new Harmony(Guid).PatchAll(typeof(HelloPlugin).Assembly);
            Logger.LogInfo(greeting.Value);
            Logger.LogInfo($"interval {interval.Value}, key {key.Value}, config {Config.ConfigFilePath}");
            Logger.LogInfo($"plugins: {string.Join(", ", Chainloader.PluginInfos.Keys.ToArray())}; PluginPath {Paths.PluginPath}; process {Paths.ProcessName}");
        }

        void Update() {
            frames++;
            if (Time.realtimeSinceStartup >= next) {
                next = Time.realtimeSinceStartup + interval.Value;
                Logger.LogInfo($"Update ran {frames} frames");
            }
            if (key.Value.IsDown()) {
                Logger.LogMessage("key pressed");
            }
        }

        internal void Write(string text) => Logger.LogInfo(text);
    }

    [HarmonyPatch(typeof(SceneManager), "Internal_SceneLoaded")]
    static class SceneLoadedPatch {
        static void Postfix(Scene scene) => HelloPlugin.Instance?.Write($"Harmony postfix: scene {scene.name} loaded");
    }
}
