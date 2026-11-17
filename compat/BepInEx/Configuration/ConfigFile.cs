using System;
using System.Collections;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.IO;
using System.Linq;
using BepInEx.Logging;

namespace BepInEx.Configuration {
    public class ConfigFile : IDictionary<ConfigDefinition, ConfigEntryBase> {
        readonly BepInPlugin ownerMetadata;
        readonly object ioLock = new object();

        public ConfigFile(string configPath, bool saveOnInit) : this(configPath, saveOnInit, null) {
        }

        public ConfigFile(string configPath, bool saveOnInit, BepInPlugin ownerMetadata) {
            if (configPath == null) {
                throw new ArgumentNullException(nameof(configPath));
            }
            this.ownerMetadata = ownerMetadata;
            ConfigFilePath = Path.GetFullPath(configPath);
            if (File.Exists(ConfigFilePath)) {
                Reload();
            } else if (saveOnInit) {
                Save();
            }
        }

        protected Dictionary<ConfigDefinition, ConfigEntryBase> Entries { get; } = new Dictionary<ConfigDefinition, ConfigEntryBase>();

        Dictionary<ConfigDefinition, string> OrphanedEntries { get; } = new Dictionary<ConfigDefinition, string>();

        [Obsolete("Use Keys instead")]
        public ReadOnlyCollection<ConfigDefinition> ConfigDefinitions {
            get {
                lock (ioLock) {
                    return Entries.Keys.ToList().AsReadOnly();
                }
            }
        }

        public string ConfigFilePath { get; }

        public bool SaveOnConfigSet { get; set; } = true;

        public event EventHandler ConfigReloaded;

        public event EventHandler<SettingChangedEventArgs> SettingChanged;

        [Obsolete("Use Values instead")]
        public ConfigEntryBase[] GetConfigEntries() {
            lock (ioLock) {
                return Entries.Values.ToArray();
            }
        }

        public void Reload() {
            lock (ioLock) {
                OrphanedEntries.Clear();
                var section = string.Empty;
                foreach (var raw in File.ReadAllLines(ConfigFilePath)) {
                    var line = raw.Trim();
                    if (line.StartsWith("#")) {
                        continue;
                    }
                    if (line.StartsWith("[") && line.EndsWith("]")) {
                        section = line.Substring(1, line.Length - 2);
                        continue;
                    }
                    var split = line.Split(new[] { '=' }, 2);
                    if (split.Length != 2) {
                        continue;
                    }
                    ConfigDefinition definition;
                    try {
                        definition = new ConfigDefinition(section, split[0].Trim());
                    } catch (ArgumentException) {
                        continue;
                    }
                    var value = split[1].Trim();
                    if (Entries.TryGetValue(definition, out var entry)) {
                        entry.SetSerializedValue(value);
                    } else {
                        OrphanedEntries[definition] = value;
                    }
                }
            }
            ConfigReloaded?.Invoke(this, EventArgs.Empty);
        }

        public void Save() {
            lock (ioLock) {
                var dir = Path.GetDirectoryName(ConfigFilePath);
                if (!string.IsNullOrEmpty(dir)) {
                    Directory.CreateDirectory(dir);
                }
                var rows = Entries.Select(x => new { Definition = x.Key, Entry = x.Value, Value = x.Value.GetSerializedValue() })
                    .Concat(OrphanedEntries.Select(x => new { Definition = x.Key, Entry = (ConfigEntryBase)null, x.Value }))
                    .ToList();
                using (var writer = new StreamWriter(ConfigFilePath, false, Utility.UTF8NoBom)) {
                    if (ownerMetadata != null) {
                        writer.WriteLine($"## Settings file was created by plugin {ownerMetadata.Name} v{ownerMetadata.Version}");
                        writer.WriteLine($"## Plugin GUID: {ownerMetadata.GUID}");
                        writer.WriteLine();
                    }
                    foreach (var section in rows.GroupBy(x => x.Definition.Section).OrderBy(x => x.Key)) {
                        writer.WriteLine($"[{section.Key}]");
                        foreach (var row in section) {
                            writer.WriteLine();
                            row.Entry?.WriteDescription(writer);
                            writer.WriteLine($"{row.Definition.Key} = {row.Value}");
                        }
                        writer.WriteLine();
                    }
                }
            }
        }

        [Obsolete("Use Bind instead")]
        public ConfigEntry<T> GetSetting<T>(ConfigDefinition configDefinition) => TryGetEntry<T>(configDefinition, out var entry) ? entry : null;

        [Obsolete("Use Bind instead")]
        public ConfigEntry<T> GetSetting<T>(string section, string key) => GetSetting<T>(new ConfigDefinition(section, key));

        public bool TryGetEntry<T>(ConfigDefinition key, out ConfigEntry<T> entry) {
            lock (ioLock) {
                if (Entries.TryGetValue(key, out var raw)) {
                    entry = (ConfigEntry<T>)raw;
                    return true;
                }
                entry = null;
                return false;
            }
        }

        public bool TryGetEntry<T>(string section, string key, out ConfigEntry<T> entry) => TryGetEntry(new ConfigDefinition(section, key), out entry);

        public ConfigEntry<T> Bind<T>(ConfigDefinition configDefinition, T defaultValue, ConfigDescription configDescription = null) {
            if (!TomlTypeConverter.CanConvert(typeof(T))) {
                throw new ArgumentException($"Type {typeof(T)} is not supported by the config system. Supported types: {string.Join(", ", TomlTypeConverter.GetSupportedTypes().Select(x => x.FullName).ToArray())}");
            }
            lock (ioLock) {
                if (Entries.TryGetValue(configDefinition, out var existing)) {
                    return (ConfigEntry<T>)existing;
                }
                var entry = new ConfigEntry<T>(this, configDefinition, defaultValue, configDescription);
                Entries[configDefinition] = entry;
                if (OrphanedEntries.TryGetValue(configDefinition, out var orphan)) {
                    entry.SetSerializedValue(orphan);
                    OrphanedEntries.Remove(configDefinition);
                }
                if (SaveOnConfigSet) {
                    Save();
                }
                return entry;
            }
        }

