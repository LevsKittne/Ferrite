using System;
using System.Collections.Generic;
using System.Linq;
using BepInEx.Logging;
using UnityEngine;

namespace BepInEx.Configuration {
    public struct KeyboardShortcut {
        public static readonly KeyboardShortcut Empty = new KeyboardShortcut();

        public static readonly IEnumerable<KeyCode> AllKeyCodes = (KeyCode[])Enum.GetValues(typeof(KeyCode));

        static readonly KeyCode[] modifierKeys = {
            KeyCode.LeftShift, KeyCode.RightShift, KeyCode.LeftControl, KeyCode.RightControl,
            KeyCode.LeftAlt, KeyCode.RightAlt, KeyCode.LeftCommand, KeyCode.RightCommand, KeyCode.AltGr,
        };

        readonly KeyCode[] allKeys;

        public KeyboardShortcut(KeyCode mainKey, params KeyCode[] modifiers) : this(new[] { mainKey }.Concat(modifiers ?? new KeyCode[0]).ToArray()) {
            if (mainKey == KeyCode.None && modifiers != null && modifiers.Any()) {
                throw new ArgumentException($"Can't set {nameof(mainKey)} to KeyCode.None if there are any {nameof(modifiers)}");
            }
        }

        KeyboardShortcut(KeyCode[] keys) {
            allKeys = Sanitize(keys);
        }

        static KeyCode[] Sanitize(KeyCode[] keys) {
            if (keys.Length == 0 || keys[0] == KeyCode.None) {
                return new[] { KeyCode.None };
            }
            return new[] { keys[0] }.Concat(keys.Skip(1).Distinct().Where(x => x != keys[0]).OrderBy(x => (int)x)).ToArray();
        }

        public KeyCode MainKey => allKeys != null && allKeys.Length > 0 ? allKeys[0] : KeyCode.None;

        public IEnumerable<KeyCode> Modifiers => allKeys?.Skip(1) ?? Enumerable.Empty<KeyCode>();

        public static KeyboardShortcut Deserialize(string str) {
            try {
                var parts = str.Split(new[] { ' ', '+', ',', ';', '|' }, StringSplitOptions.RemoveEmptyEntries)
                    .Select(x => (KeyCode)Enum.Parse(typeof(KeyCode), x))
                    .ToArray();
                return new KeyboardShortcut(parts);
            } catch (SystemException e) {
                Logging.Logger.Log(LogLevel.Error, "Failed to read keybind from settings: " + e.Message);
                return Empty;
            }
        }

        public string Serialize() => allKeys == null ? string.Empty : string.Join(" + ", allKeys.Select(x => x.ToString()).ToArray());

        public bool IsDown() {
            var main = MainKey;
            return main != KeyCode.None && UnityInput.Current.GetKeyDown(main) && ModifiersHeld();
        }

        public bool IsPressed() {
            var main = MainKey;
            return main != KeyCode.None && UnityInput.Current.GetKey(main) && ModifiersHeld();
        }

        public bool IsUp() {
            var main = MainKey;
            return main != KeyCode.None && UnityInput.Current.GetKeyUp(main) && ModifiersHeld();
        }

        bool ModifiersHeld() {
            var keys = allKeys;
            var main = MainKey;
            var input = UnityInput.Current;
            if (keys.Skip(1).Any(k => !input.GetKey(k))) {
                return false;
            }
            return modifierKeys.All(k => k == main || keys.Contains(k) || !input.GetKey(k));
        }

        public override string ToString() => MainKey == KeyCode.None ? "Not set" : string.Join(" + ", allKeys.Select(c => c.ToString()).ToArray());

        public override bool Equals(object obj) => obj is KeyboardShortcut other && MainKey == other.MainKey && Modifiers.SequenceEqual(other.Modifiers);

        public override int GetHashCode() => MainKey == KeyCode.None ? 0 : allKeys.Aggregate(allKeys.Length, (acc, k) => unchecked(acc * 31 + (int)k));
    }
}
