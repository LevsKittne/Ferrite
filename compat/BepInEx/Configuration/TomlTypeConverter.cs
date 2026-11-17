using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;
using UnityEngine;

namespace BepInEx.Configuration {
    public class TypeConverter {
        public Func<object, Type, string> ConvertToString { get; set; }
        public Func<string, Type, object> ConvertToObject { get; set; }
    }

    public static class TomlTypeConverter {
        static readonly Dictionary<Type, TypeConverter> converters = Defaults();

        static Dictionary<Type, TypeConverter> Defaults() {
            var map = new Dictionary<Type, TypeConverter> {
                [typeof(string)] = new TypeConverter {
                    ConvertToString = (o, t) => Escape((string)o),
                    ConvertToObject = (s, t) => Regex.IsMatch(s, @"^""?\w:\\(?!\\)(?!.+\\\\)") ? s : Unescape(s),
                },
                [typeof(bool)] = new TypeConverter {
                    ConvertToString = (o, t) => o.ToString().ToLowerInvariant(),
                    ConvertToObject = (s, t) => bool.Parse(s),
                },
                [typeof(Enum)] = new TypeConverter {
                    ConvertToString = (o, t) => o.ToString(),
                    ConvertToObject = (s, t) => Enum.Parse(t, s, true),
                },
                [typeof(Color)] = new TypeConverter {
                    ConvertToString = (o, t) => ColorUtility.ToHtmlStringRGBA((Color)o),
                    ConvertToObject = (s, t) => ColorUtility.TryParseHtmlString("#" + s.Trim('#', ' '), out var c) ? c : throw new FormatException("Invalid color string, expected hex #RRGGBBAA"),
                },
                [typeof(Vector2)] = Json(o => { var v = (Vector2)o; return new[] { v.x, v.y }; }, f => new Vector2(f[0], f[1]), "x", "y"),
                [typeof(Vector3)] = Json(o => { var v = (Vector3)o; return new[] { v.x, v.y, v.z }; }, f => new Vector3(f[0], f[1], f[2]), "x", "y", "z"),
                [typeof(Vector4)] = Json(o => { var v = (Vector4)o; return new[] { v.x, v.y, v.z, v.w }; }, f => new Vector4(f[0], f[1], f[2], f[3]), "x", "y", "z", "w"),
                [typeof(Quaternion)] = Json(o => { var v = (Quaternion)o; return new[] { v.x, v.y, v.z, v.w }; }, f => new Quaternion(f[0], f[1], f[2], f[3]), "x", "y", "z", "w"),
                [typeof(KeyboardShortcut)] = new TypeConverter {
                    ConvertToString = (o, t) => ((KeyboardShortcut)o).Serialize(),
                    ConvertToObject = (s, t) => KeyboardShortcut.Deserialize(s),
                },
            };
            var number = new TypeConverter {
                ConvertToString = (o, t) => Convert.ToString(o, CultureInfo.InvariantCulture),
                ConvertToObject = (s, t) => Convert.ChangeType(s, t, CultureInfo.InvariantCulture),
            };
            foreach (var t in new[] { typeof(byte), typeof(sbyte), typeof(short), typeof(ushort), typeof(int), typeof(uint), typeof(long), typeof(ulong), typeof(decimal) }) {
                map[t] = number;
            }
            map[typeof(float)] = new TypeConverter {
                ConvertToString = (o, t) => ((float)o).ToString(NumberFormatInfo.InvariantInfo),
                ConvertToObject = (s, t) => float.Parse(s, NumberFormatInfo.InvariantInfo),
            };
            map[typeof(double)] = new TypeConverter {
                ConvertToString = (o, t) => ((double)o).ToString(NumberFormatInfo.InvariantInfo),
                ConvertToObject = (s, t) => double.Parse(s, NumberFormatInfo.InvariantInfo),
            };
            return map;
        }

