param([Parameter(Mandatory)][string]$Process, [Parameter(Mandatory)][string]$Command)

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public static class ConsoleInput {
    [StructLayout(LayoutKind.Explicit, CharSet = CharSet.Unicode)]
    struct KeyEventRecord {
        [FieldOffset(0)] public ushort EventType;
        [FieldOffset(4)] public int KeyDown;
        [FieldOffset(8)] public ushort RepeatCount;
        [FieldOffset(10)] public ushort VirtualKeyCode;
        [FieldOffset(12)] public ushort VirtualScanCode;
        [FieldOffset(14)] public char UnicodeChar;
        [FieldOffset(16)] public uint ControlKeyState;
    }

    [DllImport("kernel32.dll", SetLastError = true)] static extern bool FreeConsole();
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool AttachConsole(uint pid);
    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    static extern IntPtr CreateFileW(string name, uint access, uint share, IntPtr sec, uint disposition, uint flags, IntPtr template);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern bool WriteConsoleInputW(IntPtr handle, KeyEventRecord[] records, uint length, out uint written);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);

    public static bool Send(uint pid, string text) {
        FreeConsole();
        if (!AttachConsole(pid)) {
            return false;
        }
        var input = CreateFileW("CONIN$", 0xC0000000, 3, IntPtr.Zero, 3, 0, IntPtr.Zero);
        var chars = (text + "\r").ToCharArray();
        var records = new KeyEventRecord[chars.Length * 2];
        for (var i = 0; i < chars.Length; i++) {
            var vk = chars[i] == '\r' ? (ushort)0x0D : (ushort)0;
            records[i * 2] = new KeyEventRecord { EventType = 1, KeyDown = 1, RepeatCount = 1, VirtualKeyCode = vk, UnicodeChar = chars[i] };
            records[i * 2 + 1] = new KeyEventRecord { EventType = 1, KeyDown = 0, RepeatCount = 1, VirtualKeyCode = vk, UnicodeChar = chars[i] };
        }
        uint written;
        var ok = WriteConsoleInputW(input, records, (uint)records.Length, out written);
        CloseHandle(input);
        FreeConsole();
        return ok;
    }
}
"@

$target = Get-Process -Name $Process -ErrorAction Stop | Select-Object -First 1
if ([ConsoleInput]::Send([uint32]$target.Id, $Command)) { "sent: $Command" } else { "failed" }
