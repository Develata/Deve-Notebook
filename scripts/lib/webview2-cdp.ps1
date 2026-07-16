function Get-DeveWebView2ActivePortPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$WebViewUserDataRoot
    )

    if ([string]::IsNullOrWhiteSpace($WebViewUserDataRoot)) {
        throw "webview2-cdp: WebViewUserDataRoot must not be empty"
    }
    Join-Path $WebViewUserDataRoot "EBWebView\DevToolsActivePort"
}

function Read-DeveWebView2ActivePort {
    param(
        [Parameter(Mandatory = $true)]
        [string]$WebViewUserDataRoot
    )

    $path = Get-DeveWebView2ActivePortPath -WebViewUserDataRoot $WebViewUserDataRoot
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        return $null
    }

    try {
        $lines = [System.IO.File]::ReadAllLines($path)
    } catch {
        throw "webview2-cdp: failed to read DevToolsActivePort: $($_.Exception.Message)"
    }
    if ($lines.Length -lt 2 -or [string]::IsNullOrWhiteSpace($lines[1])) {
        throw "webview2-cdp: DevToolsActivePort is incomplete"
    }

    $portText = $lines[0]
    if ($portText -notmatch '^[0-9]+$') {
        throw "webview2-cdp: DevToolsActivePort contains a non-numeric port"
    }
    $port = 0
    if (-not [int]::TryParse($portText, [ref]$port) -or $port -lt 1 -or $port -gt 65535) {
        throw "webview2-cdp: DevToolsActivePort port is outside 1..65535"
    }
    if ($lines[1] -notmatch '^/devtools/browser/[^\s]+$') {
        throw "webview2-cdp: DevToolsActivePort browser target is malformed"
    }

    [pscustomobject]@{
        Path = $path
        Port = $port
        BrowserTarget = $lines[1]
    }
}

function Wait-DeveWebView2CdpEndpoint {
    param(
        [Parameter(Mandatory = $true)]
        [System.Diagnostics.Process]$HostProcess,
        [Parameter(Mandatory = $true)]
        [string]$WebViewUserDataRoot,
        [Parameter(Mandatory = $true)]
        [DateTime]$Deadline,
        [string]$Label = "WebView2",
        [string[]]$RequiredPageOrigins = @()
    )

    $requiredOrigins = @()
    foreach ($originText in $RequiredPageOrigins) {
        $origin = $null
        if (
            [string]::IsNullOrWhiteSpace($originText) -or
            -not [Uri]::TryCreate($originText, [UriKind]::Absolute, [ref]$origin) -or
            -not [string]::Equals(
                $origin.AbsoluteUri.TrimEnd('/'),
                $origin.GetLeftPart([UriPartial]::Authority).TrimEnd('/'),
                [System.StringComparison]::OrdinalIgnoreCase
            )
        ) {
            throw "webview2-cdp: required page value must be an absolute origin"
        }
        $requiredOrigins += $origin.GetLeftPart([UriPartial]::Authority)
    }

    $lastObservation = "DevToolsActivePort has not been created"
    while ([DateTime]::UtcNow -lt $Deadline) {
        $HostProcess.Refresh()
        if ($HostProcess.HasExited) {
            throw "webview2-cdp: $Label host process exited before CDP became ready (exit code $($HostProcess.ExitCode))"
        }

        try {
            $activePort = Read-DeveWebView2ActivePort -WebViewUserDataRoot $WebViewUserDataRoot
            if ($null -eq $activePort) {
                $lastObservation = "DevToolsActivePort has not been created"
            } else {
                $endpoint = "http://127.0.0.1:$($activePort.Port)"
                $version = Invoke-RestMethod -Uri "$endpoint/json/version" -TimeoutSec 2
                $webSocketUrl = [string]$version.webSocketDebuggerUrl
                $webSocket = $null
                if (
                    [string]::IsNullOrWhiteSpace($webSocketUrl) -or
                    -not [Uri]::TryCreate($webSocketUrl, [UriKind]::Absolute, [ref]$webSocket) -or
                    ($webSocket.Scheme -ne "ws" -and $webSocket.Scheme -ne "wss") -or
                    -not $webSocket.IsLoopback -or
                    $webSocket.Port -ne $activePort.Port -or
                    $webSocket.AbsolutePath -ne $activePort.BrowserTarget
                ) {
                    throw "CDP version endpoint returned an invalid WebSocket debugger URL"
                }
                if ($requiredOrigins.Count -ne 0) {
                    $targets = @(Invoke-RestMethod -Uri "$endpoint/json/list" -TimeoutSec 2)
                    $foundRequiredPage = $false
                    foreach ($target in $targets) {
                        if ([string]$target.type -ne "page") { continue }
                        $pageUri = $null
                        if (-not [Uri]::TryCreate([string]$target.url, [UriKind]::Absolute, [ref]$pageUri)) {
                            continue
                        }
                        $pageOrigin = $pageUri.GetLeftPart([UriPartial]::Authority)
                        foreach ($requiredOrigin in $requiredOrigins) {
                            if ([string]::Equals(
                                $pageOrigin,
                                $requiredOrigin,
                                [System.StringComparison]::OrdinalIgnoreCase
                            )) {
                                $foundRequiredPage = $true
                                break
                            }
                        }
                        if ($foundRequiredPage) { break }
                    }
                    if (-not $foundRequiredPage) {
                        throw "CDP endpoint does not expose a page at the required origin"
                    }
                }
                return [pscustomobject]@{
                    Port = $activePort.Port
                    Endpoint = $endpoint
                    ActivePortPath = $activePort.Path
                    BrowserTarget = $activePort.BrowserTarget
                }
            }
        } catch {
            $lastObservation = $_.Exception.Message
        }
        Start-Sleep -Milliseconds 250
    }

    $HostProcess.Refresh()
    if ($HostProcess.HasExited) {
        throw "webview2-cdp: $Label host process exited before CDP became ready (exit code $($HostProcess.ExitCode))"
    }
    throw "webview2-cdp: timed out waiting for $Label CDP endpoint; last observation: $lastObservation"
}

