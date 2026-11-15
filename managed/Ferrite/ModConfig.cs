using System.Globalization;

namespace Ferrite {
    public sealed class ModConfig {
        readonly string id;

        internal ModConfig(string id) {
            this.id = id;
        }

        public string Get(string key, string fallback) {
            var value = Native.ConfigGet(id, key);
            if (value == null) {
                Set(key, fallback);
                return fallback;
            }
            return value;
        }

        public bool Get(string key, bool fallback) {
            var value = Native.ConfigGet(id, key);
            if (value != null && bool.TryParse(value, out var b)) {
                return b;
            }
            Set(key, fallback);
            return fallback;
        }

        public long Get(string key, long fallback) {
            var value = Native.ConfigGet(id, key);
            if (value != null && long.TryParse(value, NumberStyles.Integer, CultureInfo.InvariantCulture, out var l)) {
                return l;
            }
            Set(key, fallback);
            return fallback;
        }

        public double Get(string key, double fallback) {
            var value = Native.ConfigGet(id, key);
            if (value != null && double.TryParse(value, NumberStyles.Float, CultureInfo.InvariantCulture, out var d)) {
                return d;
            }
            Set(key, fallback);
            return fallback;
        }

        public void Set(string key, string value) => Native.ConfigSet(id, key, value ?? "", 0);
        public void Set(string key, bool value) => Native.ConfigSet(id, key, value ? "true" : "false", 1);
        public void Set(string key, long value) => Native.ConfigSet(id, key, value.ToString(CultureInfo.InvariantCulture), 2);
        public void Set(string key, double value) => Native.ConfigSet(id, key, value.ToString("R", CultureInfo.InvariantCulture), 3);
    }
}
