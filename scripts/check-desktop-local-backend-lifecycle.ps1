param(
    [string]$DesktopExe = "target\debug\deve_desktop.exe",
    [int]$StartupTimeoutSeconds = 30,
    [int]$ExitTimeoutSeconds = 8
)

$ErrorActionPreference = "Stop"

function Fail($Message) {
    Write-Error "desktop-local-backend-lifecycle: $Message"
    exit 1
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

function Stop-ProcessIfAlive($ProcessId) {
    if ($null -eq $ProcessId) {
        return
    }
    $process = Get-Process -Id $ProcessId -ErrorAction SilentlyContinue
    if ($null -ne $process) {
        Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
    }
}

$desktopPath = Resolve-Path -LiteralPath $DesktopExe -ErrorAction Stop
$dataRoot = Join-Path (Resolve-Path -LiteralPath ".") "target\desktop-local-backend-lifecycle-smoke"

$psi = [System.Diagnostics.ProcessStartInfo]::new()
$psi.FileName = $desktopPath.Path
$psi.Arguments = "--local-backend"
$psi.UseShellExecute = $false
$psi.RedirectStandardInput = $true
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.Environment["DEVE_DESKTOP_DATA_DIR"] = $dataRoot

$desktop = [System.Diagnostics.Process]::Start($psi)
$childPid = $null

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
    if ($roleBody -notmatch '"role"\s*:\s*"native-main"') {
        Fail "unexpected node role response: $roleBody"
    }

    Stop-Process -Id $desktop.Id -Force
    $desktop.WaitForExit($ExitTimeoutSeconds * 1000) | Out-Null

    $exitDeadline = [DateTime]::UtcNow.AddSeconds($ExitTimeoutSeconds)
    while ([DateTime]::UtcNow -lt $exitDeadline) {
        if ($null -eq (Get-Process -Id $childPid -ErrorAction SilentlyContinue)) {
            Write-Host "desktop-local-backend-lifecycle: ok"
            exit 0
        }
        Start-Sleep -Milliseconds 250
    }

    Fail "deve_cli child $childPid remained alive after Desktop parent exit"
} finally {
    Stop-ProcessIfAlive $desktop.Id
    Stop-ProcessIfAlive $childPid
}
