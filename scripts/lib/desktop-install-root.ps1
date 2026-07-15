Set-StrictMode -Version Latest

if (-not ("DeveCanonicalPath" -as [type])) {
    Add-Type @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;

public static class DeveCanonicalPath {
    private const uint ShareReadWriteDelete = 0x00000007;
    private const uint OpenExisting = 3;
    private const uint BackupSemantics = 0x02000000;

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern SafeFileHandle CreateFile(
        string fileName, uint desiredAccess, uint shareMode, IntPtr securityAttributes,
        uint creationDisposition, uint flagsAndAttributes, IntPtr templateFile
    );

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern uint GetFinalPathNameByHandle(
        SafeFileHandle file, StringBuilder path, uint pathLength, uint flags
    );

    public static string Resolve(string path) {
        using (SafeFileHandle file = CreateFile(
            path, 0, ShareReadWriteDelete, IntPtr.Zero, OpenExisting,
            BackupSemantics, IntPtr.Zero
        )) {
            if (file.IsInvalid) throw new Win32Exception(Marshal.GetLastWin32Error());
            StringBuilder buffer = new StringBuilder(512);
            uint length = GetFinalPathNameByHandle(file, buffer, (uint)buffer.Capacity, 0);
            if (length == 0) throw new Win32Exception(Marshal.GetLastWin32Error());
            if (length >= (uint)buffer.Capacity) {
                buffer = new StringBuilder(checked((int)length + 1));
                length = GetFinalPathNameByHandle(file, buffer, (uint)buffer.Capacity, 0);
                if (length == 0 || length >= (uint)buffer.Capacity) {
                    throw new Win32Exception(Marshal.GetLastWin32Error());
                }
            }
            return buffer.ToString();
        }
    }
}
'@
}

function Resolve-DeveCanonicalExistingPath {
    param([Parameter(Mandatory)][string]$Path)
    try {
        return [DeveCanonicalPath]::Resolve([IO.Path]::GetFullPath($Path))
    } catch {
        throw "failed to canonicalize existing path '$Path': $($_.Exception.Message)"
    }
}

function Test-DeveCanonicalDescendant {
    param(
        [Parameter(Mandatory)][string]$Root,
        [Parameter(Mandatory)][string]$Candidate
    )
    $rootPrefix = $Root.TrimEnd([char[]]@('\', '/')) + [IO.Path]::DirectorySeparatorChar
    return $Candidate.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)
}

function Assert-DeveDesktopInstallRoot {
    param(
        [Parameter(Mandatory)][string]$InstallRoot,
        [Parameter(Mandatory)][string]$DesktopBinary,
        [string]$SidecarName = "deve_cli.exe"
    )

    $rootPath = (Resolve-Path -LiteralPath $InstallRoot -ErrorAction Stop).Path
    if (-not (Test-Path -LiteralPath $rootPath -PathType Container)) {
        throw "InstallRoot must be an existing directory"
    }
    $desktopPath = (Resolve-Path -LiteralPath $DesktopBinary -ErrorAction Stop).Path
    if (-not (Test-Path -LiteralPath $desktopPath -PathType Leaf)) {
        throw "DesktopBinary must be an existing file"
    }

    $canonicalRoot = Resolve-DeveCanonicalExistingPath $rootPath
    $canonicalDesktop = Resolve-DeveCanonicalExistingPath $desktopPath
    if (-not (Test-DeveCanonicalDescendant $canonicalRoot $canonicalDesktop)) {
        throw "DesktopBinary must be a real child of InstallRoot"
    }

    $markerPath = Join-Path $rootPath ".deve-desktop-install-root.json"
    if (-not (Test-Path -LiteralPath $markerPath -PathType Leaf)) {
        throw "install marker is missing from InstallRoot"
    }
    $canonicalMarker = Resolve-DeveCanonicalExistingPath $markerPath
    if (-not (Test-DeveCanonicalDescendant $canonicalRoot $canonicalMarker)) {
        throw "install marker must be a real child of InstallRoot"
    }
    try {
        $marker = Get-Content -Raw -LiteralPath $markerPath | ConvertFrom-Json
    } catch {
        throw "install marker is not valid JSON"
    }
    $names = @($marker.PSObject.Properties.Name)
    if (
        $names.Count -ne 2 -or $names -cnotcontains "schema" -or
        $names -cnotcontains "marker" -or
        (($marker.schema -isnot [int]) -and ($marker.schema -isnot [long])) -or
        [long]$marker.schema -ne 1
    ) {
        throw "install marker schema is invalid"
    }
    if ($marker.marker -isnot [string] -or $marker.marker -cne "deve-desktop-remote-browser-smoke") {
        throw "install marker value is invalid"
    }

    $sidecarPath = Join-Path (Split-Path -Parent $desktopPath) $SidecarName
    if (-not (Test-Path -LiteralPath $sidecarPath -PathType Leaf)) {
        throw "$SidecarName sidecar is missing next to installed Desktop binary"
    }
    $canonicalSidecar = Resolve-DeveCanonicalExistingPath $sidecarPath
    if (-not (Test-DeveCanonicalDescendant $canonicalRoot $canonicalSidecar)) {
        throw "$SidecarName sidecar must be a real child of InstallRoot"
    }

    [pscustomobject]@{
        InstallRoot = $rootPath
        DesktopBinary = $desktopPath
        Sidecar = $sidecarPath
        Marker = $markerPath
    }
}
