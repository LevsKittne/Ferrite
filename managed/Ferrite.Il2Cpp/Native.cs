using System;
using System.Runtime.InteropServices;
using System.Text;

namespace Ferrite {
    [StructLayout(LayoutKind.Sequential)]
    unsafe struct Api {
        public uint Abi;
        public delegate* unmanaged<IntPtr, int, byte*, nuint, void> Log;
        public delegate* unmanaged<byte*, IntPtr> Symbol;
        public delegate* unmanaged<IntPtr, IntPtr, IntPtr> Hook;
        public delegate* unmanaged<IntPtr, byte*, nuint, byte*, nuint, IntPtr, void> Command;
        public delegate* unmanaged<IntPtr, byte*, nuint, byte*, nuint, nint> ConfigGet;
        public delegate* unmanaged<IntPtr, byte*, nuint, byte*, nuint, int, void> ConfigSet;
        public delegate* unmanaged<IntPtr, byte*, nuint, byte*, nuint, void> ConfigDescribe;
        public delegate* unmanaged<IntPtr, IntPtr, IntPtr, IntPtr> HookOwned;
        public delegate* unmanaged<IntPtr, IntPtr, byte> Unhook;
        public delegate* unmanaged<IntPtr, IntPtr, IntPtr, IntPtr> PatchPointer;
        public delegate* unmanaged<byte*, nuint, void> Execute;
        public delegate* unmanaged<IntPtr, int, byte*, nuint, nint> Directory;
    }

    static unsafe class Native {
        internal static Api* Api;

        internal static byte[] Utf8(string s) => Encoding.UTF8.GetBytes(s ?? "");

        internal static void Log(IntPtr handle, int level, string message) {
            var m = Utf8(message);
            fixed (byte* p = m) {
                Api->Log(handle, level, p, (nuint)m.Length);
            }
        }

        internal static IntPtr Symbol(string name) {
            var n = Utf8(name + "\0");
            fixed (byte* p = n) {
                return Api->Symbol(p);
            }
        }

        internal static void Command(IntPtr handle, string name, string description, IntPtr callback) {
            var n = Utf8(name);
            var d = Utf8(description);
            fixed (byte* np = n)
            fixed (byte* dp = d) {
                Api->Command(handle, np, (nuint)n.Length, dp, (nuint)d.Length, callback);
            }
        }

        internal static string ConfigGet(IntPtr handle, string key) {
            var k = Utf8(key);
            var buffer = new byte[512];
            while (true) {
                nint n;
                fixed (byte* kp = k)
                fixed (byte* bp = buffer) {
                    n = Api->ConfigGet(handle, kp, (nuint)k.Length, bp, (nuint)buffer.Length);
                }
                if (n < 0) {
                    return null;
                }
                if (n <= buffer.Length) {
                    return Encoding.UTF8.GetString(buffer, 0, (int)n);
                }
                buffer = new byte[n];
            }
        }

        internal static void ConfigSet(IntPtr handle, string key, string value, int kind) {
            var k = Utf8(key);
            var v = Utf8(value);
            fixed (byte* kp = k)
            fixed (byte* vp = v) {
                Api->ConfigSet(handle, kp, (nuint)k.Length, vp, (nuint)v.Length, kind);
            }
        }

        internal static void ConfigDescribe(IntPtr handle, string key, string description) {
            var k = Utf8(key);
            var d = Utf8(description);
            fixed (byte* kp = k)
            fixed (byte* dp = d) {
                Api->ConfigDescribe(handle, kp, (nuint)k.Length, dp, (nuint)d.Length);
            }
        }

        internal static void Execute(string line) {
            var l = Utf8(line);
            fixed (byte* p = l) {
                Api->Execute(p, (nuint)l.Length);
            }
        }

        internal static string Directory(IntPtr handle, int kind) {
            var buffer = new byte[1024];
            while (true) {
                nint n;
                fixed (byte* bp = buffer) {
                    n = Api->Directory(handle, kind, bp, (nuint)buffer.Length);
                }
                if (n < 0) {
                    return null;
                }
                if (n <= buffer.Length) {
                    return Encoding.UTF8.GetString(buffer, 0, (int)n);
                }
                buffer = new byte[n];
            }
        }
    }
}
