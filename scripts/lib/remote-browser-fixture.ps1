Set-StrictMode -Version Latest

# Single source for the fixed-name secret material a fixture run may leave in
# its state directory; every cleanup path must consume this list instead of
# hardcoding its own copy.
$script:RemoteFixtureSecretFileNames = @(
    ".password", ".password.hasher.stdout", ".password.hasher.stderr",
    ".backend.env", "credentials.json", "fixture-env.json"
)

$script:RemoteFixtureCloudflaredVersion = "2026.7.2"
$script:RemoteFixtureCloudflaredWindowsAmd64Sha256 = "CDB5D4432F6AE1595654A692A51308B69D2BF7AF961F5578D9391837CF072DF9"
$script:RemoteFixtureCloudflaredDownloadTimeoutSeconds = 180
$script:RemoteFixtureCloudflaredDownloadLimitBytes = 134217728
. (Join-Path $PSScriptRoot "remote-browser-fixture-cloudflared.ps1")

function New-RemoteFixtureRandomHex {
    param([Parameter(Mandatory)][int]$Bytes)
    $buffer = [byte[]]::new($Bytes)
    [Security.Cryptography.RandomNumberGenerator]::Fill($buffer)
    return [Convert]::ToHexString($buffer).ToLowerInvariant()
}

function Protect-RemoteFixturePath {
    param([Parameter(Mandatory)][string]$Path)
    $sid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    & icacls.exe $Path /inheritance:r /grant:r "*$sid`:(F)" | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "failed to restrict ACL for $Path" }
}
. (Join-Path $PSScriptRoot "remote-browser-fixture-state.ps1")

function Resolve-RemoteFixtureStateDirectory {
    param([Parameter(Mandatory)][string]$Path)
    if ($Path.IndexOfAny([char[]]"`r`n`t") -ge 0) {
        throw "state directory contains a control character"
    }
    if (Test-Path -LiteralPath $Path) {
        $item = Get-Item -LiteralPath $Path -Force
        if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "state directory must not be a symlink or reparse point: $Path"
        }
    } else {
        New-Item -ItemType Directory -Path $Path -Force | Out-Null
    }
    return (Get-Item -LiteralPath $Path -Force).FullName
}

function Assert-RemoteFixtureHttpsOrigin {
    param([Parameter(Mandatory)][string]$Origin)
    $uri = $null
    if (-not [Uri]::TryCreate($Origin, [UriKind]::Absolute, [ref]$uri) -or
        $uri.Scheme -ne "https" -or
        $uri.AbsolutePath -ne "/" -or
        $uri.Query -or $uri.Fragment -or $uri.UserInfo) {
        throw "expected an exact HTTPS origin, got: $Origin"
    }
}

function Assert-RemoteFixtureExpectedHead {
    param(
        [Parameter(Mandatory)][string]$RepoRoot,
        [Parameter(Mandatory)][string]$ExpectedHead
    )
    if ($ExpectedHead -notmatch '^[0-9a-fA-F]{40}$') {
        throw "expected HEAD must be a full 40-character commit SHA"
    }
    $actual = (& git -C $RepoRoot rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $actual -ine $ExpectedHead) {
        throw "workspace HEAD mismatch: expected $ExpectedHead, observed $actual"
    }
}

function Get-RemoteFixtureFreePort {
    $listener = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, 0)
    try {
        $listener.Start()
        return ([Net.IPEndPoint]$listener.LocalEndpoint).Port
    } finally {
        $listener.Stop()
    }
}

