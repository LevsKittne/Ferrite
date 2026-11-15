using System;
using System.IO;
using System.Reflection;
using System.Runtime.CompilerServices;

namespace Ferrite {
    static class Bootstrap {
        internal static string RootDir;
        internal static string CoreDir;
        internal static string ModsDir;
        internal static string PatchersDir;
        internal static string ConfigDir;
        internal static bool HotReload;

        public static void Start() {
            RootDir = Native.RootDir();
            CoreDir = Path.Combine(RootDir, "core");
            ModsDir = Path.Combine(RootDir, "mods");
            PatchersDir = Path.Combine(RootDir, "patchers");
            ConfigDir = Path.Combine(RootDir, "config");
            HotReload = Native.Option("hot_reload") == "true";
            Directory.CreateDirectory(ModsDir);
            AppDomain.CurrentDomain.AssemblyResolve += Resolve;
            Console.SetOut(new LoggerWriter(new Logger("Console")));
            RunPreloader();
            StartRuntime();
        }

        [MethodImpl(MethodImplOptions.NoInlining)]
        static void RunPreloader() {
            try {
                Preloader.Run(Native.Option("entrypoint") == "early");
            } catch (Exception e) {
                new Logger("Preloader").Error($"preloader failed: {e}");
            }
        }

        [MethodImpl(MethodImplOptions.NoInlining)]
        static void StartRuntime() => Runtime.Boot();

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
            foreach (var dir in new[] { ModsDir, PatchersDir }) {
                if (!Directory.Exists(dir)) {
                    continue;
                }
                foreach (var file in Directory.GetFiles(dir, name + ".dll", SearchOption.AllDirectories)) {
                    return Assembly.LoadFrom(file);
                }
            }
            return null;
        }
    }
}
