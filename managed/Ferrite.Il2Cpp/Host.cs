using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.Loader;
using System.Text;

namespace Ferrite {
    [StructLayout(LayoutKind.Sequential)]
    unsafe struct HostTable {
        public delegate* unmanaged<char*, int, byte*, int, nuint*, int> Inspect;
        public delegate* unmanaged<nuint, IntPtr, int> Load;
        public delegate* unmanaged<nuint, void> Update;
        public delegate* unmanaged<nuint, byte*, nuint, void> Scene;
        public delegate* unmanaged<nuint, void> Config;
        public delegate* unmanaged<nuint, void> Unload;
        public delegate* unmanaged<nuint, void> Release;
    }

    sealed class ModLoadContext : AssemblyLoadContext {
        readonly string directory;

        public ModLoadContext(string file) : base(Path.GetFileNameWithoutExtension(file), true) {
            directory = Path.GetDirectoryName(file);
        }

        public Assembly LoadFile(string file) {
            var pdb = Path.ChangeExtension(file, ".pdb");
            using var dll = new MemoryStream(File.ReadAllBytes(file));
            if (File.Exists(pdb)) {
                using var symbols = new MemoryStream(File.ReadAllBytes(pdb));
                return LoadFromStream(dll, symbols);
            }
            return LoadFromStream(dll);
        }

        protected override Assembly Load(AssemblyName name) {
            if (name.Name == typeof(Host).Assembly.GetName().Name) {
                return typeof(Host).Assembly;
            }
            var file = Path.Combine(directory, name.Name + ".dll");
            return File.Exists(file) ? LoadFile(file) : null;
        }
    }

    static unsafe class Host {
        sealed class Slot {
            public ModLoadContext Context;
            public Type Type;
            public ModAttribute Info;
            public string File;
            public Mod Instance;
            public bool Faulted;
        }

        static readonly Dictionary<nuint, Slot> slots = new Dictionary<nuint, Slot>();
        static nuint next;

        internal static Mod Current;

        [UnmanagedCallersOnly]
        static int Init(Api* api, HostTable* table) {
            try {
                Native.Api = api;
                Il2Cpp.Init();
                table->Inspect = &Inspect;
                table->Load = &Load;
                table->Update = &Update;
                table->Scene = &Scene;
                table->Config = &Config;
                table->Unload = &Unload;
                table->Release = &Release;
                return 0;
            } catch (Exception e) {
                try {
                    Native.Log(IntPtr.Zero, 3, "Ferrite.Il2Cpp: " + e);
                } catch (Exception) {
                }
                return 1;
            }
        }

        static int Write(byte* buffer, int capacity, string text) {
            var bytes = Encoding.UTF8.GetBytes(text);
            var n = Math.Min(bytes.Length, capacity);
            Marshal.Copy(bytes, 0, (IntPtr)buffer, n);
            return Math.Max(n, 1);
        }

        static string Clean(string s) => (s ?? "").Replace('\n', ' ').Replace('\r', ' ').Trim();

        static IEnumerable<Type> LoadableTypes(Assembly assembly) {
            try {
                return assembly.GetTypes();
            } catch (ReflectionTypeLoadException e) {
                return e.Types.Where(t => t != null);
            }
        }

