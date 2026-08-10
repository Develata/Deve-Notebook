param(
    [Parameter(Position = 0, Mandatory = $true)]
    [ValidateSet("start", "stop", "run")]
    [string]$Command,

    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$RemainingArguments
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
$RootDirectory = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
. (Join-Path $PSScriptRoot "lib/remote-browser-fixture.ps1")
. (Join-Path $PSScriptRoot "lib/remote-browser-fixture-progress.ps1")

function Get-ParsedRemoteFixtureArguments {
    param([string[]]$Arguments)
    $result = @{
        StateDirectory = $null; ExpectedHead = $null; RepoRoot = $RootDirectory
        ExternalOrigin = $null; ExternalHeadProofUrl = $null; ExternalCredentialsFile = $null
        BackendExecutable = $null; BackendHeadFile = $null; BackendContainerImage = $null
        PasswordHasher = $null; PasswordHasherArguments = [Collections.Generic.List[string]]::new()
        BackendPort = $null; CloudflaredExecutable = $null
    }
    for ($index = 0; $index -lt $Arguments.Count; $index++) {
        $name = $Arguments[$index]
        if ($index + 1 -ge $Arguments.Count) { throw "missing value for $name" }
        $value = $Arguments[++$index]
        switch ($name) {
            "--state-dir" { $result.StateDirectory = $value }
            "--expected-head" { $result.ExpectedHead = $value }
            "--repo-root" { $result.RepoRoot = $value }
            "--external-origin" { $result.ExternalOrigin = $value }
            "--external-head-proof-url" { $result.ExternalHeadProofUrl = $value }
            "--external-credentials-file" { $result.ExternalCredentialsFile = $value }
            "--backend-executable" { $result.BackendExecutable = $value }
            "--backend-head-file" { $result.BackendHeadFile = $value }
            "--backend-container-image" { $result.BackendContainerImage = $value }
            "--password-hasher" { $result.PasswordHasher = $value }
            "--password-hasher-arg" { $result.PasswordHasherArguments.Add($value) }
            "--backend-port" { $result.BackendPort = [int]$value }
            "--cloudflared-executable" { $result.CloudflaredExecutable = $value }
            default { throw "unknown fixture argument: $name" }
        }
    }
    return $result
}

function Invoke-RemoteFixturePasswordHasher {
    param(
        [Parameter(Mandatory)][string]$Executable,
        [AllowEmptyCollection()][Parameter(Mandatory)][Collections.Generic.List[string]]$Arguments,
        [Parameter(Mandatory)][string]$PasswordFile
    )
    $workingDirectory = Split-Path -Parent $PasswordFile
    $stdoutPath = "$PasswordFile.hasher.stdout"
    $stderrPath = "$PasswordFile.hasher.stderr"
    $argumentsWithPassword = [Collections.Generic.List[string]]::new()
    foreach ($argument in $Arguments) { $argumentsWithPassword.Add($argument) }
    $argumentsWithPassword.Add("--password-file")
    $argumentsWithPassword.Add($PasswordFile)
    try {
        $result = Invoke-RemoteFixtureBoundedProcess -Label "password hasher" `
            -FilePath $Executable -ArgumentList $argumentsWithPassword.ToArray() `
            -WorkingDirectory $workingDirectory -StdoutPath $stdoutPath -StderrPath $stderrPath `
            -TimeoutSeconds 30 -OutputLimitBytes 65536
        if ($result.ExitCode -ne 0) { throw "password hasher failed with exit code $($result.ExitCode)" }
        $hash = (Get-Content -Raw -LiteralPath $stdoutPath).Trim()
        if ($hash -notmatch '^\$argon2id\$\S+$') { throw "password hasher did not emit one Argon2id PHC string" }
        return $hash
    } finally {
        Remove-Item -LiteralPath $stdoutPath, $stderrPath -Force -ErrorAction SilentlyContinue
    }
}

function Invoke-RemoteFixtureBackendInit {
    param(
        [Parameter(Mandatory)][string]$Executable,
        [Parameter(Mandatory)][string]$RuntimeDirectory,
        [Parameter(Mandatory)][hashtable]$Environment
    )
    $result = Invoke-RemoteFixtureBoundedProcess -Label "exact-HEAD backend init" `
        -FilePath $Executable `
        -ArgumentList @("init", "--repo", "default", "--projection-base", (Join-Path $RuntimeDirectory "notes"), "--path", $RuntimeDirectory) `
        -WorkingDirectory $RuntimeDirectory -Environment $Environment `
        -StdoutPath (Join-Path $RuntimeDirectory "backend-init.stdout.log") `
        -StderrPath (Join-Path $RuntimeDirectory "backend-init.stderr.log") `
        -TimeoutSeconds 60 -OutputLimitBytes 4194304
    if ($result.ExitCode -ne 0) { throw "exact-HEAD backend init failed with exit code $($result.ExitCode)" }
}

function Start-RemoteBrowserFixture {
    param([hashtable]$Options)
    if (-not $Options.StateDirectory -or -not $Options.ExpectedHead) {
        throw "start requires --state-dir and --expected-head"
    }
    Write-RemoteFixtureStageLine "validate-head"
    Assert-RemoteFixtureExpectedHead -RepoRoot $Options.RepoRoot -ExpectedHead $Options.ExpectedHead
    $stateDirectory = Resolve-RemoteFixtureStateDirectory $Options.StateDirectory
    Protect-RemoteFixturePath $stateDirectory
    $stateFile = Join-Path $stateDirectory "fixture-state.json"
    $environmentFile = Join-Path $stateDirectory "fixture-env.json"
    $credentialsFile = Join-Path $stateDirectory "credentials.json"
    $ownerFile = Join-Path $stateDirectory ".fixture-owner"
    if ((Test-Path -LiteralPath $stateFile) -or (Test-Path -LiteralPath $ownerFile)) {
        throw "fixture state already exists; stop or remove the prior fixture first"
    }

    $fixtureId = New-RemoteFixtureRandomHex -Bytes 16
    Set-Content -LiteralPath $ownerFile -Value $fixtureId -NoNewline -Encoding utf8
    Protect-RemoteFixturePath $ownerFile
    Initialize-RemoteFixtureStartupState -StateDirectory $stateDirectory -FixtureId $fixtureId
    Update-RemoteFixtureStartupState -Stage "secure-state-directory" -Resources @{
        credentials_file = $credentialsFile; environment_file = $environmentFile
    }
    $backendProcess = $null
    $tunnelProcess = $null
    $backendToken = $null
    $tunnelToken = $null
    $containerName = $null
    $passwordFile = Join-Path $stateDirectory ".password"
    $dockerEnvFile = Join-Path $stateDirectory ".backend.env"
    $sourceKind = $null
    $origin = $null
    $complete = $false
    $primaryFailure = $null

    try {
        if ($Options.ExternalOrigin -or $Options.ExternalHeadProofUrl -or $Options.ExternalCredentialsFile) {
            if (-not $Options.ExternalOrigin -or -not $Options.ExternalHeadProofUrl -or -not $Options.ExternalCredentialsFile) {
                throw "external override requires origin, same-origin HEAD proof URL, and credentials file"
            }
            if ($Options.BackendExecutable -or $Options.BackendContainerImage -or $Options.PasswordHasher) {
                throw "external override cannot be combined with an internal backend"
            }
            Assert-RemoteFixtureHttpsOrigin $Options.ExternalOrigin
            $originUri = [Uri]$Options.ExternalOrigin
            $proofUri = [Uri]$Options.ExternalHeadProofUrl
            if ($proofUri.Scheme -ne "https" -or $proofUri.GetLeftPart([UriPartial]::Authority) -ne $originUri.GetLeftPart([UriPartial]::Authority)) {
                throw "external HEAD proof URL must use the RemoteBrowser HTTPS origin"
            }
            $observedHead = (Invoke-WebRequest -Uri $proofUri -TimeoutSec 15 -MaximumRedirection 0 -UseBasicParsing).Content.Trim()
            if ($observedHead -ine $Options.ExpectedHead) {
                throw "external backend HEAD proof does not match expected HEAD"
            }
            $externalCredentials = Get-Item -LiteralPath $Options.ExternalCredentialsFile -Force
            if (($externalCredentials.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or $externalCredentials.PSIsContainer) {
                throw "external credentials must not be a reparse point"
            }
            $credentialValue = Get-Content -Raw -LiteralPath $externalCredentials.FullName | ConvertFrom-Json
            if (
                $credentialValue.username -isnot [string] -or -not $credentialValue.username -or
                $credentialValue.password -isnot [string] -or -not $credentialValue.password -or
                $credentialValue.username.IndexOfAny([char[]]"`0`r`n") -ge 0 -or
                $credentialValue.password.IndexOfAny([char[]]"`0`r`n") -ge 0
            ) {
                throw "external credentials JSON requires non-empty username and password"
            }
            [ordered]@{
                username = $credentialValue.username
                password = $credentialValue.password
                auth_secret = $null
            } | ConvertTo-Json | Set-Content -LiteralPath $credentialsFile -Encoding utf8
            Protect-RemoteFixturePath $credentialsFile
            Update-RemoteFixtureStartupState -Stage "wait-public-health" -Resources @{ source_kind = "external" }
            Invoke-WebRequest -Uri "$($Options.ExternalOrigin)/api/node/role" -TimeoutSec 10 -UseBasicParsing | Out-Null
            $sourceKind = "external"
            $origin = $Options.ExternalOrigin
        } else {
            if (-not $Options.PasswordHasher -or -not (Test-Path -LiteralPath $Options.PasswordHasher -PathType Leaf)) {
                throw "internal fixture requires --password-hasher"
            }
            if ([bool]$Options.BackendExecutable -eq [bool]$Options.BackendContainerImage) {
                throw "select exactly one internal backend source"
            }
            Update-RemoteFixtureStartupState -Stage "generate-credentials"
            $username = "deve-ci-$(New-RemoteFixtureRandomHex -Bytes 8)"
            $password = New-RemoteFixtureRandomHex -Bytes 24
            $authSecret = New-RemoteFixtureRandomHex -Bytes 48
            Set-Content -LiteralPath $passwordFile -Value $password -NoNewline -Encoding utf8
            Protect-RemoteFixturePath $passwordFile
            Update-RemoteFixtureStartupState -Stage "hash-password"
            $authPass = Invoke-RemoteFixturePasswordHasher -Executable $Options.PasswordHasher -Arguments $Options.PasswordHasherArguments -PasswordFile $passwordFile
            @{ username = $username; password = $password; auth_secret = $authSecret } |
                ConvertTo-Json | Set-Content -LiteralPath $credentialsFile -Encoding utf8
            Protect-RemoteFixturePath $credentialsFile
            Remove-Item -LiteralPath $passwordFile -Force

            $port = if ($Options.BackendPort) { [int]$Options.BackendPort } else { Get-RemoteFixtureFreePort }
            if ($port -lt 1024 -or $port -gt 65535) { throw "backend port must be in 1024..65535" }
            $runtimeDirectory = Join-Path $stateDirectory "runtime"
            New-Item -ItemType Directory -Force (Join-Path $runtimeDirectory "ledger"), (Join-Path $runtimeDirectory "notes") | Out-Null
            $backendEnvironment = @{
                AUTH_USER = $username; AUTH_PASS = $authPass; AUTH_SECRET = $authSecret
                DEVE_ENV = "production"; DEVE_LEDGER_DIR = (Join-Path $runtimeDirectory "ledger")
                DEVE_PLUGIN_DIR = (Join-Path $Options.RepoRoot "plugins")
            }

            if ($Options.BackendContainerImage) {
                $sourceKind = "container"
                $imageHead = (& docker image inspect --format '{{ index .Config.Labels "org.opencontainers.image.revision" }}' $Options.BackendContainerImage).Trim()
                if ($LASTEXITCODE -ne 0 -or $imageHead -ine $Options.ExpectedHead) {
                    throw "candidate image revision label does not match expected HEAD"
                }
                $containerName = "deve-remote-fixture-$($fixtureId.Substring(0, 12))"
                @("AUTH_USER=$username", "AUTH_PASS=$authPass", "AUTH_SECRET=$authSecret") |
                    Set-Content -LiteralPath $dockerEnvFile -Encoding utf8
                Protect-RemoteFixturePath $dockerEnvFile
                Update-RemoteFixtureStartupState -Stage "start-backend" -Resources @{
                    source_kind = "container"; container_name = $containerName
                }
                & docker run --detach --name $containerName `
                    --label "deve.remote-fixture-id=$fixtureId" `
                    --publish "127.0.0.1`:$port`:3001" `
                    --env-file $dockerEnvFile `
                    --volume "$(Join-Path $runtimeDirectory 'ledger'):/data/ledger" `
                    --volume "$(Join-Path $runtimeDirectory 'notes'):/notes" `
                    $Options.BackendContainerImage | Out-Null
                if ($LASTEXITCODE -ne 0) { throw "failed to start candidate backend container" }
                Remove-Item -LiteralPath $dockerEnvFile -Force
            } else {
                $sourceKind = "executable"
                $backend = Get-Item -LiteralPath $Options.BackendExecutable -Force
                $headProof = Get-Item -LiteralPath $Options.BackendHeadFile -Force
                if (($backend.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or
                    ($headProof.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or
                    (Get-Content -Raw -LiteralPath $headProof.FullName).Trim() -ine $Options.ExpectedHead) {
                    throw "backend executable or build HEAD proof is unsafe or mismatched"
                }
                Update-RemoteFixtureStartupState -Stage "initialize-backend" -Resources @{ source_kind = "executable" }
                Invoke-RemoteFixtureBackendInit -Executable $backend.FullName -RuntimeDirectory $runtimeDirectory -Environment $backendEnvironment
                $arguments = @("serve", "--port", "{port}", "--loopback-only")
                $arguments = @($arguments | ForEach-Object { $_.Replace("{port}", [string]$port).Replace("{data_dir}", $runtimeDirectory) })
                Update-RemoteFixtureStartupState -Stage "start-backend"
                $backendProcess = Start-RemoteFixtureProcess -FilePath $backend.FullName -ArgumentList $arguments `
                    -WorkingDirectory $runtimeDirectory -Environment $backendEnvironment `
                    -StdoutPath (Join-Path $stateDirectory "backend.stdout.log") `
                    -StderrPath (Join-Path $stateDirectory "backend.stderr.log")
                $backendToken = Get-RemoteFixtureProcessToken $backendProcess.Id
                Update-RemoteFixtureStartupState -Stage "start-backend" -Resources @{
                    backend_pid = $backendProcess.Id; backend_process_token = $backendToken
                }
            }

            $backendHealth = "http://127.0.0.1:$port/api/node/role"
            Update-RemoteFixtureStartupState -Stage "wait-backend-health"
            if ($containerName) {
                for ($attempt = 0; $attempt -lt 120; $attempt++) {
                    try { Invoke-WebRequest -Uri $backendHealth -TimeoutSec 2 -UseBasicParsing | Out-Null; break } catch {
                        & docker inspect $containerName *> $null
                        if ($LASTEXITCODE -ne 0) { throw "backend container exited before health check" }
                        Start-Sleep -Milliseconds 250
                    }
                }
                Invoke-WebRequest -Uri $backendHealth -TimeoutSec 2 -UseBasicParsing | Out-Null
            } else {
                Wait-RemoteFixtureHttp -Url $backendHealth -Process $backendProcess `
                    -LogPath (Join-Path $stateDirectory "backend.stderr.log") `
                    -TimeoutSeconds $script:RemoteFixtureBackendHealthTimeoutSeconds
            }

            Update-RemoteFixtureStartupState -Stage "prepare-cloudflared"
            $cloudflared = Install-RemoteFixtureCloudflared -StateDirectory $stateDirectory -SuppliedPath $Options.CloudflaredExecutable
            $tunnelStdout = Join-Path $stateDirectory "cloudflared.stdout.log"
            $tunnelStderr = Join-Path $stateDirectory "cloudflared.stderr.log"
            Update-RemoteFixtureStartupState -Stage "start-tunnel"
            $tunnelProcess = Start-RemoteFixtureProcess -FilePath $cloudflared `
                -ArgumentList @("tunnel", "--no-autoupdate", "--url", "http://127.0.0.1:$port") `
                -WorkingDirectory $stateDirectory -StdoutPath $tunnelStdout -StderrPath $tunnelStderr
            $tunnelToken = Get-RemoteFixtureProcessToken $tunnelProcess.Id
            Update-RemoteFixtureStartupState -Stage "start-tunnel" -Resources @{
                tunnel_pid = $tunnelProcess.Id; tunnel_process_token = $tunnelToken
            }
            Update-RemoteFixtureStartupState -Stage "wait-tunnel-origin"
            $origin = Wait-RemoteFixtureTunnelOrigin -Process $tunnelProcess -LogPaths @($tunnelStdout, $tunnelStderr)
            Update-RemoteFixtureStartupState -Stage "wait-public-health"
            Wait-RemoteFixtureHttp -Url "$origin/api/node/role" -Process $tunnelProcess `
                -LogPath $tunnelStderr -TimeoutSeconds $script:RemoteFixturePublicHealthTimeoutSeconds
        }

        Update-RemoteFixtureStartupState -Stage "publish-ready-state"
        $environment = [ordered]@{
            https_origin = $origin
            credentials_file = $credentialsFile
            state_file = $stateFile
        }
        $environment | ConvertTo-Json | Set-Content -LiteralPath $environmentFile -Encoding utf8
        Protect-RemoteFixturePath $environmentFile
        $state = [ordered]@{
            schema = 1; state_kind = "ready"
            fixture_id = $fixtureId; expected_head = $Options.ExpectedHead
            source_kind = $sourceKind; https_origin = $origin
            credentials_file = $credentialsFile; environment_file = $environmentFile
            backend_pid = if ($backendProcess) { $backendProcess.Id } else { $null }
            backend_process_token = $backendToken
            tunnel_pid = if ($tunnelProcess) { $tunnelProcess.Id } else { $null }
            tunnel_process_token = $tunnelToken
            container_name = $containerName; created_at = [DateTimeOffset]::UtcNow.ToString("O")
        }
        Write-RemoteFixtureJsonAtomic -Path $stateFile -Value $state
        $complete = $true
        return $environmentFile
    } catch {
        $primaryFailure = $_.Exception.Message
        throw
    } finally {
        if (-not $complete) {
            $cleanupErrors = [Collections.Generic.List[string]]::new()
            foreach ($secretName in $script:RemoteFixtureSecretFileNames) {
                try {
                    $secretPath = Join-Path $stateDirectory $secretName
                    if (Test-Path -LiteralPath $secretPath) {
                        Remove-Item -LiteralPath $secretPath -Force -ErrorAction Stop
                    }
                } catch { $cleanupErrors.Add($_.Exception.Message) }
            }
            if ($tunnelProcess) {
                try { Stop-RemoteFixtureProcess -Label "tunnel" -ProcessId $tunnelProcess.Id -ExpectedToken $tunnelToken } catch { $cleanupErrors.Add($_.Exception.Message) }
            }
            if ($backendProcess) {
                try { Stop-RemoteFixtureProcess -Label "backend" -ProcessId $backendProcess.Id -ExpectedToken $backendToken } catch { $cleanupErrors.Add($_.Exception.Message) }
            }
            if ($containerName) {
                try {
                    Remove-RemoteFixtureOwnedContainer -ContainerName $containerName -FixtureId $fixtureId
                } catch {
                    $cleanupErrors.Add($_.Exception.Message)
                }
            }
            if ($cleanupErrors.Count -gt 0) {
                $recoveryState = [ordered]@{
                    schema = 1; state_kind = "recovery"
                    fixture_id = $fixtureId; expected_head = $Options.ExpectedHead
                    source_kind = $sourceKind; https_origin = $origin
                    credentials_file = $credentialsFile; environment_file = $environmentFile
                    backend_pid = if ($backendProcess) { $backendProcess.Id } else { $null }
                    backend_process_token = $backendToken
                    tunnel_pid = if ($tunnelProcess) { $tunnelProcess.Id } else { $null }
                    tunnel_process_token = $tunnelToken
                    container_name = $containerName; created_at = [DateTimeOffset]::UtcNow.ToString("O")
                }
                try {
                    Write-RemoteFixtureJsonAtomic -Path $stateFile -Value $recoveryState
                } catch {
                    $cleanupErrors.Add("failed to preserve recovery state: $($_.Exception.Message)")
                }
                $failurePrefix = if ($primaryFailure) {
                    "fixture startup failed: $primaryFailure; "
                } else {
                    ""
                }
                throw "${failurePrefix}fixture startup cleanup failed; ownership state was preserved: $($cleanupErrors -join '; ')"
            }
            Remove-RemoteFixtureStartupState -StateDirectory $stateDirectory
            Remove-Item -LiteralPath $stateFile, $ownerFile -Force -ErrorAction SilentlyContinue
        }
    }
}

function Stop-RemoteBrowserFixture {
    param([Parameter(Mandatory)][string]$StateDirectory)
    $stateDirectory = Resolve-RemoteFixtureStateDirectory $StateDirectory
    $stateFile = Join-Path $stateDirectory "fixture-state.json"
    $ownerFile = Join-Path $stateDirectory ".fixture-owner"
    $state = Read-RemoteFixtureFinalState -StateDirectory $stateDirectory
    $cleanupErrors = [Collections.Generic.List[string]]::new()
    foreach ($secretName in $script:RemoteFixtureSecretFileNames) {
        try {
            $secretPath = Join-Path $stateDirectory $secretName
            if (Test-Path -LiteralPath $secretPath) {
                Remove-Item -LiteralPath $secretPath -Force -ErrorAction Stop
            }
        } catch { $cleanupErrors.Add($_.Exception.Message) }
    }
    foreach ($secretPath in @($state.credentials_file, $state.environment_file)) {
        try {
            if (Test-Path -LiteralPath $secretPath) {
                Remove-Item -LiteralPath $secretPath -Force -ErrorAction Stop
            }
        } catch { $cleanupErrors.Add($_.Exception.Message) }
    }
    try { Stop-RemoteFixtureProcess -Label "tunnel" -ProcessId $state.tunnel_pid -ExpectedToken $state.tunnel_process_token } catch { $cleanupErrors.Add($_.Exception.Message) }
    try { Stop-RemoteFixtureProcess -Label "backend" -ProcessId $state.backend_pid -ExpectedToken $state.backend_process_token } catch { $cleanupErrors.Add($_.Exception.Message) }
    if ($state.container_name) {
        try {
            Remove-RemoteFixtureOwnedContainer -ContainerName $state.container_name -FixtureId $state.fixture_id
        } catch {
            $cleanupErrors.Add($_.Exception.Message)
        }
    }
    foreach ($processId in @($state.backend_pid, $state.tunnel_pid)) {
        if ($null -ne $processId -and (Get-Process -Id ([int]$processId) -ErrorAction SilentlyContinue)) {
            $cleanupErrors.Add("owned fixture process survived cleanup: $processId")
        }
    }
    if ($cleanupErrors.Count -gt 0) {
        throw "fixture cleanup failed; ownership state was preserved: $($cleanupErrors -join '; ')"
    }
    Remove-RemoteFixtureStartupState -StateDirectory $stateDirectory
    Remove-Item -LiteralPath (Join-Path $stateDirectory ".bounded-start.stdout.log"), `
        (Join-Path $stateDirectory ".bounded-start.stderr.log") -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $stateFile, $ownerFile -Force
    Set-Content -LiteralPath (Join-Path $stateDirectory ".fixture-stopped") -Value "stopped" -NoNewline
}

switch ($Command) {
    "start" {
        $options = Get-ParsedRemoteFixtureArguments $RemainingArguments
        Start-RemoteBrowserFixture $options
    }
    "stop" {
        $options = Get-ParsedRemoteFixtureArguments $RemainingArguments
        if (-not $options.StateDirectory) { throw "stop requires --state-dir" }
        Stop-RemoteBrowserFixture $options.StateDirectory
    }
    "run" {
        $separator = [Array]::IndexOf($RemainingArguments, "--")
        if ($separator -lt 0 -or $separator -eq $RemainingArguments.Count - 1) { throw "run requires -- COMMAND" }
        $fixtureArguments = @($RemainingArguments[0..($separator - 1)])
        $childCommand = @($RemainingArguments[($separator + 1)..($RemainingArguments.Count - 1)])
        $options = Get-ParsedRemoteFixtureArguments $fixtureArguments
        Start-RemoteBrowserFixture $options | Out-Null
        $stateDirectory = Resolve-RemoteFixtureStateDirectory $options.StateDirectory
        $state = Get-Content -Raw -LiteralPath (Join-Path $stateDirectory "fixture-state.json") | ConvertFrom-Json
        $credentials = Get-Content -Raw -LiteralPath $state.credentials_file | ConvertFrom-Json
        $names = @("DEVE_REMOTE_FIXTURE_HTTPS_ORIGIN", "DEVE_REMOTE_FIXTURE_USERNAME", "DEVE_REMOTE_FIXTURE_PASSWORD", "DEVE_REMOTE_FIXTURE_AUTH_SECRET", "DEVE_REMOTE_FIXTURE_STATE_FILE")
        $previous = @{}
        foreach ($name in $names) { $previous[$name] = [Environment]::GetEnvironmentVariable($name, "Process") }
        try {
            $env:DEVE_REMOTE_FIXTURE_HTTPS_ORIGIN = $state.https_origin
            $env:DEVE_REMOTE_FIXTURE_USERNAME = $credentials.username
            $env:DEVE_REMOTE_FIXTURE_PASSWORD = $credentials.password
            $env:DEVE_REMOTE_FIXTURE_AUTH_SECRET = $credentials.auth_secret
            $env:DEVE_REMOTE_FIXTURE_STATE_FILE = Join-Path $stateDirectory "fixture-state.json"
            $childArguments = if ($childCommand.Count -gt 1) { @($childCommand[1..($childCommand.Count - 1)]) } else { @() }
            & $childCommand[0] @childArguments
            if ($LASTEXITCODE -ne 0) { throw "fixture child command failed with exit code $LASTEXITCODE" }
        } finally {
            foreach ($name in $names) { [Environment]::SetEnvironmentVariable($name, $previous[$name], "Process") }
            Stop-RemoteBrowserFixture $stateDirectory
        }
    }
}
