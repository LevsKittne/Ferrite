using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;

namespace Ferrite {
    public static unsafe class Commands {
        sealed class Entry {
            public Action<string[]> Handler;
            public Mod Owner;
        }

        static readonly Dictionary<string, Entry> entries = new Dictionary<string, Entry>(StringComparer.OrdinalIgnoreCase);

        public static void Register(string name, string description, Action<string[]> handler) {
            var owner = Host.Current ?? throw new InvalidOperationException("Commands.Register outside a mod callback, use Mod.Command");
            Register(owner, name, description, handler);
        }

        internal static void Register(Mod owner, string name, string description, Action<string[]> handler) {
            if (string.IsNullOrWhiteSpace(name) || handler == null) {
                throw new ArgumentException("command needs a name and a handler");
            }
            entries[name.ToLowerInvariant()] = new Entry { Handler = handler, Owner = owner };
            Native.Command(owner.Handle, name, description ?? "", (IntPtr)(delegate* unmanaged<byte*, nuint, byte*, nuint, void>)&Dispatch);
        }

        public static void Execute(string line) => Native.Execute(line);

        internal static void RemoveOwner(Mod owner) {
            foreach (var key in entries.Where(e => e.Value.Owner == owner).Select(e => e.Key).ToList()) {
                entries.Remove(key);
            }
        }

        [UnmanagedCallersOnly]
        static void Dispatch(byte* name, nuint nameLength, byte* args, nuint argsLength) {
            try {
                var key = Encoding.UTF8.GetString(name, (int)nameLength);
                var line = Encoding.UTF8.GetString(args, (int)argsLength);
                if (!entries.TryGetValue(key, out var entry)) {
                    return;
                }
                var parts = line.Split(' ', StringSplitOptions.RemoveEmptyEntries);
                Host.Run(entry.Owner, "command " + key, () => entry.Handler(parts));
            } catch (Exception) {
            }
        }
    }
}
