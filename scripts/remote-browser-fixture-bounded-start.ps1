param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$RemainingArguments = @()
)

# Bounded whole-start wrapper for the RemoteBrowser candidate fixture: runs
# `remote-browser-fixture.ps1 start` under one total deadline, relays only
# allowlisted stage lines, and on timeout kills the worker tree and recovers
# owned resources/secrets from the atomic startup ownership state. The success
# stream stays exactly one environment-file path. No release/product authority.
#
# Accepted residuals: a worker killed outside its finally can orphan a child
# spawned inside the acquire-before-record window (no Job Object design); a
# failed recovery preserves startup-state.json + .fixture-owner for manual
# retry because the stop command's authority begins only at fixture-state.json;
# tail redaction is defense-in-depth — the fixture contract already keeps
# credential values off stderr, and the worker usually deletes its credential
# files before exiting, so the redaction list is often empty by design.

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
$RootDirectory = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
. (Join-Path $PSScriptRoot "lib/remote-browser-fixture.ps1")
. (Join-Path $PSScriptRoot "lib/remote-browser-fixture-progress.ps1")

$StdoutLimitBytes = 65536
$FailureTailChars = 2048
$RedactionWindowBytes = 16384
$MaximumRelayedStageLines = 64
$RelayChunkBytes = 65536
$MaximumPendingChars = 4096
$WorkerOutputLimitBytes = 8388608

if ($null -eq $RemainingArguments) { $RemainingArguments = @() }
$passthrough = [Collections.Generic.List[string]]::new()
$totalDeadlineSeconds = 1200
$workerScript = Join-Path $PSScriptRoot "remote-browser-fixture.ps1"
$stateDirectoryArgument = $null
for ($index = 0; $index -lt $RemainingArguments.Count; $index++) {
    $name = $RemainingArguments[$index]
    switch ($name) {
        "--total-deadline-seconds" {
            if ($index + 1 -ge $RemainingArguments.Count) { throw "missing value for $name" }
            $parsed = 0
            if (-not [int]::TryParse($RemainingArguments[++$index], [ref]$parsed)) {
                throw "--total-deadline-seconds must be an integer"
            }
            $totalDeadlineSeconds = $parsed
        }
        "--worker-script" {
            if ($index + 1 -ge $RemainingArguments.Count) { throw "missing value for $name" }
            if ($env:DEVE_REMOTE_FIXTURE_TEST_WORKER -ne "1") {
                throw "--worker-script is a test-only override and requires DEVE_REMOTE_FIXTURE_TEST_WORKER=1"
            }
            $workerScript = $RemainingArguments[++$index]
        }
        "--state-dir" {
            if ($index + 1 -ge $RemainingArguments.Count) { throw "missing value for $name" }
            $stateDirectoryArgument = $RemainingArguments[$index + 1]
            $passthrough.Add($name)
            $passthrough.Add($RemainingArguments[++$index])
        }
        default { $passthrough.Add($name) }
    }
}
if (-not $stateDirectoryArgument) { throw "bounded start requires --state-dir" }
if ($totalDeadlineSeconds -lt 1 -or $totalDeadlineSeconds -gt 7200) {
    throw "--total-deadline-seconds must be in 1..7200"
}
if (-not (Test-Path -LiteralPath $workerScript -PathType Leaf)) {
    throw "fixture worker script does not exist: $workerScript"
}

# Controlled failures leave as one raw single-line stderr record so CI logs
# and callers never see them re-wrapped by exception rendering.
function Exit-BoundedStartFailure {
    param([Parameter(Mandatory)][string]$Message)
    [Console]::Error.WriteLine("remote-browser-fixture-bounded-start: $Message")
    exit 1
}

function Read-NewWorkerStageLines {
    param([Parameter(Mandatory)][string]$Path, [Parameter(Mandatory)][hashtable]$Cursor)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return }
    $stream = [IO.File]::Open($Path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::ReadWrite)
    try {
        if ($stream.Length -le $Cursor.offset) { return }
        [void]$stream.Seek($Cursor.offset, [IO.SeekOrigin]::Begin)
        $buffer = [byte[]]::new([Math]::Min($stream.Length - $Cursor.offset, $RelayChunkBytes))
        $read = $stream.Read($buffer, 0, $buffer.Length)
        $Cursor.offset += $read
        $Cursor.pending += [Text.Encoding]::UTF8.GetString($buffer, 0, $read)
    } finally {
        $stream.Dispose()
    }
    while (($newlineIndex = $Cursor.pending.IndexOf("`n")) -ge 0) {
        $line = $Cursor.pending.Substring(0, $newlineIndex).TrimEnd("`r")
        $Cursor.pending = $Cursor.pending.Substring($newlineIndex + 1)
        if ($line -notmatch '^deve-remote-fixture-stage: ([a-z][a-z0-9-]*)$') { continue }
        $stage = $Matches[1]
        if ($script:RemoteFixtureStartupStageNames -notcontains $stage) { continue }
        $Cursor.lastStage = $stage
        if ($Cursor.relayed -lt $MaximumRelayedStageLines) {
            $Cursor.relayed++
            [Console]::Error.WriteLine($line)
        }
    }
    # A stage line is short; any longer newline-free run can never become one.
    if ($Cursor.pending.Length -gt $MaximumPendingChars) { $Cursor.pending = "" }
}

