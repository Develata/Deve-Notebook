param(
    [Parameter(Mandatory = $true)]
    [string]$DesktopBinary,
    [Parameter(Mandatory = $true)]
    [string]$WorkRoot,
    [int]$StartupTimeoutSeconds = 45,
    [int]$ExitTimeoutSeconds = 10,
    [int]$NpmTimeoutSeconds = 180
)

$ErrorActionPreference = "Stop"

function Fail($Message) {
    throw "desktop-packaged-ui-smoke: $Message"
}

function Get-FreeLoopbackPort {
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
    $listener.Start()
    try {
        return ([System.Net.IPEndPoint]$listener.LocalEndpoint).Port
    } finally {
        $listener.Stop()
    }
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

function Stop-ProcessIfAlive($ProcessId) {
    if ($null -ne $ProcessId -and $null -ne (Get-Process -Id $ProcessId -ErrorAction SilentlyContinue)) {
        Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
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

$npm = (Get-Command npm.cmd -ErrorAction Stop).Source
$node = (Get-Command node.exe -ErrorAction Stop).Source
$playwrightModule = Join-Path $playwrightRoot "node_modules\playwright-core\package.json"
if (-not (Test-Path -LiteralPath $playwrightModule -PathType Leaf)) {
    $quotedPlaywrightRoot = '"' + $playwrightRoot + '"'
    $npmProcess = Start-Process -FilePath $npm -ArgumentList @(
        "--prefix", $quotedPlaywrightRoot, "install", "--no-audit", "--no-fund", "playwright-core@1.58.2"
    ) -NoNewWindow -PassThru
    if (-not $npmProcess.WaitForExit($NpmTimeoutSeconds * 1000)) {
        & taskkill.exe /PID $npmProcess.Id /T /F 2>$null | Out-Null
        Fail "playwright-core install exceeded ${NpmTimeoutSeconds}s"
    }
    $npmProcess.Refresh()
    if ($npmProcess.ExitCode -ne 0) {
        Fail "failed to install playwright-core (exit $($npmProcess.ExitCode))"
    }
}

$cdpPort = Get-FreeLoopbackPort
$psi = [System.Diagnostics.ProcessStartInfo]::new()
$psi.FileName = $desktopPath
$psi.Arguments = "--local-backend"
$psi.UseShellExecute = $false
$psi.Environment["DEVE_DESKTOP_DATA_DIR"] = $dataRoot
$psi.Environment["WEBVIEW2_USER_DATA_FOLDER"] = $webviewRoot
$psi.Environment["WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS"] = "--remote-debugging-port=$cdpPort --remote-allow-origins=http://127.0.0.1:$cdpPort"
$desktop = [System.Diagnostics.Process]::Start($psi)
$success = $false

try {
    $deadline = [DateTime]::UtcNow.AddSeconds($StartupTimeoutSeconds)
    while ([DateTime]::UtcNow -lt $deadline) {
        $sidecars = @(Get-InstalledSidecars $sidecarPath)
        try {
            $version = Invoke-RestMethod -Uri "http://127.0.0.1:$cdpPort/json/version" -TimeoutSec 2
            if ($null -ne $version.webSocketDebuggerUrl -and $sidecars.Count -ge 1) {
                break
            }
        } catch {
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
    if ([DateTime]::UtcNow -ge $deadline) {
        Fail "timed out waiting for WebView2 CDP endpoint on port $cdpPort"
    }

    $env:DEVE_DESKTOP_PACKAGED_UI_CDP_ENDPOINT = "http://127.0.0.1:$cdpPort"
    $env:DEVE_DESKTOP_PACKAGED_UI_PLAYWRIGHT_REQUIRE_FROM = $packageJson
    try {
        & $node (Join-Path $PSScriptRoot "smoke-desktop-packaged-ui.mjs")
        if ($LASTEXITCODE -ne 0) {
            Fail "native WebView automation failed"
        }
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

    $success = $true
    Write-Host "desktop-packaged-ui-smoke: ok"
} finally {
    Stop-ProcessIfAlive $desktop.Id
    foreach ($sidecar in @(Get-InstalledSidecars $sidecarPath)) {
        Stop-ProcessIfAlive ([int]$sidecar.ProcessId)
    }
    if ($success -and (Test-Path -LiteralPath $runRoot)) {
        Remove-Item -LiteralPath $runRoot -Recurse -Force -ErrorAction SilentlyContinue
    } elseif (-not $success) {
        Write-Warning "desktop-packaged-ui-smoke: preserving failure evidence at $runRoot"
    }
}
