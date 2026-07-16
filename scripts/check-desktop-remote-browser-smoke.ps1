param(
    [Parameter(Mandatory = $true)]
    [string]$DesktopBinary,
    [Parameter(Mandatory = $true)]
    [string]$InstallRoot,
    [Parameter(Mandatory = $true)]
    [string]$WorkRoot,
    [Parameter(Mandatory = $true)]
    [string]$RemoteHttpsOrigin,
    [Parameter(Mandatory = $true)]
    [string]$Username,
    [string]$Password = $env:DEVE_DESKTOP_REMOTE_PASSWORD,
    [int]$StartupTimeoutSeconds = 60,
    [int]$ExitTimeoutSeconds = 15,
    [int]$NpmTimeoutSeconds = 180
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "lib\playwright-core.ps1")
. (Join-Path $PSScriptRoot "lib\webview2-cdp.ps1")

if ([string]::IsNullOrEmpty($Password)) {
    throw "desktop-remote-browser-smoke: Password or DEVE_DESKTOP_REMOTE_PASSWORD is required"
}

function Fail($Message) { throw "desktop-remote-browser-smoke: $Message" }

function Get-InstalledSidecars($ExecutablePath) {
    $expectedPath = [System.IO.Path]::GetFullPath($ExecutablePath)
    @(Get-CimInstance Win32_Process -Filter "Name = 'deve_cli.exe'" | Where-Object {
        $null -ne $_.ExecutablePath -and
        [System.IO.Path]::GetFullPath($_.ExecutablePath).Equals(
            $expectedPath, [System.StringComparison]::OrdinalIgnoreCase
        ) -and $_.CommandLine -match "serve" -and $_.CommandLine -match "--native-loopback"
    })
}

function Stop-ProcessIfAlive($ProcessId) {
    if ($null -ne $ProcessId -and $null -ne (Get-Process -Id $ProcessId -ErrorAction SilentlyContinue)) {
        Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
    }
}

. (Join-Path $PSScriptRoot "lib/desktop-install-root.ps1")

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

function Find-ReplacementDesktop($ExecutablePath, $OldPid, $Deadline) {
    $expectedPath = [System.IO.Path]::GetFullPath($ExecutablePath)
    while ([DateTime]::UtcNow -lt $Deadline) {
        $match = Get-CimInstance Win32_Process | Where-Object {
            $_.ProcessId -ne $OldPid -and $null -ne $_.ExecutablePath -and
            [System.IO.Path]::GetFullPath($_.ExecutablePath).Equals(
                $expectedPath, [System.StringComparison]::OrdinalIgnoreCase
            )
        } | Select-Object -First 1
        if ($null -ne $match) { return Get-Process -Id $match.ProcessId }
        Start-Sleep -Milliseconds 250
    }
    Fail "Desktop did not restart after native local-backend transition"
}

$remote = [Uri]$RemoteHttpsOrigin
if (
    -not $remote.IsAbsoluteUri -or $remote.Scheme -ne "https" -or
    [string]::IsNullOrWhiteSpace($remote.Host) -or $remote.AbsolutePath -ne "/" -or
    -not [string]::IsNullOrEmpty($remote.UserInfo) -or
    -not [string]::IsNullOrEmpty($remote.Query) -or
    -not [string]::IsNullOrEmpty($remote.Fragment)
) {
    Fail "RemoteHttpsOrigin must be an HTTPS origin"
}
try {
    $install = Assert-DeveDesktopInstallRoot -InstallRoot $InstallRoot -DesktopBinary $DesktopBinary
} catch {
    Fail $_.Exception.Message
}
$desktopPath = $install.DesktopBinary
$installRootPath = $install.InstallRoot

$workRootPath = [System.IO.Path]::GetFullPath($WorkRoot)
$sidecarPath = $install.Sidecar
if (@(Get-InstalledSidecars $sidecarPath).Count -ne 0) {
    Fail "installed sidecar path already has running native-loopback processes"
}
$preexistingDesktop = @(Get-CimInstance Win32_Process |
    Where-Object {
        $null -ne $_.ExecutablePath -and
        [System.IO.Path]::GetFullPath($_.ExecutablePath).Equals(
            $desktopPath, [System.StringComparison]::OrdinalIgnoreCase
        )
    })
if ($preexistingDesktop.Count -ne 0) {
    Fail "isolated installed Desktop binary is already running"
}

