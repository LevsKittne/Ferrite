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
        internal static extern void ConfigDescribe(string id, string key, string description);

        [MethodImpl(MethodImplOptions.InternalCall)]
        internal static extern string ConfigList(string id);

        [MethodImpl(MethodImplOptions.InternalCall)]
        internal static extern string ConfigPoll();

        [MethodImpl(MethodImplOptions.InternalCall)]
        internal static extern bool Builtin(string line);

        [MethodImpl(MethodImplOptions.InternalCall)]
        internal static extern string BuiltinHelp();

        [MethodImpl(MethodImplOptions.InternalCall)]
        internal static extern string RootDir();

        [MethodImpl(MethodImplOptions.InternalCall)]
        internal static extern bool MenuOpen();

        [MethodImpl(MethodImplOptions.InternalCall)]
        internal static extern void ReportMods(string rows);

        [MethodImpl(MethodImplOptions.InternalCall)]
        internal static extern bool LoadPatched(string path, byte[] data);

        [MethodImpl(MethodImplOptions.InternalCall)]
        internal static extern long LogSequence();

        [MethodImpl(MethodImplOptions.InternalCall)]
        internal static extern string LogTail();
    }
}
