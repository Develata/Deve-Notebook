Set-StrictMode -Version Latest

# Atomic final/recovery state publication and pre-cleanup ownership admission
# for the Windows RemoteBrowser fixture.

function Write-RemoteFixtureJsonAtomic {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][object]$Value
    )
    $fullPath = [IO.Path]::GetFullPath($Path)
    $directory = [IO.Path]::GetDirectoryName($fullPath)
    $temporary = Join-Path $directory ".$([IO.Path]::GetFileName($fullPath)).$PID.$(New-RemoteFixtureRandomHex -Bytes 8).tmp"
    if (Test-Path -LiteralPath $fullPath) {
        $item = Get-Item -LiteralPath $fullPath -Force
        if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or $item.PSIsContainer) {
            throw "refusing to replace unsafe fixture JSON path: $fullPath"
        }
    }
    $stream = $null
    try {
        $bytes = [Text.UTF8Encoding]::new($false).GetBytes(($Value | ConvertTo-Json -Depth 8) + "`n")
        $stream = [IO.File]::Open(
            $temporary,
            [IO.FileMode]::CreateNew,
            [IO.FileAccess]::Write,
            [IO.FileShare]::None
        )
        $stream.Write($bytes, 0, $bytes.Length)
        $stream.Flush($true)
        $stream.Dispose()
        $stream = $null
        Protect-RemoteFixturePath $temporary
        [IO.File]::Move($temporary, $fullPath, $true)
        Protect-RemoteFixturePath $fullPath
    } finally {
        if ($stream) { $stream.Dispose() }
        Remove-Item -LiteralPath $temporary -Force -ErrorAction SilentlyContinue
    }
}