function ConvertTo-RemoteFixtureWindowsArgument {
    param([AllowEmptyString()][Parameter(Mandatory)][string]$Value)
    if ($Value.Length -gt 0 -and $Value -notmatch '[\s"]') { return $Value }
    $builder = [Text.StringBuilder]::new('"')
    $backslashes = 0
    foreach ($character in $Value.ToCharArray()) {
        if ($character -eq '\') {
            $backslashes++
        } elseif ($character -eq '"') {
            [void]$builder.Append(('\' * (($backslashes * 2) + 1)))
            [void]$builder.Append('"')
            $backslashes = 0
        } else {
            [void]$builder.Append(('\' * $backslashes))
            [void]$builder.Append($character)
            $backslashes = 0
        }
    }
    [void]$builder.Append(('\' * ($backslashes * 2)))
    [void]$builder.Append('"')
    return $builder.ToString()
}

# Children spawned with redirected stdio inherit every inheritable handle of
# this process (CreateProcess bInheritHandles), including our own stdout and
# stderr endpoints when a caller captures us through a pipe. A long-lived
# descendant (fixture backend, tunnel) then keeps that pipe open and blocks the
# caller's read-to-EOF long after this process exits — on CI that turned a
# finished fixture start into a job-timeout hang. Clearing HANDLE_FLAG_INHERIT
# on our stdout/stderr before each spawn keeps those endpoints with us alone.
function Clear-RemoteFixtureStdHandleInheritance {
    param(
        [scriptblock]$GetStdHandle,
        [scriptblock]$SetHandleInformation,
        [scriptblock]$GetLastError
    )
    if ((-not $GetStdHandle -or -not $SetHandleInformation) -and
        -not ("DeveFixture.NativeStdHandles" -as [type])) {
        Add-Type -Namespace DeveFixture -Name NativeStdHandles -MemberDefinition @'
[DllImport("kernel32.dll", SetLastError = true)]
public static extern IntPtr GetStdHandle(int nStdHandle);
[DllImport("kernel32.dll", SetLastError = true)]
public static extern bool SetHandleInformation(IntPtr hObject, uint dwMask, uint dwFlags);
'@
    }
    foreach ($slot in @(-11, -12)) {
        $handle = if ($GetStdHandle) {
            & $GetStdHandle $slot
        } else {
            [DeveFixture.NativeStdHandles]::GetStdHandle($slot)
        }
        if ($handle -eq [IntPtr]::Zero -or $handle -eq [IntPtr]::new(-1)) { continue }
        $cleared = if ($SetHandleInformation) {
            [bool](& $SetHandleInformation $handle 1 0)
        } else {
            [DeveFixture.NativeStdHandles]::SetHandleInformation($handle, 1, 0)
        }
        if (-not $cleared) {
            $errorCode = if ($GetLastError) {
                [int](& $GetLastError)
            } else {
                [Runtime.InteropServices.Marshal]::GetLastWin32Error()
            }
            throw "could not clear std-handle inheritance for slot $slot (Win32 error $errorCode)"
        }
    }
}

function Start-RemoteFixtureProcess {
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [Parameter()][string[]]$ArgumentList = @(),
        [Parameter(Mandatory)][string]$WorkingDirectory,
        [Parameter(Mandatory)][string]$StdoutPath,
        [Parameter(Mandatory)][string]$StderrPath,
        [Parameter()][hashtable]$Environment = @{}
    )
    Clear-RemoteFixtureStdHandleInheritance
    $encodedArguments = @($ArgumentList | ForEach-Object { ConvertTo-RemoteFixtureWindowsArgument $_ }) -join ' '
    $parameters = @{
        FilePath = $FilePath
        ArgumentList = $encodedArguments
        WorkingDirectory = $WorkingDirectory
        RedirectStandardOutput = $StdoutPath
        RedirectStandardError = $StderrPath
        PassThru = $true
        WindowStyle = "Hidden"
    }
    $startProcess = Get-Command Start-Process
    if ($Environment.Count -gt 0 -and $startProcess.Parameters.ContainsKey("Environment")) {
        $parameters.Environment = $Environment
        return Start-Process @parameters
    }

    $previous = @{}
    try {
        foreach ($entry in $Environment.GetEnumerator()) {
            $previous[$entry.Key] = [Environment]::GetEnvironmentVariable($entry.Key, "Process")
            [Environment]::SetEnvironmentVariable($entry.Key, [string]$entry.Value, "Process")
        }
        return Start-Process @parameters
    } finally {
        foreach ($entry in $Environment.GetEnumerator()) {
            [Environment]::SetEnvironmentVariable($entry.Key, $previous[$entry.Key], "Process")
        }
    }
}

function Get-RemoteFixtureOutputBytes {
    param([Parameter(Mandatory)][string[]]$Paths)
    [long]$total = 0
    foreach ($path in $Paths) {
        if (Test-Path -LiteralPath $path -PathType Leaf) {
            $total += (Get-Item -LiteralPath $path -Force).Length
        }
    }
    return $total
}

function Limit-RemoteFixtureOutputFiles {
    param(
        [Parameter(Mandatory)][string[]]$Paths,
        [Parameter(Mandatory)][long]$CombinedLimitBytes
    )
    $perFileLimit = [Math]::Floor($CombinedLimitBytes / $Paths.Count)
    foreach ($path in $Paths) {
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { continue }
        $stream = [IO.File]::Open($path, [IO.FileMode]::Open, [IO.FileAccess]::Write, [IO.FileShare]::Read)
        try {
            if ($stream.Length -gt $perFileLimit) { $stream.SetLength($perFileLimit) }
        } finally {
            $stream.Dispose()
        }
    }
}

function Stop-RemoteFixtureBoundedProcessTree {
    param(
        [Parameter(Mandatory)][Diagnostics.Process]$Process,
        [Parameter(Mandatory)][string]$Label
    )
    $Process.Refresh()
    if ($Process.HasExited) { return }
    try {
        $Process.Kill($true)
    } catch {
        # Kill(entireProcessTree) reports per-descendant failures through an
        # AggregateException even when the root died; a partial tree kill must
        # stay a failure instead of a silent all-clear.
        $exception = $_.Exception
        while ($null -ne $exception -and $exception -isnot [AggregateException]) {
            $exception = $exception.InnerException
        }
        if ($null -ne $exception) {
            throw "failed to terminate $Label process tree descendants: $($_.Exception.Message)"
        }
        $Process.Refresh()
        if (-not $Process.HasExited) {
            throw "failed to terminate $Label process tree: $($_.Exception.Message)"
        }
    }
    if (-not $Process.WaitForExit(7000)) {
        throw "$Label process tree survived bounded termination"
    }
}

function Invoke-RemoteFixtureBoundedProcess {
    param(
        [Parameter(Mandatory)][string]$Label,
        [Parameter(Mandatory)][string]$FilePath,
        [Parameter()][string[]]$ArgumentList = @(),
        [Parameter(Mandatory)][string]$WorkingDirectory,
        [Parameter(Mandatory)][string]$StdoutPath,
        [Parameter(Mandatory)][string]$StderrPath,
        [Parameter()][hashtable]$Environment = @{},
        [ValidateRange(1, 3600)][int]$TimeoutSeconds = 60,
        [ValidateRange(1024, 268435456)][long]$OutputLimitBytes = 4194304
    )
    foreach ($path in @($StdoutPath, $StderrPath)) {
        Remove-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue
    }
    $process = Start-RemoteFixtureProcess -FilePath $FilePath -ArgumentList $ArgumentList `
        -WorkingDirectory $WorkingDirectory -StdoutPath $StdoutPath -StderrPath $StderrPath `
        -Environment $Environment
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSeconds)
    $failure = $null
    while (-not $process.HasExited) {
        $outputBytes = Get-RemoteFixtureOutputBytes -Paths @($StdoutPath, $StderrPath)
        if ($outputBytes -gt $OutputLimitBytes) {
            $failure = "exceeded the combined output limit of $OutputLimitBytes bytes"
            break
        }
        if ([DateTimeOffset]::UtcNow -ge $deadline) {
            $failure = "timed out after $TimeoutSeconds seconds"
            break
        }
        Start-Sleep -Milliseconds 50
        $process.Refresh()
    }
    if ($failure) {
        Stop-RemoteFixtureBoundedProcessTree -Process $process -Label $Label
        Limit-RemoteFixtureOutputFiles -Paths @($StdoutPath, $StderrPath) -CombinedLimitBytes $OutputLimitBytes
        throw "$Label $failure"
    }
    # WaitForExit() after HasExited ensures redirected file handles are flushed.
    $process.WaitForExit()
    $outputBytes = Get-RemoteFixtureOutputBytes -Paths @($StdoutPath, $StderrPath)
    if ($outputBytes -gt $OutputLimitBytes) {
        Limit-RemoteFixtureOutputFiles -Paths @($StdoutPath, $StderrPath) -CombinedLimitBytes $OutputLimitBytes
        throw "$Label exceeded the combined output limit of $OutputLimitBytes bytes"
    }
    return [pscustomobject]@{
        ExitCode = $process.ExitCode
        StdoutPath = $StdoutPath
        StderrPath = $StderrPath
        OutputBytes = $outputBytes
    }
}

function Get-RemoteFixtureProcessToken {
    param([Parameter(Mandatory)][int]$ProcessId)
    $process = Get-Process -Id $ProcessId -ErrorAction Stop
    return $process.StartTime.ToUniversalTime().Ticks.ToString([Globalization.CultureInfo]::InvariantCulture)
}

function Stop-RemoteFixtureProcess {
    param(
        [Parameter(Mandatory)][string]$Label,
        [AllowNull()][object]$ProcessId,
        [AllowNull()][string]$ExpectedToken
    )
    if ($null -eq $ProcessId) { return }
    $process = Get-Process -Id ([int]$ProcessId) -ErrorAction SilentlyContinue
    if ($null -eq $process) { return }
    $actual = $process.StartTime.ToUniversalTime().Ticks.ToString([Globalization.CultureInfo]::InvariantCulture)
    if (-not $ExpectedToken -or $actual -ne $ExpectedToken) {
        throw "refusing to stop reused or unowned $Label PID $ProcessId"
    }
    $process.Kill($true)
    if (-not $process.WaitForExit(7000)) {
        throw "$Label PID $ProcessId survived bounded cleanup"
    }
}

function Wait-RemoteFixtureHttp {
    param(
        [Parameter(Mandatory)][string]$Url,
        [AllowNull()][Diagnostics.Process]$Process,
        [Parameter(Mandatory)][string]$LogPath
    )
    for ($attempt = 0; $attempt -lt 120; $attempt++) {
        try {
            Invoke-WebRequest -Uri $Url -Method Get -TimeoutSec 2 -UseBasicParsing | Out-Null
            return
        } catch {
            if ($null -ne $Process -and $Process.HasExited) {
                throw "process exited before health check succeeded; log: $LogPath"
            }
            Start-Sleep -Milliseconds 250
        }
    }
    throw "timed out waiting for $Url; log: $LogPath"
}

function Wait-RemoteFixtureTunnelOrigin {
    param(
        [Parameter(Mandatory)][Diagnostics.Process]$Process,
        [Parameter(Mandatory)][string[]]$LogPaths
    )
    $pattern = 'https://[A-Za-z0-9-]+\.trycloudflare\.com'
    for ($attempt = 0; $attempt -lt 120; $attempt++) {
        foreach ($path in $LogPaths) {
            if (Test-Path -LiteralPath $path) {
                $content = Get-Content -Raw -LiteralPath $path
                if ([string]::IsNullOrEmpty($content)) { continue }
                $match = [regex]::Match($content, $pattern)
                if ($match.Success) {
                    Assert-RemoteFixtureHttpsOrigin $match.Value
                    return $match.Value
                }
            }
        }
        if ($Process.HasExited) { throw "cloudflared exited before publishing an HTTPS origin" }
        Start-Sleep -Milliseconds 250
    }
    throw "timed out waiting for cloudflared quick-tunnel origin"
}

# Removes an owned backend container only after proving the fixture owner
# label matches, and only reports success once the container is gone. Shared
# by the start failure path, the stop command, and the bounded-start recovery.
function Remove-RemoteFixtureOwnedContainer {
    param(
        [Parameter(Mandatory)][string]$ContainerName,
        [Parameter(Mandatory)][string]$FixtureId
    )
    if (Test-RemoteFixtureContainerExists $ContainerName) {
        Assert-RemoteFixtureContainerOwner -ContainerName $ContainerName -FixtureId $FixtureId
        & docker rm --force $ContainerName | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "failed to remove owned backend container" }
    }
    if (Test-RemoteFixtureContainerExists $ContainerName) { throw "owned backend container survived cleanup" }
}

function Assert-RemoteFixtureContainerOwner {
    param(
        [Parameter(Mandatory)][string]$ContainerName,
        [Parameter(Mandatory)][string]$FixtureId
    )
    $owner = (& docker inspect --format '{{ index .Config.Labels "deve.remote-fixture-id" }}' $ContainerName 2>$null).Trim()
    if ($LASTEXITCODE -ne 0 -or $owner -ne $FixtureId) {
        throw "refusing to remove container without matching fixture owner label: $ContainerName"
    }
}

function Test-RemoteFixtureContainerExists {
    param([Parameter(Mandatory)][string]$ContainerName)
    $names = @(& docker ps --all --filter "name=^/$ContainerName`$" --format '{{.Names}}' 2>$null)
    if ($LASTEXITCODE -ne 0) { throw "failed to query Docker while checking owned container: $ContainerName" }
    $matches = @($names | Where-Object { $_ -eq $ContainerName })
    if ($matches.Count -eq 1 -and $names.Count -eq 1) { return $true }
    if ($names.Count -eq 0) { return $false }
    throw "Docker returned an ambiguous exact-name result for: $ContainerName"
}
