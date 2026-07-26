param(
    [string]$DesktopExe = "target\debug\deve_desktop.exe",
    [int]$StartupTimeoutSeconds = 30,
    [int]$ExitTimeoutSeconds = 8,
    [switch]$ForceGitUnavailable,
    [ValidateSet("Force", "CloseMainWindow")]
    [string]$ShutdownMode = "Force"
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "lib\webview2-cdp.ps1")

function Fail($Message) {
    throw "desktop-local-backend-lifecycle: $Message"
}

function Get-DeveCliChild($ParentPid) {
    Get-CimInstance Win32_Process -Filter "ParentProcessId = $ParentPid" |
        Where-Object {
            $_.Name -ieq "deve_cli.exe" -and
            $_.CommandLine -match "serve" -and
            $_.CommandLine -match "--native-loopback"
        } |
        Select-Object -First 1
}

function Resolve-NormalizedPath($Path) {
    $resolved = (Resolve-Path -LiteralPath $Path -ErrorAction Stop).Path
    $providerPrefix = "Microsoft.PowerShell.Core\FileSystem::"
    if ($resolved.StartsWith($providerPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        $resolved = $resolved.Substring($providerPrefix.Length)
    }
    if ($resolved.StartsWith('\\?\')) {
        return $resolved.Substring(4)
    }
    return $resolved
}

function Read-RequiredJsonCount($Object, $PropertyName, $RoleBody) {
    if ($null -eq $Object) {
        Fail "node role response is missing the object for ${PropertyName}: $RoleBody"
    }
    $property = $Object.PSObject.Properties[$PropertyName]
    if ($null -eq $property) {
        Fail "node role response is missing required count ${PropertyName}: $RoleBody"
    }
    $value = $property.Value
    if ($value -isnot [int] -and $value -isnot [long]) {
        Fail "node role count ${PropertyName} is not an integer: $RoleBody"
    }
    return [long]$value
}

function Assert-ZeroRepoHostInitialized($DataRoot, $RoleBody) {
    try {
        $roleJson = $RoleBody | ConvertFrom-Json -ErrorAction Stop
    } catch {
        Fail "node role response is not valid JSON: $RoleBody"
    }

    if ($roleJson.role -ne "native-main") {
        Fail "unexpected node role response: $RoleBody"
    }
    if ($roleJson.native_service.endpoint.session_bound -ne $true) {
        Fail "native session was not bound: $RoleBody"
    }
    if ($roleJson.repo_health.status -ne "healthy") {
        Fail "zero-repo host health is not healthy: $RoleBody"
    }
    $localTotal = Read-RequiredJsonCount $roleJson.repo_health "local_total" $RoleBody
    $healthyRepos = Read-RequiredJsonCount $roleJson.repo_health "healthy" $RoleBody
    $degradedRepos = Read-RequiredJsonCount $roleJson.repo_health "degraded" $RoleBody
    if ($localTotal -ne 0 -or $healthyRepos -ne 0 -or $degradedRepos -ne 0) {
        Fail "zero-repo host unexpectedly mounted local authority: $RoleBody"
    }
    $watcherExpected = Read-RequiredJsonCount $roleJson.watcher_health "expected" $RoleBody
    $watcherRunning = Read-RequiredJsonCount $roleJson.watcher_health "running" $RoleBody
    $watcherUnavailable = Read-RequiredJsonCount $roleJson.watcher_health "unavailable" $RoleBody
    if (
        $roleJson.watcher_health.status -ne "healthy" -or
        $watcherExpected -ne 0 -or
        $watcherRunning -ne 0 -or
        $watcherUnavailable -ne 0
    ) {
        Fail "zero-repo watcher health is not expected=0 healthy: $RoleBody"
    }

    $ledgerRoot = Join-Path $DataRoot "ledger"
    $localLedgerRoot = Join-Path $ledgerRoot "local"
    $locatorPath = Join-Path $ledgerRoot ".host\projection-locators.toml"
    if (-not (Test-Path -LiteralPath $ledgerRoot -PathType Container)) {
        Fail "zero-repo host did not create ledger root: $ledgerRoot"
    }
    if (-not (Test-Path -LiteralPath $localLedgerRoot -PathType Container)) {
        Fail "zero-repo host did not create local ledger root: $localLedgerRoot"
    }
    $localRepos = @(
        Get-ChildItem -LiteralPath $localLedgerRoot -Filter "*.redb" -File -ErrorAction Stop
    )
    if ($localRepos.Count -ne 0) {
        Fail "zero-repo host created local repo authority before typed Create: $($localRepos.FullName -join ',')"
    }
    if (Test-Path -LiteralPath $locatorPath) {
        Fail "zero-repo host created projection locator state before typed Create: $locatorPath"
    }
    $projectionBase = Join-Path $DataRoot "workspace"
    if (Test-Path -LiteralPath $projectionBase -PathType Container) {
        $projectionEntries = @(Get-ChildItem -LiteralPath $projectionBase -Force)
        if ($projectionEntries.Count -ne 0) {
            Fail "zero-repo host created projection content before typed Create: $($projectionEntries.FullName -join ',')"
        }
    }
}

function Request-DesktopExit($Process, $Mode) {
    if ($Mode -eq "Force") {
        Stop-Process -Id $Process.Id -Force
        return
    }

    $deadline = [DateTime]::UtcNow.AddSeconds(5)
    while ([DateTime]::UtcNow -lt $deadline) {
        $Process.Refresh()
        if ($Process.MainWindowHandle -ne 0 -and $Process.CloseMainWindow()) {
            return
        }
        Start-Sleep -Milliseconds 250
    }

    Fail "desktop main window was not closeable"
}

$desktopPath = Resolve-Path -LiteralPath $DesktopExe -ErrorAction Stop
$desktopDir = Split-Path -Parent $desktopPath.Path
$sidecarPath = Join-Path $desktopDir "deve_cli.exe"
if (-not (Test-Path -LiteralPath $sidecarPath -PathType Leaf)) {
    Fail "deve_cli sidecar is missing next to DesktopExe: $sidecarPath; build it with cargo build -p deve_cli --bin deve_cli before running this smoke"
}
$smokeRoot = Join-Path (Resolve-Path -LiteralPath ".") "target\desktop-local-backend-lifecycle-smoke"
$runId = "{0}-{1}" -f ([DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()), $PID
$dataRoot = Join-Path $smokeRoot $runId
$webviewRoot = Join-Path $dataRoot "webview2-normal"
New-Item -ItemType Directory -Force -Path $dataRoot, $webviewRoot | Out-Null

$psi = [System.Diagnostics.ProcessStartInfo]::new()
$psi.FileName = $desktopPath.Path
$psi.Arguments = "--local-backend"
$psi.UseShellExecute = $false
$psi.RedirectStandardInput = $true
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.Environment["DEVE_DESKTOP_DATA_DIR"] = $dataRoot
$psi.Environment["WEBVIEW2_USER_DATA_FOLDER"] = $webviewRoot
$psi.Environment.Remove("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS") | Out-Null
$psi.Environment.Remove("DEVE_DESKTOP_WEBVIEW2_CDP") | Out-Null
if ($ForceGitUnavailable) {
    $psi.Environment["DEVE_GIT_EXECUTABLE"] = Join-Path $dataRoot "missing-git.exe"
}

$desktop = [System.Diagnostics.Process]::Start($psi)
$childPid = $null
$smokeSucceeded = $false
$caughtError = $null
$cleanupErrors = [System.Collections.Generic.List[string]]::new()

try {
    $deadline = [DateTime]::UtcNow.AddSeconds($StartupTimeoutSeconds)
    $child = $null
    while ([DateTime]::UtcNow -lt $deadline) {
        $child = Get-DeveCliChild $desktop.Id
        if ($null -ne $child) {
            break
        }
        Start-Sleep -Milliseconds 250
    }
    if ($null -eq $child) {
        Fail "timed out waiting for deve_cli native-loopback child"
    }
    $childPid = [int]$child.ProcessId

    if ($child.CommandLine -notmatch "--port\s+(\d+)") {
        Fail "child command line does not expose --port: $($child.CommandLine)"
    }
    $port = [int]$Matches[1]
    $roleUrl = "http://127.0.0.1:$port/api/node/role"

    $probeDeadline = [DateTime]::UtcNow.AddSeconds($StartupTimeoutSeconds)
    $roleBody = $null
    while ([DateTime]::UtcNow -lt $probeDeadline) {
        try {
            $response = Invoke-WebRequest -UseBasicParsing -Uri $roleUrl -TimeoutSec 2
            if (
                $response.StatusCode -eq 200 -and
                $response.Content -match '"role"\s*:\s*"native-main"' -and
                $response.Content -match '"session_bound"\s*:\s*true'
            ) {
                $roleBody = $response.Content
                break
            }
        } catch {
        }
        Start-Sleep -Milliseconds 250
    }
    if ($null -eq $roleBody) {
        Fail "timed out probing $roleUrl"
    }
    Assert-ZeroRepoHostInitialized $dataRoot $roleBody
    $webviewDeadline = [DateTime]::UtcNow.AddSeconds($StartupTimeoutSeconds)
    $webviewProcesses = @()
    while ([DateTime]::UtcNow -lt $webviewDeadline) {
        $webviewProcesses = @(Get-DeveWebView2Processes $webviewRoot)
        if ($webviewProcesses.Count -ge 1) {
            break
        }
        Start-Sleep -Milliseconds 250
    }
    if ($webviewProcesses.Count -eq 0) {
        Fail "timed out waiting for isolated normal-start WebView2"
    }
    $activePortPath = Join-Path $webviewRoot "EBWebView\DevToolsActivePort"
    if (Test-Path -LiteralPath $activePortPath) {
        Fail "normal Desktop startup unexpectedly created a CDP endpoint: $activePortPath"
    }
    $debugProcesses = @(
        $webviewProcesses |
            Where-Object { $_.CommandLine -match "--remote-debugging-port(?:=|\s)" }
    )
    if ($debugProcesses.Count -ne 0) {
        Fail "normal Desktop startup unexpectedly enabled remote debugging: $($debugProcesses.ProcessId -join ',')"
    }

    Request-DesktopExit $desktop $ShutdownMode
    $desktopExited = $desktop.WaitForExit($ExitTimeoutSeconds * 1000)
    if (-not $desktopExited) {
        Fail "desktop process remained alive after $ShutdownMode shutdown request"
    }

    $exitDeadline = [DateTime]::UtcNow.AddSeconds($ExitTimeoutSeconds)
    while ([DateTime]::UtcNow -lt $exitDeadline) {
        $childExited = $null -eq (Get-Process -Id $childPid -ErrorAction SilentlyContinue)
        $webviewExited = @(Get-DeveWebView2Processes $webviewRoot).Count -eq 0
        if ($childExited -and $webviewExited) {
            $smokeSucceeded = $true
            break
        }
        Start-Sleep -Milliseconds 250
    }

    if (-not $smokeSucceeded) {
        $remainingWebView = @(Get-DeveWebView2Processes $webviewRoot)
        Fail "Desktop cleanup incomplete; sidecar_alive=$(-not $childExited), webview2_pids=$($remainingWebView.ProcessId -join ',')"
    }
} catch {
    $caughtError = $_
} finally {
    $cleanupSteps = [System.Collections.Generic.List[scriptblock]]::new()
    $cleanupSteps.Add({
        Stop-DeveProcessIfAlive `
            -ProcessId $desktop.Id `
            -TimeoutSeconds $ExitTimeoutSeconds `
            -Label "Desktop"
    })
    if ($null -ne $childPid) {
        $cleanupSteps.Add({
            Stop-DeveProcessIfAlive `
                -ProcessId $childPid `
                -TimeoutSeconds $ExitTimeoutSeconds `
                -Label "deve_cli sidecar"
        })
    }
    $cleanupSteps.Add({
        Stop-DeveWebView2Processes `
            -WebViewUserDataRoot $webviewRoot `
            -TimeoutSeconds $ExitTimeoutSeconds `
            -Label "normal-start WebView2"
    })
    foreach ($cleanup in $cleanupSteps) {
        try {
            & $cleanup
        } catch {
            $cleanupErrors.Add($_.Exception.Message)
        }
    }
    if ($smokeSucceeded -and $cleanupErrors.Count -eq 0 -and (Test-Path -LiteralPath $dataRoot)) {
        $resolvedDataRoot = Resolve-NormalizedPath $dataRoot
        $resolvedSmokeRoot = (Resolve-NormalizedPath $smokeRoot).TrimEnd('\') + '\'
        if ($resolvedDataRoot.StartsWith($resolvedSmokeRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
            try {
                Remove-Item -LiteralPath $resolvedDataRoot -Recurse -Force -ErrorAction Stop
            } catch {
                $cleanupErrors.Add("failed to remove successful run root: $($_.Exception.Message)")
            }
        }
    }
}

if ($cleanupErrors.Count -ne 0) {
    $primary = if ($null -eq $caughtError) { "none" } else { $caughtError.Exception.Message }
    Fail "primary failure: $primary; cleanup failures: $($cleanupErrors -join '; ')"
}
if ($null -ne $caughtError) {
    throw $caughtError
}
Write-Host "desktop-local-backend-lifecycle: ok"
