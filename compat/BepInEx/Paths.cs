using System;
using System.Diagnostics;
using System.IO;
using UnityEngine;

namespace BepInEx {
    public static class Paths {
        static Paths() {
            var root = Path.GetFullPath(Ferrite.Runtime.RootDir);
            BepInExRootPath = root;
            ConfigPath = Ferrite.Runtime.ConfigDir;
            BepInExConfigPath = Path.Combine(ConfigPath, "BepInEx.cfg");
            PluginPath = Path.Combine(root, "plugins");
            PatcherPluginPath = Path.Combine(root, "patchers");
            CachePath = Path.Combine(root, "cache");
            BepInExAssemblyDirectory = Ferrite.Runtime.CoreDir;
            BepInExAssemblyPath = Path.Combine(BepInExAssemblyDirectory, "BepInEx.dll");
            string data = null;
            try {
                data = Application.dataPath;
            } catch (Exception) {
            }
            if (!string.IsNullOrEmpty(data)) {
                data = Path.GetFullPath(data);
                GameRootPath = Path.GetDirectoryName(data);
                ManagedPath = Path.Combine(data, "Managed");
                var name = Path.GetFileName(data);
                ProcessName = name.EndsWith("_Data", StringComparison.OrdinalIgnoreCase) ? name.Substring(0, name.Length - 5) : name;
                ExecutablePath = Path.Combine(GameRootPath, ProcessName + ".exe");
            } else {
                GameRootPath = Path.GetDirectoryName(root);
                ExecutablePath = Process.GetCurrentProcess().MainModule.FileName;
                ProcessName = Path.GetFileNameWithoutExtension(ExecutablePath);
                ManagedPath = Path.Combine(Path.Combine(GameRootPath, ProcessName + "_Data"), "Managed");
            }
            DllSearchPaths = new[] { ManagedPath };
        }

        public static string[] DllSearchPaths { get; private set; }
        public static string BepInExAssemblyDirectory { get; private set; }
        public static string BepInExAssemblyPath { get; private set; }
        public static string BepInExRootPath { get; private set; }
        public static string ExecutablePath { get; private set; }
        public static string GameRootPath { get; private set; }
        public static string ManagedPath { get; private set; }
        public static string ConfigPath { get; private set; }
        public static string BepInExConfigPath { get; private set; }
        public static string CachePath { get; private set; }
        public static string PatcherPluginPath { get; private set; }
        public static string PluginPath { get; private set; }
        public static string ProcessName { get; private set; }
    }
}