function Write-DeveWebView2CdpDiagnostics {
    param(
        [Parameter(Mandatory = $true)]
        [System.Diagnostics.Process]$HostProcess,
        [Parameter(Mandatory = $true)]
        [string]$WebViewUserDataRoot,
        [Parameter(Mandatory = $true)]
        [string]$OutputPath,
        [string]$Label = "WebView2"
    )

    $hostRunning = $false
    $hostExitCode = $null
    try {
        $HostProcess.Refresh()
        $hostRunning = -not $HostProcess.HasExited
        if (-not $hostRunning) {
            $hostExitCode = $HostProcess.ExitCode
        }
    } catch {
        $hostRunning = $false
    }

    $activePortPath = Get-DeveWebView2ActivePortPath -WebViewUserDataRoot $WebViewUserDataRoot
    $activePortPresent = Test-Path -LiteralPath $activePortPath -PathType Leaf
    $activePortValid = $false
    $activePort = $null
    $activePortError = $null
    try {
        $parsed = Read-DeveWebView2ActivePort -WebViewUserDataRoot $WebViewUserDataRoot
        if ($null -ne $parsed) {
            $activePortValid = $true
            $activePort = $parsed.Port
        }
    } catch {
        $activePortError = $_.Exception.Message
    }

    $webViewProcessCount = $null
    try {
        $root = [System.IO.Path]::GetFullPath($WebViewUserDataRoot)
        $webViewProcessCount = @(
            Get-CimInstance Win32_Process -Filter "Name = 'msedgewebview2.exe'" |
                Where-Object {
                    $null -ne $_.CommandLine -and
                    $_.CommandLine.IndexOf(
                        $root,
                        [System.StringComparison]::OrdinalIgnoreCase
                    ) -ge 0
                }
        ).Count
    } catch {
        $webViewProcessCount = $null
    }

    $diagnostic = [ordered]@{
        schema = 1
        label = $Label
        host_process_id = $HostProcess.Id
        host_running = $hostRunning
        host_exit_code = $hostExitCode
        active_port_file_present = $activePortPresent
        active_port_valid = $activePortValid
        active_port = $activePort
        active_port_error = $activePortError
        scoped_webview2_process_count = $webViewProcessCount
    }
    $json = $diagnostic | ConvertTo-Json -Compress
    $parent = Split-Path -Parent $OutputPath
    if (-not [string]::IsNullOrWhiteSpace($parent)) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    [System.IO.File]::WriteAllText($OutputPath, $json, [System.Text.UTF8Encoding]::new($false))
    Write-Warning "webview2-cdp: sanitized diagnostic: $json"
}