$runId = "{0}-{1}" -f ([DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()), $PID
$runRoot = Join-Path $workRootPath "remote-browser-$runId"
$dataRoot = Join-Path $runRoot "data"
$webviewRoot = Join-Path $runRoot "webview2"
$playwrightRoot = Join-Path $workRootPath "playwright-core"
$remoteAuthorityEvidence = Join-Path $runRoot "remote-authority.json"
$localAuthorityEvidence = Join-Path $runRoot "local-authority.json"
New-Item -ItemType Directory -Force -Path $dataRoot, $webviewRoot, $playwrightRoot | Out-Null
$utf8NoBom = [System.Text.UTF8Encoding]::new($false)
$preferenceJson = @{ mode = "remote"; remote_url = $remote.GetLeftPart([UriPartial]::Authority) } |
    ConvertTo-Json
[System.IO.File]::WriteAllText(
    (Join-Path $dataRoot "native-backend.json"), $preferenceJson, $utf8NoBom
)

$packageJson = Join-Path $playwrightRoot "package.json"
if (-not (Test-Path -LiteralPath $packageJson -PathType Leaf)) {
    [System.IO.File]::WriteAllText($packageJson, '{"private":true,"type":"module"}', $utf8NoBom)
}
$node = (Get-Command node.exe -ErrorAction Stop).Source
try {
    Install-DevePlaywrightCore `
        -PlaywrightRoot $playwrightRoot `
        -TimeoutSeconds $NpmTimeoutSeconds
} catch {
    Fail $_.Exception.Message
}

$psi = [System.Diagnostics.ProcessStartInfo]::new()
$psi.FileName = $desktopPath
$psi.UseShellExecute = $false
$psi.Environment["DEVE_DESKTOP_DATA_DIR"] = $dataRoot
$psi.Environment.Remove("DEVE_NATIVE_REMOTE_URL") | Out-Null
$psi.Environment["WEBVIEW2_USER_DATA_FOLDER"] = $webviewRoot
$psi.Environment["WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS"] = "--remote-debugging-port=0"
$desktop = [System.Diagnostics.Process]::Start($psi)
$replacement = $null
$success = $false

try {
    $deadline = [DateTime]::UtcNow.AddSeconds($StartupTimeoutSeconds)
    try {
        $remoteCdp = Wait-DeveWebView2CdpEndpoint `
            -HostProcess $desktop `
            -WebViewUserDataRoot $webviewRoot `
            -Deadline $deadline `
            -Label "RemoteBrowser" `
            -RequiredPageOrigins @($remote.GetLeftPart([UriPartial]::Authority))
    } catch {
        Fail $_.Exception.Message
    }
    if (@(Get-InstalledSidecars $sidecarPath).Count -ne 0) {
        Fail "RemoteBrowser started a local sidecar"
    }

    $env:DEVE_DESKTOP_PACKAGED_UI_CDP_ENDPOINT = $remoteCdp.Endpoint
    $env:DEVE_DESKTOP_PACKAGED_UI_PLAYWRIGHT_REQUIRE_FROM = $packageJson
    $env:DEVE_DESKTOP_REMOTE_HTTPS_ORIGIN = $remote.GetLeftPart([UriPartial]::Authority)
    $env:DEVE_DESKTOP_REMOTE_USERNAME = $Username
    $env:DEVE_DESKTOP_REMOTE_PASSWORD = $Password
    $env:DEVE_DESKTOP_REMOTE_AUTHORITY_EVIDENCE_PATH = $remoteAuthorityEvidence
    try {
        & $node (Join-Path $PSScriptRoot "smoke-desktop-remote-browser.mjs")
        if ($LASTEXITCODE -ne 0) { Fail "RemoteBrowser WebView automation failed" }
    } finally {
        Remove-Item Env:DEVE_DESKTOP_REMOTE_HTTPS_ORIGIN -ErrorAction SilentlyContinue
        Remove-Item Env:DEVE_DESKTOP_REMOTE_USERNAME -ErrorAction SilentlyContinue
        Remove-Item Env:DEVE_DESKTOP_REMOTE_PASSWORD -ErrorAction SilentlyContinue
        Remove-Item Env:DEVE_DESKTOP_REMOTE_AUTHORITY_EVIDENCE_PATH -ErrorAction SilentlyContinue
    }

    $oldPid = $desktop.Id
    $menuDeadline = [DateTime]::UtcNow.AddSeconds($StartupTimeoutSeconds)
    Invoke-UseLocalBackendMenu $desktop $menuDeadline
    if (-not $desktop.WaitForExit($ExitTimeoutSeconds * 1000)) {
        Fail "RemoteBrowser process did not exit for native mode transition"
    }
    $restartDeadline = [DateTime]::UtcNow.AddSeconds($StartupTimeoutSeconds)
    $replacement = Find-ReplacementDesktop $desktopPath $oldPid $restartDeadline
    try {
        $localCdp = Wait-DeveWebView2CdpEndpoint `
            -HostProcess $replacement `
            -WebViewUserDataRoot $webviewRoot `
            -Deadline $restartDeadline `
            -Label "restarted LocalBackend" `
            -RequiredPageOrigins @("http://tauri.localhost", "tauri://localhost")
    } catch {
        Fail $_.Exception.Message
    }
    $env:DEVE_DESKTOP_PACKAGED_UI_CDP_ENDPOINT = $localCdp.Endpoint

    $sidecars = @()
    while ([DateTime]::UtcNow -lt $restartDeadline) {
        $sidecars = @(Get-InstalledSidecars $sidecarPath)
        $replacement.Refresh()
        if ($replacement.HasExited) {
            Fail "restarted LocalBackend exited before its sidecar became ready (exit code $($replacement.ExitCode))"
        }
        if ($sidecars.Count -eq 1) { break }
        Start-Sleep -Milliseconds 250
    }
    if ($sidecars.Count -ne 1) { Fail "local restart did not own exactly one sidecar" }
    $preference = Get-Content -Raw -LiteralPath (Join-Path $dataRoot "native-backend.json") |
        ConvertFrom-Json
    if ($preference.mode -ne "local" -or $null -ne $preference.remote_url) {
        Fail "native local transition did not persist canonical local preference"
    }

    $env:DEVE_DESKTOP_LOCAL_AUTHORITY_EVIDENCE_PATH = $localAuthorityEvidence
    try {
        & $node (Join-Path $PSScriptRoot "smoke-desktop-packaged-ui.mjs")
        if ($LASTEXITCODE -ne 0) { Fail "restarted LocalBackend WebView automation failed" }
    } finally {
        Remove-Item Env:DEVE_DESKTOP_LOCAL_AUTHORITY_EVIDENCE_PATH -ErrorAction SilentlyContinue
    }
    $remoteAuthority = Get-Content -Raw -LiteralPath $remoteAuthorityEvidence | ConvertFrom-Json
    $localAuthority = Get-Content -Raw -LiteralPath $localAuthorityEvidence | ConvertFrom-Json
    if ($remoteAuthority.origin -eq $localAuthority.httpBase) {
        Fail "local restart reused the remote authority origin"
    }
    if (
        [string]::IsNullOrWhiteSpace($remoteAuthority.repoId) -or
        [string]::IsNullOrWhiteSpace($localAuthority.repoId) -or
        [int64]$remoteAuthority.scopeNonce -le 0 -or [int64]$localAuthority.scopeNonce -le 0 -or
        $localAuthority.sessionBound -ne $true
    ) {
        Fail "mode transition did not capture fresh endpoint/session/scope evidence"
    }

    $replacement.Refresh()
    if ($replacement.HasExited -or -not $replacement.CloseMainWindow()) {
        Fail "restarted LocalBackend main window was not closeable"
    }
    if (-not $replacement.WaitForExit($ExitTimeoutSeconds * 1000)) {
        Fail "restarted LocalBackend remained alive after close"
    }
    $exitDeadline = [DateTime]::UtcNow.AddSeconds($ExitTimeoutSeconds)
    while ([DateTime]::UtcNow -lt $exitDeadline -and @(Get-InstalledSidecars $sidecarPath).Count -ne 0) {
        Start-Sleep -Milliseconds 250
    }
    if (@(Get-InstalledSidecars $sidecarPath).Count -ne 0) { Fail "orphaned sidecar after switch smoke" }
    $success = $true
    Write-Host "desktop-remote-browser-smoke: ok"
} finally {
    Remove-Item Env:DEVE_DESKTOP_PACKAGED_UI_CDP_ENDPOINT -ErrorAction SilentlyContinue
    Remove-Item Env:DEVE_DESKTOP_PACKAGED_UI_PLAYWRIGHT_REQUIRE_FROM -ErrorAction SilentlyContinue
    if (-not $success) {
        $diagnosticProcess = $desktop
        $diagnosticLabel = "RemoteBrowser"
        if ($null -ne $replacement) {
            $diagnosticProcess = $replacement
            $diagnosticLabel = "restarted LocalBackend"
        }
        try {
            Write-DeveWebView2CdpDiagnostics `
                -HostProcess $diagnosticProcess `
                -WebViewUserDataRoot $webviewRoot `
                -OutputPath (Join-Path $runRoot "webview2-cdp-diagnostic.json") `
                -Label $diagnosticLabel
        } catch {
            Write-Warning "desktop-remote-browser-smoke: failed to write sanitized CDP diagnostic: $($_.Exception.Message)"
        }
    }
    Stop-ProcessIfAlive $desktop.Id
    if ($null -ne $replacement) { Stop-ProcessIfAlive $replacement.Id }
    try {
        foreach ($sidecar in @(Get-InstalledSidecars $sidecarPath)) {
            Stop-ProcessIfAlive ([int]$sidecar.ProcessId)
        }
    } catch {
        Write-Warning "desktop-remote-browser-smoke: sidecar cleanup observation failed: $($_.Exception.Message)"
    }
    if ($success -and (Test-Path -LiteralPath $runRoot)) {
        Remove-Item -LiteralPath $runRoot -Recurse -Force -ErrorAction SilentlyContinue
    } elseif (-not $success) {
        Write-Warning "desktop-remote-browser-smoke: preserving failure evidence at $runRoot"
    }
}
