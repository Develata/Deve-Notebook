$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "lib\webview2-cdp.ps1")

function Assert-True($Condition, $Message) {
    if (-not $Condition) { throw "webview2-cdp-test: $Message" }
}

function Assert-Throws($Action, $Pattern, $Message) {
    try {
        & $Action
    } catch {
        if ($_.Exception.Message -match $Pattern) { return }
        throw "webview2-cdp-test: $Message; unexpected error: $($_.Exception.Message)"
    }
    throw "webview2-cdp-test: $Message; expected an error"
}

function Assert-NodeScriptStopped($ProcessId, $ScriptPath, $Message) {
    $process = Get-CimInstance `
        -ClassName Win32_Process `
        -Filter "ProcessId = $ProcessId" `
        -OperationTimeoutSec 5
    if ($null -eq $process) { return }
    $expectedScript = [System.IO.Path]::GetFullPath($ScriptPath)
    if (
        -not [string]::IsNullOrWhiteSpace([string]$process.CommandLine) -and
        $process.CommandLine.IndexOf(
            $expectedScript,
            [System.StringComparison]::OrdinalIgnoreCase
        ) -lt 0
    ) {
        return
    }
    throw "webview2-cdp-test: $Message"
}

$root = Join-Path ([System.IO.Path]::GetTempPath()) ("deve-webview2-cdp-test-{0}" -f [Guid]::NewGuid())
$activePortDirectory = Join-Path $root "EBWebView"
$activePortPath = Join-Path $activePortDirectory "DevToolsActivePort"
$utf8NoBom = [System.Text.UTF8Encoding]::new($false)
New-Item -ItemType Directory -Force -Path $activePortDirectory | Out-Null

try {
    $missing = Read-DeveWebView2ActivePort -WebViewUserDataRoot (Join-Path $root "missing")
    Assert-True ($null -eq $missing) "missing ActivePort file must remain retryable"

    foreach ($fixture in @(
        @{ Lines = @(); Pattern = "incomplete" },
        @{ Lines = @("abc", "/devtools/browser/test"); Pattern = "non-numeric" },
        @{ Lines = @("0", "/devtools/browser/test"); Pattern = "outside" },
        @{ Lines = @("65536", "/devtools/browser/test"); Pattern = "outside" },
        @{ Lines = @("9222", "not-a-browser-target"); Pattern = "malformed" }
    )) {
        [System.IO.File]::WriteAllLines($activePortPath, [string[]]$fixture.Lines, $utf8NoBom)
        Assert-Throws {
            Read-DeveWebView2ActivePort -WebViewUserDataRoot $root | Out-Null
        } $fixture.Pattern "invalid ActivePort fixture must fail closed"
    }

    [System.IO.File]::WriteAllLines(
        $activePortPath,
        [string[]]@("43123", "/devtools/browser/test-target"),
        $utf8NoBom
    )
    $parsed = Read-DeveWebView2ActivePort -WebViewUserDataRoot $root
    Assert-True ($parsed.Port -eq 43123) "valid ActivePort port was not parsed"
    Assert-True ($parsed.Path -eq $activePortPath) "valid ActivePort path was not preserved"
    Assert-True ($parsed.BrowserTarget -eq "/devtools/browser/test-target") "valid browser target was not preserved"

    $script:requestedCdpUri = $null
    $script:pageProbeCount = 0
    function Invoke-RestMethod {
        param([string]$Uri, [int]$TimeoutSec)
        $script:requestedCdpUri = $Uri
        if ($Uri.EndsWith("/json/version")) {
            return [pscustomobject]@{
                webSocketDebuggerUrl = "ws://127.0.0.1:43123/devtools/browser/test-target"
            }
        }
        $script:pageProbeCount += 1
        $pageUrl = "https://remote.example/"
        if ($script:pageProbeCount -ge 2) { $pageUrl = "http://tauri.localhost/" }
        [pscustomobject]@{ type = "page"; url = $pageUrl }
    }
    $current = [System.Diagnostics.Process]::GetCurrentProcess()
    $endpoint = Wait-DeveWebView2CdpEndpoint `
        -HostProcess $current `
        -WebViewUserDataRoot $root `
        -Deadline ([DateTime]::UtcNow.AddSeconds(2)) `
        -Label "test-host" `
        -RequiredPageOrigins @("http://tauri.localhost")
    Assert-True ($endpoint.Endpoint -eq "http://127.0.0.1:43123") "waiter returned wrong endpoint"
    Assert-True ($script:requestedCdpUri -eq "http://127.0.0.1:43123/json/list") "waiter did not query page targets"
    Assert-True ($script:pageProbeCount -eq 2) "waiter accepted a stale page origin"
    Remove-Item Function:\Invoke-RestMethod

    function Invoke-RestMethod {
        param([string]$Uri, [int]$TimeoutSec)
        [pscustomobject]@{
            webSocketDebuggerUrl = "ws://127.0.0.1:43123/devtools/browser/different-target"
        }
    }
    Assert-Throws {
        Wait-DeveWebView2CdpEndpoint `
            -HostProcess $current `
            -WebViewUserDataRoot $root `
            -Deadline ([DateTime]::UtcNow.AddMilliseconds(400)) `
            -Label "mismatched-target" | Out-Null
    } "invalid WebSocket debugger URL" "browser target mismatch must fail closed"
    Remove-Item Function:\Invoke-RestMethod

    $shell = (Get-Command powershell.exe -ErrorAction Stop).Source
    $exited = Start-Process -FilePath $shell -ArgumentList @("-NoProfile", "-Command", "exit 7") -PassThru
    [void]$exited.WaitForExit(10000)
    Assert-Throws {
        Wait-DeveWebView2CdpEndpoint `
            -HostProcess $exited `
            -WebViewUserDataRoot $root `
            -Deadline ([DateTime]::UtcNow.AddSeconds(2)) `
            -Label "exited-host" | Out-Null
    } "exit code 7" "host exit must fail immediately with its exit code"

    $isolatedRoot = Join-Path $root "isolated-profile"
    $isolatedProfile = Join-Path $isolatedRoot "EBWebView"
    $otherProfile = "$isolatedProfile-other"
    $script:observedCimOperationTimeout = $null
    function Get-CimInstance {
        param([string]$ClassName, [string]$Filter, [int]$OperationTimeoutSec)
        $script:observedCimOperationTimeout = $OperationTimeoutSec
        @(
            [pscustomobject]@{
                ProcessId = 101
                CommandLine = "msedgewebview2.exe --user-data-dir=`"$isolatedProfile`""
            },
            [pscustomobject]@{
                ProcessId = 202
                CommandLine = "msedgewebview2.exe --user-data-dir=`"$otherProfile`""
            }
        )
    }
    $scoped = @(Get-DeveWebView2Processes -WebViewUserDataRoot $isolatedRoot)
    Assert-True ($scoped.Count -eq 1) "process discovery must remain exact-profile scoped"
    Assert-True ($scoped[0].ProcessId -eq 101) "process discovery selected the wrong profile"
    Remove-Item Function:\Get-CimInstance

    function Get-CimInstance {
        param([string]$ClassName, [string]$Filter, [int]$OperationTimeoutSec)
        $script:observedCimOperationTimeout = $OperationTimeoutSec
        [pscustomobject]@{
            ProcessId = $current.Id
            ParentProcessId = 0
            CreationDate = $current.StartTime
        }
    }
    $treeSnapshot = @(Get-DeveProcessTreeSnapshot -RootProcessId $current.Id)
    Assert-True ($script:observedCimOperationTimeout -eq 5) `
        "process-tree CIM snapshot must have an operation timeout"
    Assert-True ($treeSnapshot.Count -eq 1) "process-tree snapshot returned unexpected identities"
    Assert-True ($treeSnapshot[0].ProcessId -eq $current.Id) "process-tree snapshot lost its root"
    Assert-True (Test-DeveProcessIdentityAlive -Snapshot $treeSnapshot[0]) `
        "process identity snapshot must match its live process"
    Remove-Item Function:\Get-CimInstance

    $sleeping = Start-Process `
        -FilePath $shell `
        -ArgumentList @("-NoProfile", "-Command", "Start-Sleep -Seconds 30") `
        -PassThru
    Stop-DeveProcessIfAlive -ProcessId $sleeping.Id -TimeoutSeconds 5 -Label "test child"
    Assert-True $sleeping.HasExited "bounded process cleanup left the child alive"
    Stop-DeveWebView2Processes -WebViewUserDataRoot (Join-Path $root "empty-profile")

    $node = (Get-Command node.exe -ErrorAction Stop).Source
    $nodeOk = Join-Path $root "node-ok.mjs"
    $nodeHang = Join-Path $root "node-hang.mjs"
    $nodeChildHang = Join-Path $root "node-child-hang.mjs"
    $nodeRootHang = Join-Path $root "node-root-hang.mjs"
    $nodeChildPid = Join-Path $root "node-child.pid"
    $nodeRootPid = Join-Path $root "node-root.pid"
    [System.IO.File]::WriteAllText($nodeOk, 'console.log("node-journey-ok");', $utf8NoBom)
    $nodeChildPidLiteral = $nodeChildPid | ConvertTo-Json -Compress
    $nodeChildHangSource = @"
import { writeFileSync } from "node:fs";
writeFileSync($nodeChildPidLiteral, String(process.pid));
setInterval(() => {}, 1000);
"@
    [System.IO.File]::WriteAllText($nodeChildHang, $nodeChildHangSource, $utf8NoBom)
    $nodeChildHangLiteral = $nodeChildHang | ConvertTo-Json -Compress
    $nodeHangSource = @"
import { spawn } from "node:child_process";
spawn(process.execPath, [$nodeChildHangLiteral], { stdio: "ignore" });
setInterval(() => {}, 1000);
"@
    [System.IO.File]::WriteAllText($nodeHang, $nodeHangSource, $utf8NoBom)
    $nodeRootPidLiteral = $nodeRootPid | ConvertTo-Json -Compress
    $nodeRootHangSource = @"
import { writeFileSync } from "node:fs";
writeFileSync($nodeRootPidLiteral, String(process.pid));
setInterval(() => {}, 1000);
"@
    [System.IO.File]::WriteAllText($nodeRootHang, $nodeRootHangSource, $utf8NoBom)
    Invoke-DeveNodeJourney `
        -NodePath $node `
        -ScriptPath $nodeOk `
        -TimeoutSeconds 5 `
        -Label "test Node success"
    Assert-Throws {
        Invoke-DeveNodeJourney `
            -NodePath $node `
            -ScriptPath $nodeHang `
            -TimeoutSeconds 2 `
            -Label "test Node timeout"
    } "timed out after 2 seconds" "Node journey total timeout must fail closed"
    $childPid = [int](Get-Content -Raw -LiteralPath $nodeChildPid)
    Assert-NodeScriptStopped $childPid $nodeChildHang `
        "Node journey timeout left its child process alive"

    function Get-CimInstance {
        param([string]$ClassName, [string]$Filter, [int]$OperationTimeoutSec)
        throw "injected CIM failure"
    }
    Remove-Item -LiteralPath $nodeChildPid -Force
    Assert-Throws {
        Invoke-DeveNodeJourney `
            -NodePath $node `
            -ScriptPath $nodeHang `
            -TimeoutSeconds 2 `
            -Label "test Node snapshot failure"
    } "process-tree cleanup failed; snapshot=injected CIM failure" `
        "process-tree snapshot failure must clean up before failing closed"
    Remove-Item Function:\Get-CimInstance
    $snapshotFailureChildPid = [int](Get-Content -Raw -LiteralPath $nodeChildPid)
    Assert-NodeScriptStopped $snapshotFailureChildPid $nodeChildHang `
        "process-tree snapshot failure left its child process alive"

    $boundedTaskkill = (Get-Command Invoke-DeveBoundedTaskkill -CommandType Function).ScriptBlock
    function Invoke-DeveBoundedTaskkill {
        param([int]$RootProcessId, [int]$TimeoutSeconds)
        throw "injected taskkill launch failure"
    }
    try {
        Assert-Throws {
            Invoke-DeveNodeJourney `
                -NodePath $node `
                -ScriptPath $nodeRootHang `
                -TimeoutSeconds 1 `
                -Label "test taskkill launch failure"
        } "process-tree cleanup failed; taskkill=injected taskkill launch failure" `
            "taskkill launch failure must use direct-child fallback and fail closed"
    } finally {
        Set-Item Function:\Invoke-DeveBoundedTaskkill -Value $boundedTaskkill
    }
    $taskkillFailureRootPid = [int](Get-Content -Raw -LiteralPath $nodeRootPid)
    Assert-NodeScriptStopped $taskkillFailureRootPid $nodeRootHang `
        "taskkill launch failure left its direct Node process alive"

    $diagnosticPath = Join-Path $root "diagnostic.json"
    Write-DeveWebView2CdpDiagnostics `
        -HostProcess $current `
        -WebViewUserDataRoot (Join-Path $root "empty-profile") `
        -OutputPath $diagnosticPath `
        -Label "sanitized-test"
    $diagnostic = Get-Content -Raw -LiteralPath $diagnosticPath
    Assert-True ($diagnostic -notmatch '(?i)command.?line|credential|https?://') `
        "sanitized diagnostic must not expose command lines, credentials, or URLs"

    Write-Host "webview2-cdp-test: ok"
} finally {
    Remove-Item Function:\Invoke-RestMethod -ErrorAction SilentlyContinue
    Remove-Item Function:\Get-CimInstance -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}
