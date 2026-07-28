Set-StrictMode -Version Latest

# Atomic startup state and sanitized stage progress for the RemoteBrowser
# fixture. Stage names are a fixed nonsecret allowlist; the startup state file
# carries only resource ownership facts (pids, process tokens, container name,
# owned file paths) and never credential values. Writes use a same-directory
# temporary file plus an atomic replace so a partially written state file can
# never be observed.

$script:RemoteFixtureStartupStageNames = @(
    "validate-head", "secure-state-directory", "generate-credentials",
    "hash-password", "initialize-backend", "start-backend",
    "wait-backend-health", "prepare-cloudflared", "start-tunnel",
    "wait-tunnel-origin", "wait-public-health", "publish-ready-state"
)
$script:RemoteFixtureStartupResourceFields = @(
    "source_kind", "backend_pid", "backend_process_token",
    "tunnel_pid", "tunnel_process_token", "container_name",
    "credentials_file", "environment_file"
)
$script:RemoteFixtureStartupStateFile = $null
$script:RemoteFixtureStartupState = $null

function Get-RemoteFixtureStartupStatePath {
    param([Parameter(Mandatory)][string]$StateDirectory)
    return Join-Path $StateDirectory "startup-state.json"
}

function Write-RemoteFixtureStageLine {
    param([Parameter(Mandatory)][string]$Stage)
    if ($script:RemoteFixtureStartupStageNames -notcontains $Stage) {
        throw "unknown fixture startup stage: $Stage"
    }
    [Console]::Error.WriteLine("deve-remote-fixture-stage: $Stage")
}

function Save-RemoteFixtureStartupState {
    $temporary = "$script:RemoteFixtureStartupStateFile.tmp"
    $script:RemoteFixtureStartupState | ConvertTo-Json | Set-Content -LiteralPath $temporary -Encoding utf8
    Move-Item -LiteralPath $temporary -Destination $script:RemoteFixtureStartupStateFile -Force
}

function Initialize-RemoteFixtureStartupState {
    param(
        [Parameter(Mandatory)][string]$StateDirectory,
        [Parameter(Mandatory)][string]$FixtureId
    )
    $script:RemoteFixtureStartupStateFile = Get-RemoteFixtureStartupStatePath $StateDirectory
    $script:RemoteFixtureStartupState = [ordered]@{
        schema = 1
        fixture_id = $FixtureId
        stage = "secure-state-directory"
        updated_at = [DateTimeOffset]::UtcNow.ToString("O")
        source_kind = $null
        backend_pid = $null
        backend_process_token = $null
        tunnel_pid = $null
        tunnel_process_token = $null
        container_name = $null
        credentials_file = $null
        environment_file = $null
    }
    Save-RemoteFixtureStartupState
    Write-RemoteFixtureStageLine "secure-state-directory"
}

function Update-RemoteFixtureStartupState {
    param(
        [Parameter(Mandatory)][string]$Stage,
        [hashtable]$Resources = @{}
    )
    if ($null -eq $script:RemoteFixtureStartupState) {
        throw "startup state has not been initialized"
    }
    if ($script:RemoteFixtureStartupStageNames -notcontains $Stage) {
        throw "unknown fixture startup stage: $Stage"
    }
    foreach ($entry in $Resources.GetEnumerator()) {
        if ($script:RemoteFixtureStartupResourceFields -notcontains $entry.Key) {
            throw "startup state field is not allowlisted: $($entry.Key)"
        }
        $script:RemoteFixtureStartupState[$entry.Key] = $entry.Value
    }
    $stageChanged = $script:RemoteFixtureStartupState.stage -ne $Stage
    $script:RemoteFixtureStartupState.stage = $Stage
    $script:RemoteFixtureStartupState.updated_at = [DateTimeOffset]::UtcNow.ToString("O")
    Save-RemoteFixtureStartupState
    if ($stageChanged) { Write-RemoteFixtureStageLine $Stage }
}

# Returns $null when no startup state exists. Throws on unreadable or
# schema-mismatched content so corrupted state is never consumed as valid.
function Read-RemoteFixtureStartupState {
    param([Parameter(Mandatory)][string]$StateDirectory)
    $path = Get-RemoteFixtureStartupStatePath $StateDirectory
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { return $null }
    $item = Get-Item -LiteralPath $path -Force
    if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "startup state must not be a reparse point: $path"
    }
    $state = Get-Content -Raw -LiteralPath $path | ConvertFrom-Json
    $required = @("schema", "fixture_id", "stage") + $script:RemoteFixtureStartupResourceFields
    foreach ($field in $required) {
        if (-not $state.PSObject.Properties[$field]) {
            throw "startup state is invalid and will not be consumed: $path"
        }
    }
    if ($state.schema -ne 1 -or
        $state.fixture_id -isnot [string] -or -not $state.fixture_id -or
        $state.stage -isnot [string] -or
        $script:RemoteFixtureStartupStageNames -notcontains $state.stage) {
        throw "startup state is invalid and will not be consumed: $path"
    }
    return $state
}

function Remove-RemoteFixtureStartupState {
    param([Parameter(Mandatory)][string]$StateDirectory)
    $path = Get-RemoteFixtureStartupStatePath $StateDirectory
    Remove-Item -LiteralPath $path, "$path.tmp" -Force -ErrorAction SilentlyContinue
}
