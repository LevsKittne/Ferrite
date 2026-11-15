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
        }

        static readonly Dictionary<string, Entry> entries = new Dictionary<string, Entry>(StringComparer.OrdinalIgnoreCase);
        static readonly Logger log = new Logger("Console");

        public static void Register(string name, string description, Action<string[]> handler, string usage = "") {
            entries[name] = new Entry { Name = name, Description = description, Usage = usage ?? "", Handler = handler };
        }

        public static void Print(string text) => log.Info(text);

        public static bool Execute(string line) {
            if (string.IsNullOrWhiteSpace(line)) {
                return false;
            }
            var args = line.Trim().Split(new[] { ' ' }, StringSplitOptions.RemoveEmptyEntries);
            if (!entries.TryGetValue(args[0], out var entry)) {
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
                foreach (var e in entries.Values.OrderBy(e => e.Name, StringComparer.OrdinalIgnoreCase)) {
                    Print($"{e.Name}{(e.Usage.Length > 0 ? " " + e.Usage : "")} - {e.Description}");
                }
            });
            Register("mods", "Lists loaded mods.", _ => {
                Print($"Ferrite {Runtime.Version}, {Runtime.Mods.Count} mod(s):");
                foreach (var m in Runtime.Mods) {
                    Print($"  {m.Info.Name} {m.Info.Version} ({m.Info.Id}){(m.Faulted ? " [faulted]" : "")}");
                }
            });
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
