using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.RegularExpressions;

namespace BepInEx {
    [AttributeUsage(AttributeTargets.Class, AllowMultiple = false)]
    public class BepInPlugin : Attribute {
        public BepInPlugin(string GUID, string Name, string Version) {
            this.GUID = GUID;
            this.Name = Name;
            this.Version = ParseVersion(Version);
        }

        public string GUID { get; protected set; }
        public string Name { get; protected set; }
        public Version Version { get; protected set; }

        internal static Version ParseVersion(string version) {
            if (string.IsNullOrEmpty(version)) {
                return null;
            }
            try {
                return new Version(version);
            } catch (Exception) {
            }
            var parts = Regex.Match(version, @"^\d+(\.\d+){0,3}").Value.Split(new[] { '.' }, StringSplitOptions.RemoveEmptyEntries);
            if (parts.Length == 0) {
                return null;
            }
            var numbers = parts.Select(p => int.TryParse(p, out var n) ? n : 0).ToList();
            while (numbers.Count < 2) {
                numbers.Add(0);
            }
            switch (numbers.Count) {
                case 2:
                    return new Version(numbers[0], numbers[1]);
                case 3:
                    return new Version(numbers[0], numbers[1], numbers[2]);
                default:
                    return new Version(numbers[0], numbers[1], numbers[2], numbers[3]);
            }
        }
    }

    [AttributeUsage(AttributeTargets.Class, AllowMultiple = true)]
    public class BepInDependency : Attribute {
        [Flags]
        public enum DependencyFlags {
            HardDependency = 1,
            SoftDependency = 2,
        }

        public BepInDependency(string DependencyGUID, DependencyFlags Flags = DependencyFlags.HardDependency) {
            this.DependencyGUID = DependencyGUID;
            this.Flags = Flags;
            MinimumVersion = new Version();
        }

        public BepInDependency(string DependencyGUID, string MinimumDependencyVersion) : this(DependencyGUID) {
            MinimumVersion = BepInPlugin.ParseVersion(MinimumDependencyVersion) ?? new Version();
        }

        public string DependencyGUID { get; protected set; }
        public DependencyFlags Flags { get; protected set; }
        public Version MinimumVersion { get; protected set; }
    }

    [AttributeUsage(AttributeTargets.Class, AllowMultiple = true)]
    public class BepInIncompatibility : Attribute {
        public BepInIncompatibility(string IncompatibilityGUID) {
            this.IncompatibilityGUID = IncompatibilityGUID;
        }

        public string IncompatibilityGUID { get; protected set; }
    }

    [AttributeUsage(AttributeTargets.Class, AllowMultiple = true)]
    public class BepInProcess : Attribute {
        public BepInProcess(string ProcessName) {
            this.ProcessName = ProcessName;
        }

        public string ProcessName { get; protected set; }
    }

    public class PluginInfo {
        public BepInPlugin Metadata { get; internal set; }
        public IEnumerable<BepInProcess> Processes { get; internal set; } = new BepInProcess[0];
        public IEnumerable<BepInDependency> Dependencies { get; internal set; } = new BepInDependency[0];
        public IEnumerable<BepInIncompatibility> Incompatibilities { get; internal set; } = new BepInIncompatibility[0];
        public string Location { get; internal set; }
        public BaseUnityPlugin Instance { get; internal set; }
        public string TypeName { get; internal set; }

        public override string ToString() => $"{Metadata?.Name} {Metadata?.Version}";
    }

    public static class MetadataHelper {
        public static BepInPlugin GetMetadata(Type pluginType) {
            var attributes = pluginType.GetCustomAttributes(typeof(BepInPlugin), false);
            return attributes.Length == 0 ? null : (BepInPlugin)attributes[0];
        }

        public static BepInPlugin GetMetadata(object plugin) => GetMetadata(plugin.GetType());

        public static T[] GetAttributes<T>(Type pluginType) where T : Attribute => (T[])pluginType.GetCustomAttributes(typeof(T), true);

        public static IEnumerable<T> GetAttributes<T>(object plugin) where T : Attribute => GetAttributes<T>(plugin.GetType());

        public static IEnumerable<BepInDependency> GetDependencies(Type plugin) => plugin.GetCustomAttributes(typeof(BepInDependency), true).Cast<BepInDependency>();

        internal static PluginInfo Describe(Type type, string location) => new PluginInfo {
            Metadata = GetMetadata(type),
            Processes = GetAttributes<BepInProcess>(type),
            Dependencies = GetDependencies(type).ToArray(),
            Incompatibilities = GetAttributes<BepInIncompatibility>(type),
            Location = location,
            TypeName = type.FullName,
        };
    }
}
