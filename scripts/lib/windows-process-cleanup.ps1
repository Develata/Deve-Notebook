# Bounded Windows process-tree cleanup shared by target-host browser journeys.

function Get-DeveProcessTreeSnapshot {
    param(
        [Parameter(Mandatory = $true)]
        [int]$RootProcessId
    )

    $processes = @(Get-CimInstance Win32_Process -OperationTimeoutSec 5)
    $byId = @{}
    foreach ($process in $processes) {
        $byId[[int]$process.ProcessId] = $process
    }
    $root = $byId[$RootProcessId]
    if ($null -eq $root -or $null -eq $root.CreationDate) {
        throw "root process identity is unavailable from CIM snapshot"
    }

    $owned = @{}
    $owned[$RootProcessId] = ([DateTime]$root.CreationDate).ToUniversalTime()
    $changed = $true
    while ($changed) {
        $changed = $false
        foreach ($process in $processes) {
            $processId = [int]$process.ProcessId
            $parentProcessId = [int]$process.ParentProcessId
            if (
                $owned.ContainsKey($processId) -or
                -not $owned.ContainsKey($parentProcessId) -or
                $null -eq $process.CreationDate
            ) {
                continue
            }
            $createdAtUtc = ([DateTime]$process.CreationDate).ToUniversalTime()
            if (
                $createdAtUtc -ge $owned[$parentProcessId]
            ) {
                $owned[$processId] = $createdAtUtc
                $changed = $true
            }
        }
    }

    @(
        foreach ($processId in $owned.Keys) {
            $liveProcess = Get-Process -Id $processId -ErrorAction SilentlyContinue
            if ($null -eq $liveProcess) {
                continue
            }
            [pscustomobject]@{
                ProcessId = [int]$processId
                StartedAtUtc = $liveProcess.StartTime.ToUniversalTime()
            }
        }
    )
}

function Test-DeveProcessIdentityAlive {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Snapshot
    )

    $liveProcess = Get-Process -Id ([int]$Snapshot.ProcessId) -ErrorAction SilentlyContinue
    if ($null -eq $liveProcess) {
        return $false
    }
    try {
        return $liveProcess.StartTime.ToUniversalTime().Ticks -eq
            ([DateTime]$Snapshot.StartedAtUtc).ToUniversalTime().Ticks
    } catch {
        return $true
    }
}

function Invoke-DeveBoundedTaskkill {
    param(
        [Parameter(Mandatory = $true)]
        [int]$RootProcessId,
        [int]$TimeoutSeconds = 10
    )

    $taskkillPath = Join-Path $env:SystemRoot "System32\taskkill.exe"
    if (-not (Test-Path -LiteralPath $taskkillPath -PathType Leaf)) {
        throw "taskkill executable is unavailable"
    }

    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $taskkillPath
    $psi.Arguments = "/PID $RootProcessId /T /F"
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $taskkill = [System.Diagnostics.Process]::Start($psi)
    $stdoutTask = $taskkill.StandardOutput.ReadToEndAsync()
    $stderrTask = $taskkill.StandardError.ReadToEndAsync()
    try {
        if (-not $taskkill.WaitForExit($TimeoutSeconds * 1000)) {
            $taskkill.Kill()
            [void]$taskkill.WaitForExit(5000)
            throw "taskkill exceeded its ${TimeoutSeconds}s operation timeout"
        }
        $taskkill.WaitForExit()
        [void]$stdoutTask.GetAwaiter().GetResult()
        [void]$stderrTask.GetAwaiter().GetResult()
        return $taskkill.ExitCode
    } finally {
        $taskkill.Dispose()
    }
}

function Invoke-DeveNodeJourney {
    param(
        [Parameter(Mandatory = $true)]
        [string]$NodePath,
        [Parameter(Mandatory = $true)]
        [string]$ScriptPath,
        [Parameter(Mandatory = $true)]
        [int]$TimeoutSeconds,
        [string]$Label = "Node journey"
    )

    $node = [System.IO.Path]::GetFullPath($NodePath)
    $script = [System.IO.Path]::GetFullPath($ScriptPath)
    if (
        -not (Test-Path -LiteralPath $node -PathType Leaf) -or
        -not (Test-Path -LiteralPath $script -PathType Leaf) -or
        $script.Contains('"')
    ) {
        throw "webview2-cdp: $Label executable or script path is invalid"
    }

    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $node
    $psi.Arguments = "`"$script`""
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $process = [System.Diagnostics.Process]::Start($psi)
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()

    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        $cleanupFailures = @()
        $ownedProcesses = @(
            [pscustomobject]@{
                ProcessId = $process.Id
                StartedAtUtc = $process.StartTime.ToUniversalTime()
            }
        )
        try {
            $ownedProcesses = @(Get-DeveProcessTreeSnapshot -RootProcessId $process.Id)
        } catch {
            $cleanupFailures += "snapshot=$($_.Exception.Message)"
        }
        $taskkillExitCode = $null
        try {
            $taskkillExitCode = Invoke-DeveBoundedTaskkill -RootProcessId $process.Id
        } catch {
            $cleanupFailures += "taskkill=$($_.Exception.Message)"
        }
        try {
            $process.Refresh()
            if (-not $process.HasExited) {
                Stop-Process -InputObject $process -Force -ErrorAction Stop
            }
            if (-not $process.WaitForExit(10000)) {
                $cleanupFailures += "root process remained alive after direct-child fallback"
            }
        } catch {
            $cleanupFailures += "root=$($_.Exception.Message)"
        }
        $treeDeadline = [DateTime]::UtcNow.AddSeconds(10)
        do {
            $remaining = @(
                $ownedProcesses |
                    Where-Object {
                        Test-DeveProcessIdentityAlive -Snapshot $_
                    }
            )
            if ($remaining.Count -eq 0) {
                break
            }
            Start-Sleep -Milliseconds 200
        } while ([DateTime]::UtcNow -lt $treeDeadline)
        if ($null -ne $taskkillExitCode -and $taskkillExitCode -ne 0) {
            $cleanupFailures += "taskkill=$taskkillExitCode"
        }
        if ($remaining.Count -ne 0) {
            $cleanupFailures += "residual=$($remaining.ProcessId -join ',')"
        }
        if ($cleanupFailures.Count -ne 0) {
            throw "webview2-cdp: $Label timed out and process-tree cleanup failed; $($cleanupFailures -join '; ')"
        }
        throw "webview2-cdp: $Label timed out after $TimeoutSeconds seconds"
    }
    $process.WaitForExit()
    $stdout = $stdoutTask.GetAwaiter().GetResult()
    $stderr = $stderrTask.GetAwaiter().GetResult()
    if (-not [string]::IsNullOrWhiteSpace($stdout)) {
        Write-Host $stdout.TrimEnd()
    }
    if ($process.ExitCode -ne 0) {
        $detail = if ([string]::IsNullOrWhiteSpace($stderr)) {
            "no stderr"
        } else {
            $stderr.Trim().Substring(0, [Math]::Min(4096, $stderr.Trim().Length))
        }
        throw "webview2-cdp: $Label failed with exit code $($process.ExitCode): $detail"
    }
    if (-not [string]::IsNullOrWhiteSpace($stderr)) {
        Write-Warning "$Label stderr: $($stderr.Trim())"
    }
}
