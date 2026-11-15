using System;
using System.Runtime.CompilerServices;

namespace Ferrite {
    static class Native {
        [MethodImpl(MethodImplOptions.InternalCall)]
        internal static extern void Log(int level, string source, string message);

        [MethodImpl(MethodImplOptions.InternalCall)]
        internal static extern string NextCommand();

        [MethodImpl(MethodImplOptions.InternalCall)]
        internal static extern Type[] DiscoverMods();

        [MethodImpl(MethodImplOptions.InternalCall)]
        internal static extern string Option(string key);

        [MethodImpl(MethodImplOptions.InternalCall)]
        internal static extern string ConfigGet(string id, string key);

        [MethodImpl(MethodImplOptions.InternalCall)]
        internal static extern void ConfigSet(string id, string key, string value, int kind);

        [MethodImpl(MethodImplOptions.InternalCall)]
        internal static extern string RootDir();
    }
}