function Get-BoundedStartSecretValues {
    param([Parameter(Mandatory)][string]$StateDirectory)
    $secrets = [Collections.Generic.List[string]]::new()
    $credentialsPath = Join-Path $StateDirectory "credentials.json"
    if (Test-Path -LiteralPath $credentialsPath -PathType Leaf) {
        try {
            $credentials = Get-Content -Raw -LiteralPath $credentialsPath | ConvertFrom-Json
            foreach ($field in @("username", "password", "auth_secret")) {
                $value = $credentials.PSObject.Properties[$field]
                if ($value -and $value.Value -is [string] -and $value.Value.Trim()) { $secrets.Add($value.Value) }
            }
        } catch {
            # Best-effort redaction source only; an unreadable credentials file
            # degrades to the AUTH_* line filter in the tail writer.
        }
    }
    $passwordPath = Join-Path $StateDirectory ".password"
    if (Test-Path -LiteralPath $passwordPath -PathType Leaf) {
        $password = (Get-Content -Raw -LiteralPath $passwordPath -ErrorAction SilentlyContinue)
        if ($password -and $password.Trim()) { $secrets.Add($password.Trim()) }
    }
    return , $secrets
}

function Write-BoundedStartRedactedTail {
    param(
        [Parameter(Mandatory)][string]$StderrPath,
        [Parameter(Mandatory)][AllowNull()][AllowEmptyCollection()][string[]]$Secrets
    )
    if (-not (Test-Path -LiteralPath $StderrPath -PathType Leaf)) { return }
    $stream = [IO.File]::Open($StderrPath, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::ReadWrite)
    try {
        $window = [Math]::Min($stream.Length, $RedactionWindowBytes)
        if ($window -le 0) { return }
        [void]$stream.Seek(-$window, [IO.SeekOrigin]::End)
        $buffer = [byte[]]::new($window)
        $read = $stream.Read($buffer, 0, $buffer.Length)
        $tail = [Text.Encoding]::UTF8.GetString($buffer, 0, $read)
    } finally {
        $stream.Dispose()
    }
    # Drop whole lines naming fixture auth material (for example a parameter
    # binding error rendering an environment value) before value redaction.
    $tail = (($tail -split "`n") | Where-Object { $_ -notmatch 'AUTH_(USER|PASS|SECRET)' }) -join "`n"
    foreach ($secret in @($Secrets)) {
        if ($secret) { $tail = $tail.Replace($secret, "[redacted]") }
    }
    if ($tail.Length -gt $FailureTailChars) { $tail = $tail.Substring($tail.Length - $FailureTailChars) }
    [Console]::Error.WriteLine("remote-browser-fixture-bounded-start: redacted worker stderr tail:")
    [Console]::Error.WriteLine($tail)
}

