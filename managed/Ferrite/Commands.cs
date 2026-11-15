using System;
using System.Collections.Generic;
using System.Linq;

namespace Ferrite {
    public static class Commands {
        sealed class Entry {
            public string Name;
            public string Description;
            public string Usage;
            public Action<string[]> Handler;
            public Mod Owner;
        }

        static readonly Dictionary<string, Entry> entries = new Dictionary<string, Entry>(StringComparer.OrdinalIgnoreCase);
        static readonly Logger log = new Logger("Console");
        static readonly List<string> history = new List<string>();

        internal static Mod CurrentOwner;

        public static event Action<string> Printed;
        public static IReadOnlyList<string> History => history;

        public static void Register(string name, string description, Action<string[]> handler, string usage = "") {
            entries[name] = new Entry { Name = name, Description = description, Usage = usage ?? "", Handler = handler, Owner = CurrentOwner };
        }

        public static bool Unregister(string name) => entries.Remove(name);

        internal static void RemoveOwner(Mod owner) {
            foreach (var key in entries.Where(e => e.Value.Owner == owner).Select(e => e.Key).ToList()) {
                entries.Remove(key);
            }
        }

        public static void Print(string text) {
            log.Info(text);
            Printed?.Invoke(text);
        }

        public static IEnumerable<string> Names => entries.Keys.OrderBy(k => k, StringComparer.OrdinalIgnoreCase);

        public static bool Execute(string line) {
            if (string.IsNullOrWhiteSpace(line)) {
                return false;
            }
            line = line.Trim();
            if (history.Count == 0 || history[history.Count - 1] != line) {
                history.Add(line);
            }
            var args = line.Split(new[] { ' ' }, StringSplitOptions.RemoveEmptyEntries);
            if (!entries.TryGetValue(args[0], out var entry)) {
                if (Native.Builtin(line)) {
                    return true;
                }
                log.Warning($"unknown command {args[0]}, type help");
                return false;
            }
            try {
                entry.Handler(args.Skip(1).ToArray());
            } catch (Exception e) {
                log.Error($"{args[0]} failed: {e.Message}");
            }
            return true;
        }

        internal static void RegisterBuiltins() {
            Register("help", "Lists commands.", _ => {
                var lines = entries.Values.Select(e => $"{e.Name}{(e.Usage.Length > 0 ? " " + e.Usage : "")} - {e.Description}").ToList();
                lines.AddRange((Native.BuiltinHelp() ?? "").Split(new[] { '\n' }, StringSplitOptions.RemoveEmptyEntries));
                foreach (var l in lines.OrderBy(l => l, StringComparer.OrdinalIgnoreCase)) {
                    Print(l);
                }
            });
            Register("mods", "Lists loaded mods.", _ => {
                Print($"Ferrite {Runtime.Version}, {Runtime.Mods.Count} mod(s):");
                foreach (var m in Runtime.Mods) {
                    Print($"  {m.Info.Name} {m.Info.Version} ({m.Info.Id}){(m.Faulted ? " [faulted]" : "")}{(string.IsNullOrEmpty(m.Info.Author) ? "" : " by " + m.Info.Author)}");
                }
            });
            Register("reload", "Reloads a mod from its dll without restarting the game.", args => {
                if (args.Length == 0) {
                    Print("usage: reload <mod id>");
                    return;
                }
                Runtime.Reload(args[0]);
            }, "<mod id>");
        }

        internal static void Poll() {
            for (var i = 0; i < 16; i++) {
                var line = Native.NextCommand();
                if (line == null) {
                    return;
                }
                log.Info("> " + line);
                Execute(line);
            }
        }
    }
}