function Read-RemoteFixtureFinalState {
    param([Parameter(Mandatory)][string]$StateDirectory)
    $stateFile = Join-Path $StateDirectory "fixture-state.json"
    $ownerFile = Join-Path $StateDirectory ".fixture-owner"
    foreach ($path in @($stateFile, $ownerFile)) {
        $item = Get-Item -LiteralPath $path -Force -ErrorAction Stop
        if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or $item.PSIsContainer) {
            throw "unsafe fixture state path: $path"
        }
    }
    $state = Get-Content -Raw -LiteralPath $stateFile | ConvertFrom-Json
    foreach ($field in @(
        "schema", "state_kind", "fixture_id", "expected_head", "source_kind", "https_origin",
        "credentials_file", "environment_file", "backend_pid", "backend_process_token",
        "tunnel_pid", "tunnel_process_token", "container_name", "created_at"
    )) {
        if (-not $state.PSObject.Properties[$field]) {
            throw "fixture state is incomplete and will not be consumed: $stateFile"
        }
    }
    if ($state.schema -ne 1 -or
        $state.state_kind -isnot [string] -or $state.state_kind -notin @("ready", "recovery") -or
        $state.fixture_id -isnot [string] -or $state.fixture_id -notmatch '^[0-9a-f]{32}$' -or
        $state.expected_head -isnot [string] -or $state.expected_head -notmatch '^[0-9a-fA-F]{40}$' -or
        ($null -ne $state.source_kind -and
            ($state.source_kind -isnot [string] -or
                $state.source_kind -notin @("external", "executable", "container"))) -or
        ($null -ne $state.https_origin -and
            ($state.https_origin -isnot [string] -or -not $state.https_origin))) {
        throw "fixture state identity is invalid and will not be consumed: $stateFile"
    }
    if ($state.state_kind -eq "ready" -and
        ($null -eq $state.source_kind -or $null -eq $state.https_origin)) {
        throw "ready fixture state lacks source or HTTPS origin"
    }
    if ($null -ne $state.https_origin) {
        Assert-RemoteFixtureHttpsOrigin $state.https_origin
    }
    if ((Get-Content -Raw -LiteralPath $ownerFile).Trim() -ne $state.fixture_id) {
        throw "fixture owner marker does not match state"
    }
    $expectedCredentials = [IO.Path]::GetFullPath((Join-Path $StateDirectory "credentials.json"))
    $expectedEnvironment = [IO.Path]::GetFullPath((Join-Path $StateDirectory "fixture-env.json"))
    if ($state.credentials_file -isnot [string] -or
        $state.environment_file -isnot [string] -or
        [IO.Path]::GetFullPath($state.credentials_file) -ne $expectedCredentials -or
        [IO.Path]::GetFullPath($state.environment_file) -ne $expectedEnvironment) {
        throw "fixture state file ownership paths are invalid"
    }

    $backendPair = Assert-RemoteFixtureProcessPair `
        -State $state -PidField "backend_pid" -TokenField "backend_process_token"
    $tunnelPair = Assert-RemoteFixtureProcessPair `
        -State $state -PidField "tunnel_pid" -TokenField "tunnel_process_token"
    $containerPresent = $null -ne $state.container_name
    if ($containerPresent -and
        ($state.container_name -isnot [string] -or
            $state.container_name -notmatch '^deve-remote-fixture-[0-9a-f]{12}$')) {
        throw "fixture state container identity is invalid"
    }
    switch ($state.source_kind) {
        "external" {
            if ($backendPair -or $tunnelPair -or $containerPresent) {
                throw "external fixture state registered internal resources"
            }
        }
        "executable" {
            if ($containerPresent -or
                ($state.state_kind -eq "ready" -and (-not $backendPair -or -not $tunnelPair))) {
                throw "executable fixture state resource shape is invalid"
            }
        }
        "container" {
            if ($backendPair -or -not $containerPresent -or
                ($state.state_kind -eq "ready" -and -not $tunnelPair)) {
                throw "container fixture state resource shape is invalid"
            }
        }
        $null {
            if ($state.state_kind -ne "recovery" -or $backendPair -or $tunnelPair -or $containerPresent) {
                throw "unbound fixture state registered resources"
            }
        }
    }
    Assert-RemoteFixtureLiveProcessOwner `
        -Label "backend" -ProcessId $state.backend_pid -ExpectedToken $state.backend_process_token
    Assert-RemoteFixtureLiveProcessOwner `
        -Label "tunnel" -ProcessId $state.tunnel_pid -ExpectedToken $state.tunnel_process_token
    if ($containerPresent -and (Test-RemoteFixtureContainerExists $state.container_name)) {
        Assert-RemoteFixtureContainerOwner `
            -ContainerName $state.container_name -FixtureId $state.fixture_id
    }
    return $state
}

function Assert-RemoteFixtureProcessPair {
    param(
        [Parameter(Mandatory)][object]$State,
        [Parameter(Mandatory)][string]$PidField,
        [Parameter(Mandatory)][string]$TokenField
    )
    $processId = $State.$PidField
    $token = $State.$TokenField
    if ($null -eq $processId -and $null -eq $token) { return $false }
    if ($null -eq $processId -or $null -eq $token -or
        [string]$processId -notmatch '^[1-9][0-9]*$' -or
        [int64]$processId -gt [int]::MaxValue -or
        $token -isnot [string] -or $token -notmatch '^[1-9][0-9]*$') {
        throw "fixture state has an invalid $PidField/$TokenField ownership pair"
    }
    return $true
}

function Assert-RemoteFixtureStartupRecoveryAuthority {
    param([Parameter(Mandatory)][object]$State)
    if ($State.fixture_id -isnot [string] -or $State.fixture_id -notmatch '^[0-9a-f]{32}$') {
        throw "startup state fixture identity is invalid"
    }
    $backendPair = Assert-RemoteFixtureProcessPair `
        -State $State -PidField "backend_pid" -TokenField "backend_process_token"
    $tunnelPair = Assert-RemoteFixtureProcessPair `
        -State $State -PidField "tunnel_pid" -TokenField "tunnel_process_token"
    $containerPresent = $null -ne $State.container_name
    if ($containerPresent -and
        ($State.container_name -isnot [string] -or
            $State.container_name -notmatch '^deve-remote-fixture-[0-9a-f]{12}$')) {
        throw "startup state container identity is invalid"
    }
    if ($null -eq $State.source_kind) {
        if ($backendPair -or $tunnelPair -or $containerPresent) {
            throw "unbound startup state registered resources"
        }
    } elseif ($State.source_kind -eq "external") {
        if ($backendPair -or $tunnelPair -or $containerPresent) {
            throw "external startup state registered internal resources"
        }
    } elseif ($State.source_kind -eq "executable") {
        if ($containerPresent) {
            throw "executable startup state registered a container"
        }
    } elseif ($State.source_kind -eq "container") {
        if ($backendPair -or -not $containerPresent) {
            throw "container startup state resource shape is invalid"
        }
    } else {
        throw "startup state source kind is invalid"
    }
    Assert-RemoteFixtureLiveProcessOwner `
        -Label "backend" -ProcessId $State.backend_pid -ExpectedToken $State.backend_process_token
    Assert-RemoteFixtureLiveProcessOwner `
        -Label "tunnel" -ProcessId $State.tunnel_pid -ExpectedToken $State.tunnel_process_token
    if ($containerPresent -and (Test-RemoteFixtureContainerExists $State.container_name)) {
        Assert-RemoteFixtureContainerOwner `
            -ContainerName $State.container_name -FixtureId $State.fixture_id
    }
}

function Assert-RemoteFixtureLiveProcessOwner {
    param(
        [Parameter(Mandatory)][string]$Label,
        [AllowNull()][object]$ProcessId,
        [AllowNull()][string]$ExpectedToken
    )
    if ($null -eq $ProcessId) { return }
    $process = Get-Process -Id ([int]$ProcessId) -ErrorAction SilentlyContinue
    if ($null -eq $process) { return }
    $actual = $process.StartTime.ToUniversalTime().Ticks.ToString(
        [Globalization.CultureInfo]::InvariantCulture
    )
    if ($actual -ne $ExpectedToken) {
        throw "fixture state does not own live $Label PID $ProcessId"
    }
}
