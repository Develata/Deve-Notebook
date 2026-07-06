param(
    [string]$DesktopExe = "target\debug\deve_desktop.exe",
    [int]$StartupTimeoutSeconds = 30,
    [int]$ExitTimeoutSeconds = 8,
    [ValidateSet("Force", "CloseMainWindow")]
    [string]$ShutdownMode = "Force"
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

function Assert-DefaultWorkspaceInitialized($DataRoot, $RoleBody) {
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
        Fail "default local workspace repo health is not healthy: $RoleBody"
    }

    $ledgerRoot = Join-Path $DataRoot "ledger"
    $localLedgerRoot = Join-Path $ledgerRoot "local"
    $locatorPath = Join-Path $ledgerRoot ".host\projection-locators.toml"
    if (-not (Test-Path -LiteralPath $ledgerRoot -PathType Container)) {
        Fail "default local workspace did not create ledger root: $ledgerRoot"
    }
    if (-not (Test-Path -LiteralPath $localLedgerRoot -PathType Container)) {
        Fail "default local workspace did not create local ledger root: $localLedgerRoot"
    }
    $localRepo = Get-ChildItem -LiteralPath $localLedgerRoot -Filter "*.redb" -File -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if ($null -eq $localRepo) {
        Fail "default local workspace did not create a local repo ledger under: $localLedgerRoot"
    }
    if (-not (Test-Path -LiteralPath $locatorPath -PathType Leaf)) {
        Fail "default local workspace did not create projection locator file: $locatorPath"
    }
    $locatorContent = Get-Content -Raw -LiteralPath $locatorPath
    if ($locatorContent -notmatch "(?m)^projection_base_abs\s*=\s*'([^']+)'\s*$") {
        Fail "projection locator file does not expose projection_base_abs: $locatorPath"
    }
    $projectionBaseRoot = $Matches[1]
    if (-not (Test-Path -LiteralPath $projectionBaseRoot -PathType Container)) {
        Fail "default local workspace projection base does not exist: $projectionBaseRoot"
    }
    $normalizedDataRoot = Resolve-NormalizedPath $DataRoot
    $normalizedProjectionBase = Resolve-NormalizedPath $projectionBaseRoot
    $dataRootPrefix = $normalizedDataRoot.TrimEnd('\') + '\'
    if (-not $normalizedProjectionBase.StartsWith($dataRootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        Fail "projection base is outside the isolated desktop data root: $projectionBaseRoot"
    }
    $projectionWorkspace = Get-ChildItem -LiteralPath $projectionBaseRoot -Directory -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if ($null -eq $projectionWorkspace) {
        Fail "default local workspace did not create a projection workspace under: $projectionBaseRoot"
    }
    $workspaceRoot = Resolve-NormalizedPath $projectionWorkspace.FullName
    $identityPath = Join-Path $workspaceRoot ".notegit\identity.toml"
    if (-not (Test-Path -LiteralPath $identityPath -PathType Leaf)) {
        Fail "default local workspace did not create repo workspace identity: $identityPath"
    }
    $gitignorePath = Join-Path $workspaceRoot ".gitignore"
    if (-not (Test-Path -LiteralPath $gitignorePath -PathType Leaf)) {
        Fail "default local workspace did not create repo-local .gitignore: $gitignorePath"
    }
    $gitignore = Get-Content -LiteralPath $gitignorePath
    if (-not ($gitignore | Where-Object { $_.Trim() -eq ".notegit/" } | Select-Object -First 1)) {
        Fail "repo-local .gitignore does not protect .notegit/: $gitignorePath"
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
$smokeRoot = Join-Path (Resolve-Path -LiteralPath ".") "target\desktop-local-backend-lifecycle-smoke"
$runId = "{0}-{1}" -f ([DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()), $PID
$dataRoot = Join-Path $smokeRoot $runId
New-Item -ItemType Directory -Force -Path $dataRoot | Out-Null

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
$smokeSucceeded = $false

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
    Assert-DefaultWorkspaceInitialized $dataRoot $roleBody

    Request-DesktopExit $desktop $ShutdownMode
    $desktopExited = $desktop.WaitForExit($ExitTimeoutSeconds * 1000)
    if (-not $desktopExited) {
        Fail "desktop process remained alive after $ShutdownMode shutdown request"
    }

    $exitDeadline = [DateTime]::UtcNow.AddSeconds($ExitTimeoutSeconds)
    while ([DateTime]::UtcNow -lt $exitDeadline) {
        if ($null -eq (Get-Process -Id $childPid -ErrorAction SilentlyContinue)) {
            Write-Host "desktop-local-backend-lifecycle: ok"
            $smokeSucceeded = $true
            exit 0
        }
        Start-Sleep -Milliseconds 250
    }

    Fail "deve_cli child $childPid remained alive after Desktop parent exit"
} finally {
    Stop-ProcessIfAlive $desktop.Id
    Stop-ProcessIfAlive $childPid
    if ($smokeSucceeded -and (Test-Path -LiteralPath $dataRoot)) {
        $resolvedDataRoot = Resolve-NormalizedPath $dataRoot
        $resolvedSmokeRoot = (Resolve-NormalizedPath $smokeRoot).TrimEnd('\') + '\'
        if ($resolvedDataRoot.StartsWith($resolvedSmokeRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
            Remove-Item -LiteralPath $resolvedDataRoot -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}
