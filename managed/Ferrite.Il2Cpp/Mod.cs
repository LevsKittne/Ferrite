using System;

namespace Ferrite {
    [AttributeUsage(AttributeTargets.Class, Inherited = false)]
    public sealed class ModAttribute : Attribute {
        public readonly string Id;
        public readonly string Name;
        public readonly string Version;
        public string Author;
        public string[] Dependencies;
        public string[] Incompatibilities;
        public string[] Processes;

        public ModAttribute(string id, string name, string version) {
            Id = id;
            Name = name;
            Version = version;
        }
    }

    public abstract class Mod {
        internal IntPtr Handle;

        public ModAttribute Info { get; internal set; }
        public Logger Logger { get; internal set; }
        public ModConfig Config { get; internal set; }
        public string Location { get; internal set; }
        public string Directory => System.IO.Path.GetDirectoryName(Location);
        public string GameDirectory => Native.Directory(Handle, 1);
        public string ConfigDirectory => Native.Directory(Handle, 3);

        public virtual void OnLoad() {
        }

        public virtual void OnUpdate() {
        }

        public virtual void OnSceneLoaded(string scene) {
        }

        public virtual void OnConfigChanged() {
        }

        public virtual void OnUnload() {
        }

        public void Command(string name, string description, Action<string[]> handler) => Commands.Register(this, name, description, handler);

        public unsafe IntPtr Hook(IntPtr target, IntPtr detour) {
            if (target == IntPtr.Zero || detour == IntPtr.Zero) {
                throw new ArgumentNullException(target == IntPtr.Zero ? nameof(target) : nameof(detour));
            }
            return Native.Api->HookOwned(Handle, target, detour);
        }

        public IntPtr Hook(Il2CppMethod method, IntPtr detour) => Hook(method.Address, detour);

        public unsafe bool Unhook(IntPtr target) => Native.Api->Unhook(Handle, target) != 0;

        public bool Unhook(Il2CppMethod method) => Unhook(method.Address);

        public unsafe IntPtr ReplacePointer(Il2CppMethod method, IntPtr replacement) {
            var slots = (IntPtr*)method.Pointer;
            var previous = slots[0];
            Native.Api->PatchPointer(Handle, (IntPtr)slots, replacement);
            if (slots[1] == previous) {
                Native.Api->PatchPointer(Handle, (IntPtr)(slots + 1), replacement);
            }
            return previous;
        }

        public static void Execute(string line) => Native.Execute(line);
    }
}