# Recovers owned resources and secret material from the persisted startup
# ownership state after abrupt worker termination. Returns "deferred" once
# fixture-state.json exists (the ordinary stop command is then the cleanup
# authority) and "recovered" after successful recovery. Throws when cleanup
# fails; the startup state and owner marker are then preserved.
function Invoke-BoundedStartRecovery {
    param([Parameter(Mandatory)][string]$StateDirectory)
    $finalStatePath = Join-Path $StateDirectory "fixture-state.json"
    if (Test-Path -LiteralPath $finalStatePath -PathType Leaf) {
        try {
            [void](Read-RemoteFixtureFinalState -StateDirectory $StateDirectory)
            return "deferred"
        } catch {
            throw "published fixture state is invalid; ownership state was preserved: $($_.Exception.Message)"
        }
    }
    $cleanupErrors = [Collections.Generic.List[string]]::new()
    $ownerPath = Join-Path $StateDirectory ".fixture-owner"
    $state = $null
    try {
        $state = Read-RemoteFixtureStartupState -StateDirectory $StateDirectory
    } catch {
        $cleanupErrors.Add($_.Exception.Message)
    }
    if ($null -ne $state) {
        if (-not (Test-Path -LiteralPath $ownerPath -PathType Leaf)) {
            $cleanupErrors.Add("fixture owner marker is missing for startup state")
            $state = $null
        } elseif ((Get-Content -Raw -LiteralPath $ownerPath).Trim() -ne $state.fixture_id) {
            # A stale prior-round state must not drive this round's cleanup.
            $cleanupErrors.Add("fixture owner marker does not match startup state")
            $state = $null
        }
    } elseif (Test-Path -LiteralPath $ownerPath -PathType Leaf) {
        $cleanupErrors.Add("fixture owner marker exists without valid startup state")
    }
    if ($null -ne $state) {
        try {
            Assert-RemoteFixtureStartupRecoveryAuthority -State $state
        } catch {
            $cleanupErrors.Add($_.Exception.Message)
            $state = $null
        }
    }
    $ownedSecretPaths = @($script:RemoteFixtureSecretFileNames | ForEach-Object { Join-Path $StateDirectory $_ })
    if ($null -eq $state -and ($ownedSecretPaths | Where-Object { Test-Path -LiteralPath $_ })) {
        $cleanupErrors.Add("secret material exists without matching startup ownership state")
    }
    if ($null -ne $state) {
        foreach ($secretPath in $ownedSecretPaths) {
            try {
                if (Test-Path -LiteralPath $secretPath) {
                    Remove-Item -LiteralPath $secretPath -Force -ErrorAction Stop
                }
            } catch { $cleanupErrors.Add($_.Exception.Message) }
        }
    }
    if ($null -ne $state) {
        try { Stop-RemoteFixtureProcess -Label "tunnel" -ProcessId $state.tunnel_pid -ExpectedToken $state.tunnel_process_token } catch { $cleanupErrors.Add($_.Exception.Message) }
        try { Stop-RemoteFixtureProcess -Label "backend" -ProcessId $state.backend_pid -ExpectedToken $state.backend_process_token } catch { $cleanupErrors.Add($_.Exception.Message) }
        if ($state.container_name) {
            try {
                Remove-RemoteFixtureOwnedContainer -ContainerName $state.container_name -FixtureId $state.fixture_id
            } catch { $cleanupErrors.Add($_.Exception.Message) }
        }
        foreach ($processId in @($state.backend_pid, $state.tunnel_pid)) {
            if ($null -ne $processId -and (Get-Process -Id ([int]$processId) -ErrorAction SilentlyContinue)) {
                $cleanupErrors.Add("owned fixture process survived cleanup: $processId")
            }
        }
    }
    if ($cleanupErrors.Count -gt 0) {
        throw "startup recovery failed; ownership state was preserved: $($cleanupErrors -join '; ')"
    }
    Remove-RemoteFixtureStartupState -StateDirectory $StateDirectory
    Remove-Item -LiteralPath $ownerPath -Force -ErrorAction SilentlyContinue
    return "recovered"
}

$stateDirectory = Resolve-RemoteFixtureStateDirectory $stateDirectoryArgument
Protect-RemoteFixturePath $stateDirectory
$workerStdout = Join-Path $stateDirectory ".bounded-start.stdout.log"
$workerStderr = Join-Path $stateDirectory ".bounded-start.stderr.log"
Remove-Item -LiteralPath $workerStdout, $workerStderr -Force -ErrorAction SilentlyContinue

$worker = Start-RemoteFixtureProcess -FilePath (Get-Process -Id $PID).Path `
    -ArgumentList (@("-NoProfile", "-File", $workerScript, "start") + $passthrough.ToArray()) `
    -WorkingDirectory $RootDirectory -StdoutPath $workerStdout -StderrPath $workerStderr