        [UnmanagedCallersOnly]
        static int Inspect(char* path, int length, byte* buffer, int capacity, nuint* token) {
            *token = 0;
            ModLoadContext context = null;
            try {
                var file = new string(path, 0, length);
                context = new ModLoadContext(file);
                Assembly assembly;
                try {
                    assembly = context.LoadFile(file);
                } catch (BadImageFormatException) {
                    context.Unload();
                    return 0;
                }
                var references = assembly.GetReferencedAssemblies();
                if (!references.Any(r => r.Name == typeof(Host).Assembly.GetName().Name)) {
                    context.Unload();
                    if (references.Any(r => r.Name == "Ferrite")) {
                        return -Write(buffer, capacity, "built against the Mono API (Ferrite.dll), IL2CPP games need mods that reference Ferrite.Il2Cpp");
                    }
                    return 0;
                }
                var type = LoadableTypes(assembly).FirstOrDefault(t => !t.IsAbstract && typeof(Mod).IsAssignableFrom(t) && t.GetCustomAttribute<ModAttribute>() != null);
                if (type == null) {
                    context.Unload();
                    return -Write(buffer, capacity, "no class derives from Ferrite.Mod with [Mod]");
                }
                var info = type.GetCustomAttribute<ModAttribute>();
                var id = ++next;
                slots[id] = new Slot { Context = context, Type = type, Info = info, File = file };
                *token = id;
                var deps = (info.Dependencies ?? Array.Empty<string>())
                    .Concat((info.Incompatibilities ?? Array.Empty<string>()).Select(d => "!" + d))
                    .Concat((info.Processes ?? Array.Empty<string>()).Select(p => "@" + p))
                    .Select(Clean);
                var text = string.Join("\n", Clean(info.Id), Clean(info.Name), Clean(info.Version), Clean(info.Author), string.Join(",", deps));
                return Write(buffer, capacity, text);
            } catch (Exception e) {
                context?.Unload();
                return -Write(buffer, capacity, e.GetType().Name + ": " + e.Message);
            }
        }

        internal static bool Run(Mod mod, string what, Action action) {
            var previous = Current;
            Current = mod;
            try {
                action();
                return true;
            } catch (Exception e) {
                var inner = e is TargetInvocationException t && t.InnerException != null ? t.InnerException : e;
                try {
                    mod.Logger.Error(what + " threw " + inner);
                } catch (Exception) {
                }
                return false;
            } finally {
                Current = previous;
            }
        }

        [UnmanagedCallersOnly]
        static int Load(nuint token, IntPtr handle) {
            try {
                if (!slots.TryGetValue(token, out var slot)) {
                    return 0;
                }
                var mod = (Mod)Activator.CreateInstance(slot.Type, true);
                mod.Handle = handle;
                mod.Info = slot.Info;
                mod.Location = slot.File;
                mod.Logger = new Logger(handle, slot.Info.Name);
                mod.Config = new ModConfig(handle);
                slot.Instance = mod;
                if (!Run(mod, "OnLoad", mod.OnLoad)) {
                    slot.Instance = null;
                    Commands.RemoveOwner(mod);
                    return 0;
                }
                return 1;
            } catch (Exception e) {
                try {
                    Native.Log(handle, 3, "cannot create the mod: " + e);
                } catch (Exception) {
                }
                return 0;
            }
        }

        [UnmanagedCallersOnly]
        static void Update(nuint token) {
            if (!slots.TryGetValue(token, out var slot) || slot.Instance == null || slot.Faulted) {
                return;
            }
            if (!Run(slot.Instance, "OnUpdate", slot.Instance.OnUpdate)) {
                slot.Faulted = true;
                slot.Instance.Logger.Error("OnUpdate is disabled until the mod is reloaded");
            }
        }

        [UnmanagedCallersOnly]
        static void Scene(nuint token, byte* name, nuint length) {
            if (!slots.TryGetValue(token, out var slot) || slot.Instance == null) {
                return;
            }
            var scene = Encoding.UTF8.GetString(name, (int)length);
            Run(slot.Instance, "OnSceneLoaded", () => slot.Instance.OnSceneLoaded(scene));
        }

        [UnmanagedCallersOnly]
        static void Config(nuint token) {
            if (!slots.TryGetValue(token, out var slot) || slot.Instance == null) {
                return;
            }
            Run(slot.Instance, "OnConfigChanged", () => {
                slot.Instance.OnConfigChanged();
                slot.Instance.Config.RaiseChanged();
            });
        }

        [UnmanagedCallersOnly]
        static void Unload(nuint token) {
            if (slots.TryGetValue(token, out var slot) && slot.Instance != null) {
                Run(slot.Instance, "OnUnload", slot.Instance.OnUnload);
            }
        }

        [UnmanagedCallersOnly]
        static void Release(nuint token) {
            if (!slots.Remove(token, out var slot)) {
                return;
            }
            try {
                if (slot.Instance != null) {
                    Commands.RemoveOwner(slot.Instance);
                }
                slot.Instance = null;
                slot.Type = null;
                slot.Context.Unload();
            } catch (Exception) {
            }
        }
    }
}
