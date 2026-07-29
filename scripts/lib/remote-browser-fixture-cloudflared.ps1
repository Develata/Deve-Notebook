Set-StrictMode -Version Latest

# Bounded, checksum-bound cloudflared acquisition for the Windows
# RemoteBrowser candidate fixture.

function Invoke-RemoteFixtureBoundedDownload {
    param(
        [Parameter(Mandatory)][string]$Url,
        [Parameter(Mandatory)][string]$Destination,
        [ValidateRange(1, 3600)][int]$TimeoutSeconds = $script:RemoteFixtureCloudflaredDownloadTimeoutSeconds,
        [ValidateRange(1024, 1073741824)][long]$MaximumBytes = $script:RemoteFixtureCloudflaredDownloadLimitBytes
    )
    Remove-Item -LiteralPath $Destination -Force -ErrorAction SilentlyContinue
    $handler = [Net.Http.HttpClientHandler]::new()
    $handler.AllowAutoRedirect = $true
    $handler.MaxAutomaticRedirections = 5
    $client = [Net.Http.HttpClient]::new($handler)
    $client.Timeout = [Threading.Timeout]::InfiniteTimeSpan
    $cancellation = [Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds($TimeoutSeconds))
    $request = [Net.Http.HttpRequestMessage]::new([Net.Http.HttpMethod]::Get, $Url)
    $response = $null
    $inputStream = $null
    $outputStream = $null
    try {
        $response = $client.SendAsync(
            $request,
            [Net.Http.HttpCompletionOption]::ResponseHeadersRead,
            $cancellation.Token
        ).GetAwaiter().GetResult()
        [void]$response.EnsureSuccessStatusCode()
        $contentLength = $response.Content.Headers.ContentLength
        if ($null -ne $contentLength -and [long]$contentLength -gt $MaximumBytes) {
            throw "cloudflared download exceeds the $MaximumBytes byte limit"
        }
        $inputStream = $response.Content.ReadAsStreamAsync().GetAwaiter().GetResult()
        $outputStream = [IO.File]::Open(
            $Destination,
            [IO.FileMode]::CreateNew,
            [IO.FileAccess]::Write,
            [IO.FileShare]::None
        )
        $buffer = [byte[]]::new(65536)
        [long]$written = 0
        while (($read = $inputStream.ReadAsync($buffer, 0, $buffer.Length, $cancellation.Token).GetAwaiter().GetResult()) -gt 0) {
            $written += $read
            if ($written -gt $MaximumBytes) {
                throw "cloudflared download exceeds the $MaximumBytes byte limit"
            }
            $outputStream.Write($buffer, 0, $read)
        }
        $outputStream.Flush($true)
    } catch [OperationCanceledException] {
        throw "cloudflared download timed out after $TimeoutSeconds seconds"
    } catch [AggregateException] {
        if ($_.Exception.GetBaseException() -is [OperationCanceledException]) {
            throw "cloudflared download timed out after $TimeoutSeconds seconds"
        }
        throw
    } finally {
        if ($outputStream) { $outputStream.Dispose() }
        if ($inputStream) { $inputStream.Dispose() }
        if ($response) { $response.Dispose() }
        $request.Dispose()
        $cancellation.Dispose()
        $client.Dispose()
        $handler.Dispose()
    }
}

function Install-RemoteFixtureCloudflared {
    param(
        [Parameter(Mandatory)][string]$StateDirectory,
        [string]$SuppliedPath
    )
    if (-not [Environment]::Is64BitOperatingSystem) {
        throw "pinned cloudflared fixture currently supports Windows amd64 only"
    }
    $tools = Join-Path $StateDirectory "tools"
    New-Item -ItemType Directory -Force $tools | Out-Null
    $target = Join-Path $tools "cloudflared.exe"
    $temporary = "$target.tmp"
    $completed = $false
    try {
        if ($SuppliedPath) {
            $item = Get-Item -LiteralPath $SuppliedPath -Force
            if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or $item.PSIsContainer) {
                throw "supplied cloudflared must be a regular non-reparse file"
            }
            if ($item.Length -gt $script:RemoteFixtureCloudflaredDownloadLimitBytes) {
                throw "supplied cloudflared exceeds the $script:RemoteFixtureCloudflaredDownloadLimitBytes byte limit"
            }
            Copy-Item -LiteralPath $item.FullName -Destination $temporary -Force
        } else {
            $url = "https://github.com/cloudflare/cloudflared/releases/download/$script:RemoteFixtureCloudflaredVersion/cloudflared-windows-amd64.exe"
            Invoke-RemoteFixtureBoundedDownload -Url $url -Destination $temporary
        }
        $observed = (Get-FileHash -Algorithm SHA256 -LiteralPath $temporary).Hash
        if ($observed -ne $script:RemoteFixtureCloudflaredWindowsAmd64Sha256) {
            throw "cloudflared checksum mismatch: expected $script:RemoteFixtureCloudflaredWindowsAmd64Sha256, observed $observed"
        }
        Move-Item -LiteralPath $temporary -Destination $target -Force
        $completed = $true
        return $target
    } finally {
        if (-not $completed) {
            Remove-Item -LiteralPath $temporary -Force -ErrorAction SilentlyContinue
        }
    }
}