        public ConfigEntry<T> Bind<T>(string section, string key, T defaultValue, ConfigDescription configDescription = null) => Bind(new ConfigDefinition(section, key), defaultValue, configDescription);

        public ConfigEntry<T> Bind<T>(string section, string key, T defaultValue, string description) => Bind(new ConfigDefinition(section, key), defaultValue, new ConfigDescription(description));

        [Obsolete("Use Bind instead")]
        public ConfigEntry<T> AddSetting<T>(ConfigDefinition configDefinition, T defaultValue, ConfigDescription configDescription = null) => Bind(configDefinition, defaultValue, configDescription);

        [Obsolete("Use Bind instead")]
        public ConfigEntry<T> AddSetting<T>(string section, string key, T defaultValue, ConfigDescription configDescription = null) => Bind(section, key, defaultValue, configDescription);

        [Obsolete("Use Bind instead")]
        public ConfigEntry<T> AddSetting<T>(string section, string key, T defaultValue, string description) => Bind(section, key, defaultValue, description);

        [Obsolete("Use Bind instead")]
        public ConfigWrapper<T> Wrap<T>(string section, string key, string description = null, T defaultValue = default) => new ConfigWrapper<T>(Bind(section, key, defaultValue, new ConfigDescription(description ?? "")));

        [Obsolete("Use Bind instead")]
        public ConfigWrapper<T> Wrap<T>(ConfigDefinition configDefinition, T defaultValue = default) => new ConfigWrapper<T>(Bind(configDefinition, defaultValue));

        internal void OnSettingChanged(object sender, ConfigEntryBase changed) {
            if (changed == null) {
                throw new ArgumentNullException(nameof(changed));
            }
            if (SettingChanged == null && !SaveOnConfigSet) {
                return;
            }
            var args = new SettingChangedEventArgs(changed);
            foreach (var handler in SettingChanged?.GetInvocationList() ?? new Delegate[0]) {
                try {
                    ((EventHandler<SettingChangedEventArgs>)handler)(sender, args);
                } catch (Exception e) {
                    Logger.Log(LogLevel.Error, e);
                }
            }
            if (SaveOnConfigSet) {
                Save();
            }
        }

        public ConfigEntryBase this[ConfigDefinition key] {
            get {
                lock (ioLock) {
                    return Entries[key];
                }
            }
        }

        ConfigEntryBase IDictionary<ConfigDefinition, ConfigEntryBase>.this[ConfigDefinition key] {
            get => this[key];
            set => throw new InvalidOperationException("Directly setting a config entry is not supported");
        }

        public ConfigEntryBase this[string section, string key] => this[new ConfigDefinition(section, key)];

        public int Count {
            get {
                lock (ioLock) {
                    return Entries.Count;
                }
            }
        }

        public bool IsReadOnly => false;

        public ICollection<ConfigDefinition> Keys {
            get {
                lock (ioLock) {
                    return Entries.Keys.ToArray();
                }
            }
        }

        ICollection<ConfigEntryBase> IDictionary<ConfigDefinition, ConfigEntryBase>.Values {
            get {
                lock (ioLock) {
                    return Entries.Values.ToArray();
                }
            }
        }

        public IEnumerator<KeyValuePair<ConfigDefinition, ConfigEntryBase>> GetEnumerator() {
            lock (ioLock) {
                return Entries.ToList().GetEnumerator();
            }
        }

        IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();

        void ICollection<KeyValuePair<ConfigDefinition, ConfigEntryBase>>.Add(KeyValuePair<ConfigDefinition, ConfigEntryBase> item) {
            lock (ioLock) {
                Entries.Add(item.Key, item.Value);
            }
        }

        public bool Contains(KeyValuePair<ConfigDefinition, ConfigEntryBase> item) {
            lock (ioLock) {
                return ((ICollection<KeyValuePair<ConfigDefinition, ConfigEntryBase>>)Entries).Contains(item);
            }
        }

        void ICollection<KeyValuePair<ConfigDefinition, ConfigEntryBase>>.CopyTo(KeyValuePair<ConfigDefinition, ConfigEntryBase>[] array, int arrayIndex) {
            lock (ioLock) {
                ((ICollection<KeyValuePair<ConfigDefinition, ConfigEntryBase>>)Entries).CopyTo(array, arrayIndex);
            }
        }

        bool ICollection<KeyValuePair<ConfigDefinition, ConfigEntryBase>>.Remove(KeyValuePair<ConfigDefinition, ConfigEntryBase> item) {
            lock (ioLock) {
                return Entries.Remove(item.Key);
            }
        }

        public bool ContainsKey(ConfigDefinition key) {
            lock (ioLock) {
                return Entries.ContainsKey(key);
            }
        }

        public void Add(ConfigDefinition key, ConfigEntryBase value) => throw new InvalidOperationException("Directly adding a config entry is not supported");

        public bool Remove(ConfigDefinition key) {
            lock (ioLock) {
                return Entries.Remove(key);
            }
        }

        public void Clear() {
            lock (ioLock) {
                Entries.Clear();
            }
        }

        bool IDictionary<ConfigDefinition, ConfigEntryBase>.TryGetValue(ConfigDefinition key, out ConfigEntryBase value) {
            lock (ioLock) {
                return Entries.TryGetValue(key, out value);
            }
        }
    }
}
