using Ferrite;
using HarmonyLib;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace HelloMod {
    [Mod("hello", "Hello Mod", "1.0.0", Author = "you")]
    public sealed class HelloMod : Mod {
        internal static HelloMod Instance;
        bool overlay;
        bool legacyInput = true;
        string scene = "";
        string greeting;

        public override void OnLoad() {
            Instance = this;
            greeting = Config.Get("greeting", "Hello from Ferrite!", "Text printed on load and by the hello command.");
            overlay = Config.Get("overlay", false, "Shows a label in the top-left corner. F8 toggles it.");
            Harmony.PatchAll(typeof(HelloMod).Assembly);
            Commands.Register("hello", "Prints the greeting or sets a new one.", args => {
                if (args.Length > 0) {
                    greeting = string.Join(" ", args);
                    Config.Set("greeting", greeting);
                }
                Commands.Print(greeting);
            }, "[text]");
            var marker = System.Type.GetType("Ferrite.PatchedByHelloPatcher, Assembly-CSharp");
            Logger.Info(marker != null ? "Assembly-CSharp was patched by HelloPatcher" : "HelloPatcher is not installed");
            Logger.Info($"[build 4] {greeting} Type `hello` in the Ferrite console, F8 toggles the overlay.");
        }

        public override void OnConfigChanged() {
            greeting = Config.Get("greeting", greeting);
            overlay = Config.Get("overlay", overlay);
            Logger.Info($"config changed: greeting = {greeting}, overlay = {overlay}");
        }

        public override void OnUnload() {
            Logger.Info("v2 unloading, Harmony patches and commands are removed by Ferrite");
        }

        public override void OnSceneLoaded(Scene loaded, LoadSceneMode mode) {
            scene = loaded.name;
        }

        public override void OnUpdate() {
            if (!legacyInput) {
                return;
            }
            bool pressed;
            try {
                pressed = Input.GetKeyDown(KeyCode.F8);
            } catch (System.InvalidOperationException) {
                legacyInput = false;
                Logger.Info("this game uses only the new Input System, F8 overlay disabled (set overlay = true in the config instead)");
                return;
            }
            if (pressed) {
                overlay = !overlay;
                Config.Set("overlay", overlay);
            }
        }

        public override void OnGUI() {
            if (overlay) {
                GUI.Label(new Rect(10, 10, 800, 25), $"{greeting} | {Application.productName} | scene: {scene} | {Runtime.Mods.Count} mod(s)");
            }
        }
    }

    [HarmonyPatch(typeof(Application), nameof(Application.OpenURL))]
    static class OpenUrlPatch {
        static void Prefix(string url) {
            HelloMod.Instance?.Logger.Info($"game opens {url}");
        }
    }
}