        static TypeConverter Json(Func<object, float[]> split, Func<float[], object> join, params string[] names) => new TypeConverter {
            ConvertToString = (o, t) => "{" + string.Join(",", split(o).Select((f, i) => $"\"{names[i]}\":{f.ToString("R", CultureInfo.InvariantCulture)}").ToArray()) + "}",
            ConvertToObject = (s, t) => {
                var values = new float[names.Length];
                foreach (Match m in Regex.Matches(s, @"""(\w+)""\s*:\s*([-+0-9.eE]+)")) {
                    var i = Array.IndexOf(names, m.Groups[1].Value);
                    if (i >= 0) {
                        values[i] = float.Parse(m.Groups[2].Value, CultureInfo.InvariantCulture);
                    }
                }
                return join(values);
            },
        };

        public static string ConvertToString(object value, Type valueType) {
            var conv = GetConverter(valueType) ?? throw new InvalidOperationException($"Cannot convert from type {valueType}");
            return conv.ConvertToString(value, valueType);
        }

        public static T ConvertToValue<T>(string value) => (T)ConvertToValue(value, typeof(T));

        public static object ConvertToValue(string value, Type valueType) {
            var conv = GetConverter(valueType) ?? throw new InvalidOperationException($"Cannot convert to type {valueType.Name}");
            return conv.ConvertToObject(value, valueType);
        }

        public static TypeConverter GetConverter(Type valueType) {
            if (valueType == null) {
                throw new ArgumentNullException(nameof(valueType));
            }
            if (valueType.IsEnum) {
                return converters[typeof(Enum)];
            }
            converters.TryGetValue(valueType, out var result);
            return result;
        }

        public static bool AddConverter(Type type, TypeConverter converter) {
            if (type == null) {
                throw new ArgumentNullException(nameof(type));
            }
            if (converter == null) {
                throw new ArgumentNullException(nameof(converter));
            }
            if (CanConvert(type)) {
                Logging.Logger.Log(Logging.LogLevel.Warning, "Tried to add a TomlConverter when one already exists for type " + type.FullName);
                return false;
            }
            converters.Add(type, converter);
            return true;
        }

        public static bool CanConvert(Type type) => GetConverter(type) != null;

        public static IEnumerable<Type> GetSupportedTypes() => converters.Keys;

        static string Escape(string text) {
            if (string.IsNullOrEmpty(text)) {
                return string.Empty;
            }
            var sb = new StringBuilder(text.Length + 2);
            foreach (var c in text) {
                switch (c) {
                    case '\0':
                        sb.Append(@"\0");
                        break;
                    case '\a':
                        sb.Append(@"\a");
                        break;
                    case '\b':
                        sb.Append(@"\b");
                        break;
                    case '\t':
                        sb.Append(@"\t");
                        break;
                    case '\n':
                        sb.Append(@"\n");
                        break;
                    case '\v':
                        sb.Append(@"\v");
                        break;
                    case '\f':
                        sb.Append(@"\f");
                        break;
                    case '\r':
                        sb.Append(@"\r");
                        break;
                    case '\'':
                        sb.Append(@"\'");
                        break;
                    case '\\':
                        sb.Append(@"\\");
                        break;
                    case '"':
                        sb.Append(@"\""");
                        break;
                    default:
                        sb.Append(c);
                        break;
                }
            }
            return sb.ToString();
        }

        static string Unescape(string text) {
            if (string.IsNullOrEmpty(text)) {
                return string.Empty;
            }
            var sb = new StringBuilder(text.Length);
            for (var i = 0; i < text.Length;) {
                var j = text.IndexOf('\\', i);
                if (j < 0 || j == text.Length - 1) {
                    j = text.Length;
                }
                sb.Append(text, i, j - i);
                if (j >= text.Length) {
                    break;
                }
                var c = text[j + 1];
                switch (c) {
                    case '0':
                        sb.Append('\0');
                        break;
                    case 'a':
                        sb.Append('\a');
                        break;
                    case 'b':
                        sb.Append('\b');
                        break;
                    case 't':
                        sb.Append('\t');
                        break;
                    case 'n':
                        sb.Append('\n');
                        break;
                    case 'v':
                        sb.Append('\v');
                        break;
                    case 'f':
                        sb.Append('\f');
                        break;
                    case 'r':
                        sb.Append('\r');
                        break;
                    case '\'':
                        sb.Append('\'');
                        break;
                    case '"':
                        sb.Append('"');
                        break;
                    case '\\':
                        sb.Append('\\');
                        break;
                    default:
                        sb.Append('\\').Append(c);
                        break;
                }
                i = j + 2;
            }
            return sb.ToString();
        }
    }
}
