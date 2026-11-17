using System;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Threading;
using Ferrite;

namespace HelloIl2CppCs {
    [Mod("hello-il2cpp-cs", "Hello IL2CPP C#", "1.0.0", Author = "Ferrite")]
    public sealed unsafe class HelloIl2CppCs : Mod {
        const string DefaultGreeting = "Hello from C#!";

        static delegate* unmanaged<IntPtr, void> canvasOriginal;
        static long canvasCalls;

        readonly Stopwatch clock = Stopwatch.StartNew();
        int frames;
        long updates;
        double fps;
        Il2CppMethod canvas;

        public override void OnLoad() {
            var greeting = Config.Get("greeting", DefaultGreeting, "Text printed on load and by the hello command.");
            if (UnityLog("[Hello IL2CPP C#] " + greeting)) {
                Logger.Info(greeting + " Also written to the game's Unity log.");
            } else {
                Logger.Info(greeting);
            }
            Command("cshello", "Prints the greeting, `cshello <text>` sets a new one.", args => {
                if (args.Length > 0) {
                    Config.Set("greeting", string.Join(" ", args));
                }
                Logger.Info(Config.Get("greeting", DefaultGreeting));
            });
            Command("csfps", "Prints frames per second measured by the C# mod.", _ => Logger.Info($"{fps:F1} fps, {updates} updates since load"));
            Command("cshooktest", "Hooks Canvas.SendWillRenderCanvases from C# and counts calls.", _ => HookCanvas());
        }

        static bool UnityLog(string message) {
            var log = Il2Cpp.Method("UnityEngine", "Debug", "Log", 1);
            if (log.IsNull) {
                return false;
            }
            log.Invoke(default, message);
            return true;
        }

        [UnmanagedCallersOnly]
        static void CanvasDetour(IntPtr method) {
            Interlocked.Increment(ref canvasCalls);
            canvasOriginal(method);
        }

        void HookCanvas() {
            if (canvasOriginal != null) {
                Logger.Info($"Canvas.SendWillRenderCanvases called {Interlocked.Read(ref canvasCalls)} times");
                return;
            }
            canvas = Il2Cpp.Method("UnityEngine", "Canvas", "SendWillRenderCanvases", 0);
            if (canvas.IsNull) {
                Logger.Warning("Canvas.SendWillRenderCanvases not found");
                return;
            }
            var original = Hook(canvas, (IntPtr)(delegate* unmanaged<IntPtr, void>)&CanvasDetour);
            if (original == IntPtr.Zero) {
                Logger.Error("hook failed");
                return;
            }
            canvasOriginal = (delegate* unmanaged<IntPtr, void>)original;
            Logger.Info("hooked Canvas.SendWillRenderCanvases, run cshooktest again to see the count");
        }

        public override void OnUpdate() {
            frames++;
            updates++;
            var elapsed = clock.Elapsed.TotalSeconds;
            if (elapsed >= 1.0) {
                fps = frames / elapsed;
                frames = 0;
                clock.Restart();
            }
        }

        public override void OnSceneLoaded(string scene) => Logger.Info($"scene {scene} loaded");

        public override void OnConfigChanged() => Logger.Info("config changed, greeting is now: " + Config.Get("greeting", DefaultGreeting));

        public override void OnUnload() {
            canvasOriginal = null;
            Logger.Info("unloading, hooks are removed by Ferrite");
        }
    }
}
