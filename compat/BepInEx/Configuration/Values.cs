using System;
using System.Linq;

namespace BepInEx.Configuration {
    public class ConfigDefinition : IEquatable<ConfigDefinition> {
        static readonly char[] invalidChars = { '=', '\n', '\t', '\\', '"', '\'', '[', ']' };

        public ConfigDefinition(string section, string key) {
            Check(section, nameof(section));
            Check(key, nameof(key));
            Section = section;
            Key = key;
        }

        [Obsolete("description argument is no longer used, put it in a ConfigDescription instead")]
        public ConfigDefinition(string section, string key, string description) : this(section, key) {
        }

        public string Section { get; }
        public string Key { get; }

        static void Check(string value, string name) {
            if (value == null) {
                throw new ArgumentNullException(name);
            }
            if (value != value.Trim()) {
                throw new ArgumentException("Cannot use whitespace characters at start or end of section and key names", name);
            }
            if (value.IndexOfAny(invalidChars) >= 0) {
                throw new ArgumentException("Can't use any of the following characters in section and key names: = \\n \\t \\ \" ' [ ]", name);
            }
        }

        public bool Equals(ConfigDefinition other) => !ReferenceEquals(other, null) && string.Equals(Key, other.Key) && string.Equals(Section, other.Section);

        public override bool Equals(object obj) => obj is ConfigDefinition other && Equals(other);

        public override int GetHashCode() {
            unchecked {
                return ((Key?.GetHashCode() ?? 0) * 397) ^ (Section?.GetHashCode() ?? 0);
            }
        }

        public static bool operator ==(ConfigDefinition left, ConfigDefinition right) => ReferenceEquals(left, null) ? ReferenceEquals(right, null) : left.Equals(right);

        public static bool operator !=(ConfigDefinition left, ConfigDefinition right) => !(left == right);

        public override string ToString() => Section + "." + Key;
    }

    public class ConfigDescription {
        public ConfigDescription(string description, AcceptableValueBase acceptableValues = null, params object[] tags) {
            AcceptableValues = acceptableValues;
            Tags = tags ?? new object[0];
            Description = description ?? throw new ArgumentNullException(nameof(description));
        }

        public string Description { get; }
        public AcceptableValueBase AcceptableValues { get; }
        public object[] Tags { get; }

        public static ConfigDescription Empty { get; } = new ConfigDescription("");
    }

    public abstract class AcceptableValueBase {
        protected AcceptableValueBase(Type valueType) {
            ValueType = valueType;
        }

        public Type ValueType { get; }

        public abstract object Clamp(object value);
        public abstract bool IsValid(object value);
        public abstract string ToDescriptionString();
    }

    public class AcceptableValueRange<T> : AcceptableValueBase where T : IComparable {
        public AcceptableValueRange(T minValue, T maxValue) : base(typeof(T)) {
            if (maxValue == null) {
                throw new ArgumentNullException(nameof(maxValue));
            }
            if (minValue == null) {
                throw new ArgumentNullException(nameof(minValue));
            }
            if (minValue.CompareTo(maxValue) >= 0) {
                throw new ArgumentException("minValue has to be lower than maxValue");
            }
            MinValue = minValue;
            MaxValue = maxValue;
        }

        public virtual T MinValue { get; }
        public virtual T MaxValue { get; }

        public override object Clamp(object value) {
            if (MinValue.CompareTo(value) > 0) {
                return MinValue;
            }
            if (MaxValue.CompareTo(value) < 0) {
                return MaxValue;
            }
            return value;
        }

        public override bool IsValid(object value) => MinValue.CompareTo(value) <= 0 && MaxValue.CompareTo(value) >= 0;

        public override string ToDescriptionString() => $"# Acceptable value range: From {MinValue} to {MaxValue}";
    }

    public class AcceptableValueList<T> : AcceptableValueBase where T : IEquatable<T> {
        public AcceptableValueList(params T[] acceptableValues) : base(typeof(T)) {
            if (acceptableValues == null) {
                throw new ArgumentNullException(nameof(acceptableValues));
            }
            if (acceptableValues.Length == 0) {
                throw new ArgumentException("At least one acceptable value is needed", nameof(acceptableValues));
            }
            AcceptableValues = acceptableValues;
        }

        public virtual T[] AcceptableValues { get; }

        public override object Clamp(object value) => IsValid(value) ? value : AcceptableValues[0];

        public override bool IsValid(object value) => value is T v && AcceptableValues.Any(x => x.Equals(v));

        public override string ToDescriptionString() => "# Acceptable values: " + string.Join(", ", AcceptableValues.Select(x => x.ToString()).ToArray());
    }

    public sealed class SettingChangedEventArgs : EventArgs {
        public SettingChangedEventArgs(ConfigEntryBase changedSetting) {
            ChangedSetting = changedSetting;
        }

        public ConfigEntryBase ChangedSetting { get; }
    }
}
