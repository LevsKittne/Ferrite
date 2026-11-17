param(
    [Parameter(Mandatory)][string]$Process,
    [Parameter(Mandatory)][string]$Out,
    [int]$X = -1,
    [int]$Y = -1
)

Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class GameWindow {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
    public delegate bool EnumProc(IntPtr hwnd, IntPtr lparam);

    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc proc, IntPtr lparam);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassNameW(IntPtr hwnd, StringBuilder name, int max);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hwnd, out RECT rect);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hwnd, ref POINT p);
    [DllImport("user32.dll")] public static extern bool ScreenToClient(IntPtr hwnd, ref POINT p);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd, IntPtr after, int x, int y, int w, int h, uint flags);
    [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT p);
    [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr hwnd, uint msg, IntPtr wp, IntPtr lp);

    public static IntPtr Find(uint pid) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((h, l) => {
            uint p;
            GetWindowThreadProcessId(h, out p);
            if (p != pid || !IsWindowVisible(h)) {
                return true;
            }
            var name = new StringBuilder(64);
            GetClassNameW(h, name, 64);
            if (name.ToString() == "UnityWndClass") {
                found = h;
                return false;
            }
            return true;
        }, IntPtr.Zero);
        return found;
    }

    public static string Click(IntPtr game, int x, int y) {
        var p = new POINT { X = x, Y = y };
        ClientToScreen(game, ref p);
        var target = WindowFromPoint(p);
        var local = p;
        ScreenToClient(target, ref local);
        var lp = (IntPtr)((local.Y << 16) | (local.X & 0xFFFF));
        PostMessageW(target, 0x0200, IntPtr.Zero, lp);
        System.Threading.Thread.Sleep(80);
        PostMessageW(target, 0x0201, (IntPtr)1, lp);
        System.Threading.Thread.Sleep(80);
        PostMessageW(target, 0x0202, IntPtr.Zero, lp);
        var cls = new StringBuilder(64);
        GetClassNameW(target, cls, 64);
        return cls.ToString();
    }
}
"@

[GameWindow]::SetProcessDPIAware() | Out-Null
$target = Get-Process -Name $Process -ErrorAction Stop | Select-Object -First 1
$hwnd = [GameWindow]::Find([uint32]$target.Id)
if ($hwnd -eq [IntPtr]::Zero) { throw "no Unity window in $Process" }

$topmost = [IntPtr](-1)
$notopmost = [IntPtr](-2)
$flags = 0x0001 -bor 0x0002 -bor 0x0010
[GameWindow]::SetWindowPos($hwnd, $topmost, 0, 0, 0, 0, $flags) | Out-Null
try {
    Start-Sleep -Milliseconds 400
    if ($X -ge 0) {
        $hit = [GameWindow]::Click($hwnd, $X, $Y)
        Write-Host "clicked ${X},${Y} on $hit"
        Start-Sleep -Milliseconds 600
    }
    $rect = New-Object GameWindow+RECT
    [GameWindow]::GetClientRect($hwnd, [ref]$rect) | Out-Null
    $origin = New-Object GameWindow+POINT
    [GameWindow]::ClientToScreen($hwnd, [ref]$origin) | Out-Null
    $w = $rect.Right - $rect.Left
    $h = $rect.Bottom - $rect.Top
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($origin.X, $origin.Y, 0, 0, (New-Object System.Drawing.Size $w, $h))
    $g.Dispose()
    $bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    Write-Host "captured ${w}x${h} -> $Out"
} finally {
    [GameWindow]::SetWindowPos($hwnd, $notopmost, 0, 0, 0, 0, $flags) | Out-Null
}