try {
    $cursor = @{ offset = [long]0; pending = ""; lastStage = $null; relayed = 0 }
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($totalDeadlineSeconds)
    $timedOut = $false
    $outputExceeded = $false
    while (-not $worker.HasExited) {
        Read-NewWorkerStageLines -Path $workerStderr -Cursor $cursor
        if ((Get-RemoteFixtureOutputBytes -Paths @($workerStdout, $workerStderr)) -gt $WorkerOutputLimitBytes) {
            $outputExceeded = $true
            break
        }
        if ([DateTimeOffset]::UtcNow -ge $deadline) { $timedOut = $true; break }
        Start-Sleep -Milliseconds 250
        $worker.Refresh()
    }
    Read-NewWorkerStageLines -Path $workerStderr -Cursor $cursor
    $lastStage = if ($cursor.lastStage) { $cursor.lastStage } else { "validate-head (no stage recorded)" }

    if ($timedOut -or $outputExceeded) {
        $failure = if ($outputExceeded) {
            "fixture start worker exceeded the $WorkerOutputLimitBytes byte output limit at stage '$lastStage'"
        } else {
            "fixture start exceeded the ${totalDeadlineSeconds}s total deadline at stage '$lastStage'"
        }
        try {
            Stop-RemoteFixtureBoundedProcessTree -Process $worker -Label "fixture start worker"
        } catch {
            Exit-BoundedStartFailure "$failure; worker process tree could not be terminated and ownership state was preserved: $($_.Exception.Message)"
        }
        if ($outputExceeded) {
            Limit-RemoteFixtureOutputFiles -Paths @($workerStdout, $workerStderr) -CombinedLimitBytes $WorkerOutputLimitBytes
        }
        $recovery = "recovered"
        try {
            $recovery = Invoke-BoundedStartRecovery -StateDirectory $stateDirectory
        } catch {
            Exit-BoundedStartFailure "$failure; $($_.Exception.Message)"
        }
        if ($recovery -eq "deferred") {
            Exit-BoundedStartFailure "$failure; fixture-state.json was already published, so cleanup is deferred to the fixture stop command"
        }
        Exit-BoundedStartFailure "$failure; owned processes, containers, and secret material were cleaned up"
    }

    $worker.WaitForExit()
    $postExitOutputBytes = Get-RemoteFixtureOutputBytes -Paths @($workerStdout, $workerStderr)
    if ($postExitOutputBytes -gt $WorkerOutputLimitBytes) {
        Limit-RemoteFixtureOutputFiles -Paths @($workerStdout, $workerStderr) -CombinedLimitBytes $WorkerOutputLimitBytes
        $failure = "fixture start worker exceeded the $WorkerOutputLimitBytes byte output limit at stage '$lastStage'"
        $recoveryFailure = $null
        $recovery = $null
        try {
            $recovery = Invoke-BoundedStartRecovery -StateDirectory $stateDirectory
        } catch {
            $recoveryFailure = $_.Exception.Message
        }
        if ($recoveryFailure) { Exit-BoundedStartFailure "$failure; $recoveryFailure" }
        if ($recovery -eq "deferred") {
            Exit-BoundedStartFailure "$failure; fixture-state.json was already published, so cleanup is deferred to the fixture stop command"
        }
        Exit-BoundedStartFailure "$failure; owned processes, containers, and secret material were cleaned up"
    }
    if ($worker.ExitCode -ne 0) {
        $secrets = Get-BoundedStartSecretValues -StateDirectory $stateDirectory
        $recoveryFailure = $null
        try {
            [void](Invoke-BoundedStartRecovery -StateDirectory $stateDirectory)
        } catch {
            $recoveryFailure = $_.Exception.Message
        }
        try {
            Write-BoundedStartRedactedTail -StderrPath $workerStderr -Secrets $secrets
        } catch {
            [Console]::Error.WriteLine("remote-browser-fixture-bounded-start: failure tail unavailable: $($_.Exception.Message)")
        }
        $failure = "fixture start worker failed with exit code $($worker.ExitCode) at stage '$lastStage'"
        if ($recoveryFailure) { Exit-BoundedStartFailure "$failure; $recoveryFailure" }
        Exit-BoundedStartFailure $failure
    }

    $stdoutItem = Get-Item -LiteralPath $workerStdout -ErrorAction SilentlyContinue
    if ($null -eq $stdoutItem -or $stdoutItem.Length -gt $StdoutLimitBytes) {
        Exit-BoundedStartFailure "fixture start worker success output is missing or exceeds $StdoutLimitBytes bytes"
    }
    $environmentLines = @(
        (Get-Content -LiteralPath $workerStdout) |
            ForEach-Object { $_.Trim() } |
            Where-Object { $_ }
    )
    $expectedEnvironmentFile = [IO.Path]::GetFullPath((Join-Path $stateDirectory "fixture-env.json"))
    if ($environmentLines.Count -ne 1 -or
        [IO.Path]::GetFullPath($environmentLines[0]) -ne $expectedEnvironmentFile -or
        -not (Test-Path -LiteralPath $environmentLines[0] -PathType Leaf)) {
        Exit-BoundedStartFailure "fixture start worker must emit exactly the fixture environment-file path"
    }
    Write-Output $environmentLines[0]
} finally {
    # Last-ditch containment: an unexpected wrapper-side exception must not
    # leave the worker, backend, or tunnel running with nobody responsible.
    $worker.Refresh()
    if (-not $worker.HasExited) {
        try {
            Stop-RemoteFixtureBoundedProcessTree -Process $worker -Label "fixture start worker"
        } catch {
            [Console]::Error.WriteLine("remote-browser-fixture-bounded-start: last-ditch worker termination failed: $($_.Exception.Message)")
        }
        try {
            [void](Invoke-BoundedStartRecovery -StateDirectory $stateDirectory)
        } catch {
            [Console]::Error.WriteLine("remote-browser-fixture-bounded-start: last-ditch recovery failed; ownership state was preserved: $($_.Exception.Message)")
        }
    }
}
