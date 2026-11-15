using System;
using HarmonyLib;
using UnityEngine.SceneManagement;

namespace Ferrite {
    [AttributeUsage(AttributeTargets.Class, Inherited = false)]
    public sealed class ModAttribute : Attribute {
        public readonly string Id;
        public readonly string Name;
        public readonly string Version;
        public string Author;
        public string[] Dependencies;

        public ModAttribute(string id, string name, string version) {
            Id = id;
            Name = name;
            Version = version;
        }
    }

    public abstract class Mod {
        Harmony harmony;

        public ModAttribute Info { get; internal set; }
        public Logger Logger { get; internal set; }
        public ModConfig Config { get; internal set; }
        public string Location { get; internal set; }
        public Harmony Harmony => harmony ?? (harmony = new Harmony(Info.Id));
        internal bool Faulted { get; set; }

        public virtual void OnLoad() {
        }

        public virtual void OnSceneLoaded(Scene scene, LoadSceneMode mode) {
        }

        public virtual void OnUpdate() {
        }

        public virtual void OnLateUpdate() {
        }

        public virtual void OnFixedUpdate() {
        }

        public virtual void OnGUI() {
        }

        public virtual void OnQuit() {
        }
    }
}
