param(
    [Parameter(Mandatory = $true)]
    [string]$DesktopBinary,
    [Parameter(Mandatory = $true)]
    [string]$WorkRoot,
    [int]$StartupTimeoutSeconds = 45,
    [int]$ExitTimeoutSeconds = 10,
    [int]$NpmTimeoutSeconds = 180,
    [int]$JourneyTimeoutSeconds = 300
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "lib\playwright-core.ps1")
. (Join-Path $PSScriptRoot "lib\webview2-cdp.ps1")

function Fail($Message) {
    throw "desktop-packaged-ui-smoke: $Message"
}

function Get-InstalledSidecars($ExecutablePath) {
    $expectedPath = [System.IO.Path]::GetFullPath($ExecutablePath)
    @(Get-CimInstance Win32_Process -Filter "Name = 'deve_cli.exe'" |
        Where-Object {
            $null -ne $_.ExecutablePath -and
            [System.IO.Path]::GetFullPath($_.ExecutablePath).Equals(
                $expectedPath,
                [System.StringComparison]::OrdinalIgnoreCase
            ) -and
            $_.CommandLine -match "serve" -and
            $_.CommandLine -match "--native-loopback"
        })
}

function Stop-InstalledSidecars($ExecutablePath, $TimeoutSeconds) {
    foreach ($sidecar in @(Get-InstalledSidecars $ExecutablePath)) {
        Stop-DeveProcessIfAlive `
            -ProcessId ([int]$sidecar.ProcessId) `
            -TimeoutSeconds $TimeoutSeconds `
            -Label "installed sidecar"
    }
    $remaining = @(Get-InstalledSidecars $ExecutablePath)
    if ($remaining.Count -ne 0) {
        throw "installed sidecar cleanup left processes: $($remaining.ProcessId -join ',')"
    }
}

$desktopPath = (Resolve-Path -LiteralPath $DesktopBinary -ErrorAction Stop).Path
$sidecarPath = Join-Path (Split-Path -Parent $desktopPath) "deve_cli.exe"
if (-not (Test-Path -LiteralPath $sidecarPath -PathType Leaf)) {
    Fail "deve_cli sidecar is missing next to installed Desktop binary: $sidecarPath"
}
$preexistingSidecars = @(Get-InstalledSidecars $sidecarPath)
if ($preexistingSidecars.Count -ne 0) {
    Fail "installed sidecar path already has running native-loopback processes: $($preexistingSidecars.ProcessId -join ',')"
}
if ($desktopPath -notmatch '(?i)DeveNotebookInstallerSmoke') {
    Fail "DesktopBinary must point at the isolated installed package, not a build-tree executable: $desktopPath"
}

$runId = "{0}-{1}" -f ([DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()), $PID
$runRoot = Join-Path $WorkRoot "packaged-ui-$runId"
$dataRoot = Join-Path $runRoot "data"
$webviewRoot = Join-Path $runRoot "webview2"
$playwrightRoot = Join-Path $WorkRoot "playwright-core"
New-Item -ItemType Directory -Force -Path $dataRoot, $webviewRoot, $playwrightRoot | Out-Null
$packageJson = Join-Path $playwrightRoot "package.json"
if (-not (Test-Path -LiteralPath $packageJson -PathType Leaf)) {
    Set-Content -LiteralPath $packageJson -Encoding utf8 -Value '{"private":true,"type":"module"}'
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
$psi.Arguments = "--local-backend"
$psi.UseShellExecute = $false
$psi.Environment["DEVE_DESKTOP_DATA_DIR"] = $dataRoot
$psi.Environment["WEBVIEW2_USER_DATA_FOLDER"] = $webviewRoot
$psi.Environment.Remove("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS") | Out-Null
$psi.Environment["DEVE_DESKTOP_WEBVIEW2_CDP"] = "assigned-loopback"
$desktop = [System.Diagnostics.Process]::Start($psi)
$journeyCompleted = $false
$caughtError = $null
$cleanupErrors = [System.Collections.Generic.List[string]]::new()

try {
    $deadline = [DateTime]::UtcNow.AddSeconds($StartupTimeoutSeconds)
    try {
        $cdp = Wait-DeveWebView2CdpEndpoint `
            -HostProcess $desktop `
            -WebViewUserDataRoot $webviewRoot `
            -Deadline $deadline `
            -Label "packaged Desktop" `
            -RequiredPageOrigins @("http://tauri.localhost", "tauri://localhost")
    } catch {
        Fail $_.Exception.Message
    }
    while ([DateTime]::UtcNow -lt $deadline) {
        $sidecars = @(Get-InstalledSidecars $sidecarPath)
        if ($sidecars.Count -ge 1) {
            break
        }
        $desktop.Refresh()
        if ($desktop.HasExited) {
            Fail "packaged Desktop exited before its sidecar became ready (exit code $($desktop.ExitCode))"
        }
        Start-Sleep -Milliseconds 250
    }
    $sidecars = @(Get-InstalledSidecars $sidecarPath)
    if ($sidecars.Count -eq 0) {
        Fail "timed out waiting for packaged Desktop sidecar"
    }
    if ($sidecars.Count -ne 1) {
        Fail "packaged Desktop started multiple native-loopback sidecars: $($sidecars.ProcessId -join ',')"
    }
    $env:DEVE_DESKTOP_PACKAGED_UI_CDP_ENDPOINT = $cdp.Endpoint
    $env:DEVE_DESKTOP_PACKAGED_UI_PLAYWRIGHT_REQUIRE_FROM = $packageJson
    try {
        Invoke-DeveNodeJourney `
            -NodePath $node `
            -ScriptPath (Join-Path $PSScriptRoot "smoke-desktop-packaged-ui.mjs") `
            -TimeoutSeconds $JourneyTimeoutSeconds `
            -Label "packaged Desktop WebView automation"
    } finally {
        Remove-Item Env:DEVE_DESKTOP_PACKAGED_UI_CDP_ENDPOINT -ErrorAction SilentlyContinue
        Remove-Item Env:DEVE_DESKTOP_PACKAGED_UI_PLAYWRIGHT_REQUIRE_FROM -ErrorAction SilentlyContinue
    }

    $desktop.Refresh()
    if ($desktop.HasExited -or -not $desktop.CloseMainWindow()) {
        Fail "packaged Desktop main window was not closeable"
    }
    if (-not $desktop.WaitForExit($ExitTimeoutSeconds * 1000)) {
        Fail "packaged Desktop remained alive after main-window close"
    }

    $exitDeadline = [DateTime]::UtcNow.AddSeconds($ExitTimeoutSeconds)
    while ([DateTime]::UtcNow -lt $exitDeadline -and
        @(Get-InstalledSidecars $sidecarPath).Count -ne 0) {
        Start-Sleep -Milliseconds 250
    }
    $orphanedSidecars = @(Get-InstalledSidecars $sidecarPath)
    if ($orphanedSidecars.Count -ne 0) {
        Fail "installed deve_cli sidecars remained alive after packaged Desktop exit: $($orphanedSidecars.ProcessId -join ',')"
    }

    $journeyCompleted = $true
} catch {
    $caughtError = $_
} finally {
    if (-not $journeyCompleted) {
        try {
            Write-DeveWebView2CdpDiagnostics `
                -HostProcess $desktop `
                -WebViewUserDataRoot $webviewRoot `
                -OutputPath (Join-Path $runRoot "webview2-cdp-diagnostic.json") `
                -Label "packaged Desktop"
        } catch {
            Write-Warning "desktop-packaged-ui-smoke: failed to write sanitized CDP diagnostic: $($_.Exception.Message)"
        }
    }
    foreach ($cleanup in @(
        {
            Stop-DeveProcessIfAlive `
                -ProcessId $desktop.Id `
                -TimeoutSeconds $ExitTimeoutSeconds `
                -Label "packaged Desktop"
        },
        {
            Stop-InstalledSidecars `
                -ExecutablePath $sidecarPath `
                -TimeoutSeconds $ExitTimeoutSeconds
        },
        {
            Stop-DeveWebView2Processes `
                -WebViewUserDataRoot $webviewRoot `
                -TimeoutSeconds $ExitTimeoutSeconds `
                -Label "packaged Desktop WebView2"
        }
    )) {
        try {
            & $cleanup
        } catch {
            $cleanupErrors.Add($_.Exception.Message)
        }
    }
    if ($journeyCompleted -and $cleanupErrors.Count -eq 0 -and (Test-Path -LiteralPath $runRoot)) {
        try {
            Remove-Item -LiteralPath $runRoot -Recurse -Force -ErrorAction Stop
        } catch {
            $cleanupErrors.Add("failed to remove successful run root: $($_.Exception.Message)")
        }
    }
    if (-not $journeyCompleted -or $cleanupErrors.Count -ne 0) {
        Write-Warning "desktop-packaged-ui-smoke: preserving failure evidence at $runRoot"
    }
}

if ($cleanupErrors.Count -ne 0) {
    $primary = if ($null -eq $caughtError) { "none" } else { $caughtError.Exception.Message }
    Fail "primary failure: $primary; cleanup failures: $($cleanupErrors -join '; ')"
}
if ($null -ne $caughtError) {
    throw $caughtError
}
Write-Host "desktop-packaged-ui-smoke: ok"
