Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class DeveNativeMenuAutomation {
    [DllImport("user32.dll")]
    public static extern IntPtr GetMenu(IntPtr window);

    [DllImport("user32.dll")]
    public static extern int GetMenuItemCount(IntPtr menu);

    [DllImport("user32.dll")]
    public static extern IntPtr GetSubMenu(IntPtr menu, int position);

    [DllImport("user32.dll")]
    public static extern uint GetMenuItemID(IntPtr menu, int position);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetMenuString(
        IntPtr menu,
        uint item,
        StringBuilder text,
        int textLength,
        uint flags
    );

    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool PostMessage(
        IntPtr window,
        uint message,
        IntPtr wParam,
        IntPtr lParam
    );
}
'@

function Find-NativeMenuCommandId($Menu, $Label) {
    $itemCount = [DeveNativeMenuAutomation]::GetMenuItemCount($Menu)
    for ($position = 0; $position -lt $itemCount; $position++) {
        $text = [System.Text.StringBuilder]::new(256)
        [void][DeveNativeMenuAutomation]::GetMenuString(
            $Menu,
            [uint32]$position,
            $text,
            $text.Capacity,
            0x400 # MF_BYPOSITION
        )
        if ($text.ToString() -eq $Label) {
            $commandId = [DeveNativeMenuAutomation]::GetMenuItemID($Menu, $position)
            if ($commandId -ne [uint32]::MaxValue) { return $commandId }
        }
        $submenu = [DeveNativeMenuAutomation]::GetSubMenu($Menu, $position)
        if ($submenu -ne [IntPtr]::Zero) {
            $commandId = Find-NativeMenuCommandId $submenu $Label
            if ($null -ne $commandId) { return $commandId }
        }
    }
    return $null
}

function Invoke-UseLocalBackendMenu($Process, $Deadline) {
    while ([DateTime]::UtcNow -lt $Deadline) {
        $Process.Refresh()
        if ($Process.MainWindowHandle -eq 0) {
            Start-Sleep -Milliseconds 250
            continue
        }
        $menu = [DeveNativeMenuAutomation]::GetMenu($Process.MainWindowHandle)
        if ($menu -eq [IntPtr]::Zero) {
            Start-Sleep -Milliseconds 250
            continue
        }
        $commandId = Find-NativeMenuCommandId $menu "Use Local Backend"
        if ($null -ne $commandId) {
            $posted = [DeveNativeMenuAutomation]::PostMessage(
                $Process.MainWindowHandle,
                0x0111, # WM_COMMAND
                [IntPtr][int64]$commandId,
                [IntPtr]::Zero
            )
            if (-not $posted) { Fail "failed to dispatch the native Use Local Backend menu" }
            return
        }
        Start-Sleep -Milliseconds 250
    }
    Fail "native Use Local Backend menu item was not invokable"
}

function Find-ReplacementDesktop($ExecutablePath, $OldPid, $NotBefore, $Deadline) {
    $expectedPath = [System.IO.Path]::GetFullPath($ExecutablePath)
    while ([DateTime]::UtcNow -lt $Deadline) {
        $matches = @(
            Get-CimInstance Win32_Process |
                Where-Object {
                    $_.ProcessId -ne $OldPid -and $null -ne $_.ExecutablePath -and
                    [System.IO.Path]::GetFullPath($_.ExecutablePath).Equals(
                        $expectedPath, [System.StringComparison]::OrdinalIgnoreCase
                    )
                }
        )
        if ($matches.Count -gt 1) {
            Fail "Desktop restart produced multiple replacement processes: $($matches.ProcessId -join ',')"
        }
        if ($matches.Count -eq 1) {
            $match = $matches[0]
            $createdAt = ([DateTime]$match.CreationDate).ToUniversalTime()
            if ($createdAt -lt $NotBefore) {
                Fail "Desktop replacement predates the native local-backend transition"
            }
            return Get-Process -Id $match.ProcessId -ErrorAction Stop
        }
        Start-Sleep -Milliseconds 250
    }
    Fail "Desktop did not restart after native local-backend transition"
}
