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

    Write-Host "webview2-cdp-test: ok"
} finally {
    Remove-Item Function:\Invoke-RestMethod -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}
