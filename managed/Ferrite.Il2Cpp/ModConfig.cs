using System;
using System.Globalization;

namespace Ferrite {
    public sealed class ModConfig {
        readonly IntPtr handle;

        internal ModConfig(IntPtr handle) {
            this.handle = handle;
        }

        public event Action Changed;

        internal void RaiseChanged() => Changed?.Invoke();

        public string Get(string key, string fallback, string description = null) {
            var value = Native.ConfigGet(handle, key);
            if (value == null) {
                Set(key, fallback);
                value = fallback;
            }
            Describe(key, description);
            return value;
        }

        public bool Get(string key, bool fallback, string description = null) {
            var value = Native.ConfigGet(handle, key);
            if (value == null || !bool.TryParse(value, out var b)) {
                Set(key, fallback);
                b = fallback;
            }
            Describe(key, description);
            return b;
        }

        public long Get(string key, long fallback, string description = null) {
            var value = Native.ConfigGet(handle, key);
            if (value == null || !long.TryParse(value, NumberStyles.Integer, CultureInfo.InvariantCulture, out var l)) {
                Set(key, fallback);
                l = fallback;
            }
            Describe(key, description);
            return l;
        }

        public int Get(string key, int fallback, string description = null) => (int)Get(key, (long)fallback, description);

        public double Get(string key, double fallback, string description = null) {
            var value = Native.ConfigGet(handle, key);
            if (value == null || !double.TryParse(value, NumberStyles.Float, CultureInfo.InvariantCulture, out var d)) {
                Set(key, fallback);
                d = fallback;
            }
            Describe(key, description);
            return d;
        }

        public float Get(string key, float fallback, string description = null) => (float)Get(key, (double)fallback, description);

        public T GetEnum<T>(string key, T fallback, string description = null) where T : struct, Enum {
            var names = "One of: " + string.Join(", ", Enum.GetNames<T>());
            var value = Native.ConfigGet(handle, key);
            if (value == null || !Enum.TryParse<T>(value, true, out var e)) {
                e = fallback;
            }
            Set(key, e.ToString());
            Describe(key, string.IsNullOrEmpty(description) ? names : description + "\n" + names);
            return e;
        }

        void Describe(string key, string description) {
            if (!string.IsNullOrEmpty(description)) {
                Native.ConfigDescribe(handle, key, description);
            }
        }

        public void Set(string key, string value) => Native.ConfigSet(handle, key, value ?? "", 0);
        public void Set(string key, bool value) => Native.ConfigSet(handle, key, value ? "true" : "false", 1);
        public void Set(string key, long value) => Native.ConfigSet(handle, key, value.ToString(CultureInfo.InvariantCulture), 2);
        public void Set(string key, double value) => Native.ConfigSet(handle, key, value.ToString("R", CultureInfo.InvariantCulture), 3);
    }
}
