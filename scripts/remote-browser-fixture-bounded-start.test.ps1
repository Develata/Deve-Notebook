$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
$RootDirectory = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
. (Join-Path $PSScriptRoot "lib/remote-browser-fixture.ps1")
. (Join-Path $PSScriptRoot "lib/remote-browser-fixture-progress.ps1")
$Wrapper = Join-Path $PSScriptRoot "remote-browser-fixture-bounded-start.ps1"
$SecretSentinel = "bounded-start-secret-sentinel-9f3a71"

function Assert-Throws {
    param([Parameter(Mandatory)][scriptblock]$Action)
    try {
        & $Action
        throw "action unexpectedly succeeded"
    } catch {
        if ($_.Exception.Message -eq "action unexpectedly succeeded") { throw }
    }
}

function Wait-ProcessGone {
    param([Parameter(Mandatory)][int]$ProcessId)
    for ($attempt = 0; $attempt -lt 40; $attempt++) {
        if (-not (Get-Process -Id $ProcessId -ErrorAction SilentlyContinue)) { return $true }
        Start-Sleep -Milliseconds 250
    }
    return $false
}

function Invoke-Wrapper {
    param(
        [Parameter(Mandatory)][string]$CaseDirectory,
        [Parameter(Mandatory)][string[]]$Arguments,
        [hashtable]$Environment = @{}
    )
    # The wrapper strips inherited ACLs on the state directory, which also
    # re-stamps pre-existing children; harness capture files therefore live in
    # a sibling directory (production captures wrapper output via the console).
    $harnessDirectory = "$CaseDirectory-harness"
    New-Item -ItemType Directory -Force $harnessDirectory | Out-Null
    $stdoutPath = Join-Path $harnessDirectory "wrapper.stdout.log"
    $stderrPath = Join-Path $harnessDirectory "wrapper.stderr.log"
    if (-not $Environment.ContainsKey("DEVE_REMOTE_FIXTURE_TEST_WORKER")) {
        $Environment["DEVE_REMOTE_FIXTURE_TEST_WORKER"] = "1"
    }
    $process = Start-RemoteFixtureProcess -FilePath (Get-Process -Id $PID).Path `
        -ArgumentList (@("-NoProfile", "-File", $Wrapper) + $Arguments) `
        -WorkingDirectory $CaseDirectory -Environment $Environment `
        -StdoutPath $stdoutPath -StderrPath $stderrPath
    if (-not $process.WaitForExit(120000)) {
        $process.Kill($true)
        throw "bounded-start wrapper did not finish within its own test deadline"
    }
    $process.WaitForExit()
    $stdoutText = ""
    $stderrText = ""
    if (Test-Path -LiteralPath $stdoutPath) { $stdoutText = [IO.File]::ReadAllText($stdoutPath) }
    if (Test-Path -LiteralPath $stderrPath) { $stderrText = [IO.File]::ReadAllText($stderrPath) }
    return [pscustomobject]@{
        ExitCode = $process.ExitCode
        Stdout = $stdoutText
        Stderr = $stderrText
    }
}

function Write-MockStartupState {
    param(
        [Parameter(Mandatory)][string]$StateDirectory,
        [Parameter(Mandatory)][hashtable]$Overrides
    )
    $state = [ordered]@{
        schema = 1; fixture_id = "feedfacefeedfacefeedfacefeedface"; stage = "start-backend"
        updated_at = [DateTimeOffset]::UtcNow.ToString("O"); source_kind = "executable"
        backend_pid = $null; backend_process_token = $null
        tunnel_pid = $null; tunnel_process_token = $null; container_name = $null
        credentials_file = $null; environment_file = $null
    }
    foreach ($entry in $Overrides.GetEnumerator()) { $state[$entry.Key] = $entry.Value }
    $state | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $StateDirectory "startup-state.json") -Encoding utf8
}

$temporary = Join-Path ([IO.Path]::GetTempPath()) "deve-bounded-start-test-$(New-RemoteFixtureRandomHex -Bytes 8)"
New-Item -ItemType Directory -Force $temporary | Out-Null
$decoy = $null
try {
    # Progress library strictness: unknown stages and non-allowlisted state
    # fields are rejected, so the stage/state channel cannot carry secrets.
    Assert-Throws { Write-RemoteFixtureStageLine "not-a-stage" }
    $strictDirectory = Join-Path $temporary "strict"
    New-Item -ItemType Directory -Force $strictDirectory | Out-Null
    Initialize-RemoteFixtureStartupState -StateDirectory $strictDirectory -FixtureId "feedface"
    Assert-Throws { Update-RemoteFixtureStartupState -Stage "start-backend" -Resources @{ password = $SecretSentinel } }
    Update-RemoteFixtureStartupState -Stage "start-backend" -Resources @{ backend_pid = 4242 }
    if (Test-Path -LiteralPath (Join-Path $strictDirectory "startup-state.json.tmp")) {
        throw "atomic state write left a temporary file behind"
    }
    $strictState = Get-Content -Raw -LiteralPath (Join-Path $strictDirectory "startup-state.json") | ConvertFrom-Json
    $allowedFields = @("schema", "fixture_id", "stage", "updated_at") + $script:RemoteFixtureStartupResourceFields
    foreach ($property in $strictState.PSObject.Properties.Name) {
        if ($allowedFields -notcontains $property) { throw "startup state carries a non-allowlisted field: $property" }
    }
    if ((Read-RemoteFixtureStartupState -StateDirectory $strictDirectory).backend_pid -ne 4242) {
        throw "startup state round-trip lost the recorded resource"
    }

    # Normal startup: exactly one environment-file path on stdout, sanitized
    # stage lines on stderr.
    $normalDirectory = Join-Path $temporary "normal"
    New-Item -ItemType Directory -Force $normalDirectory | Out-Null
    $environmentFile = Join-Path $normalDirectory "fixture-env.json"
    Set-Content -LiteralPath $environmentFile -Value "{}" -Encoding utf8
    $normalWorker = Join-Path $temporary "normal-worker.ps1"
    Set-Content -LiteralPath $normalWorker -Encoding utf8 -Value @'
param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)
[Console]::Error.WriteLine("deve-remote-fixture-stage: validate-head")
[Console]::Error.WriteLine("deve-remote-fixture-stage: publish-ready-state")
[Console]::Error.WriteLine("deve-remote-fixture-stage: not-an-allowlisted-stage")
Write-Output $env:DEVE_TEST_ENVIRONMENT_FILE
exit 0
'@
    $result = Invoke-Wrapper -CaseDirectory $normalDirectory `
        -Arguments @("--state-dir", $normalDirectory, "--worker-script", $normalWorker, "--total-deadline-seconds", "60") `
        -Environment @{ DEVE_TEST_ENVIRONMENT_FILE = $environmentFile }
    if ($result.ExitCode -ne 0) { throw "normal startup failed: $($result.Stderr)" }
    $stdoutLines = @($result.Stdout -split "`r?`n" | Where-Object { $_ })
    if ($stdoutLines.Count -ne 1 -or $stdoutLines[0] -ne $environmentFile) {
        throw "wrapper success stream must be exactly one environment-file path"
    }
    if ($result.Stderr -notmatch 'deve-remote-fixture-stage: publish-ready-state') {
        throw "wrapper did not relay sanitized stage progress"
    }
    if ($result.Stderr -match 'not-an-allowlisted-stage') {
        throw "wrapper relayed a non-allowlisted stage line"
    }

    # Timeout: worker tree (child + grandchild) is terminated at the total
    # deadline, owned state and secrets are removed, and the final error names
    # the last stage.
    $hangDirectory = Join-Path $temporary "hang"
    New-Item -ItemType Directory -Force $hangDirectory | Out-Null
    $hangWorker = Join-Path $temporary "hang-worker.ps1"
    Set-Content -LiteralPath $hangWorker -Encoding utf8 -Value @'
param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)
$stateDirectory = $env:DEVE_TEST_STATE_DIR
[Console]::Error.WriteLine("deve-remote-fixture-stage: validate-head")
[Console]::Error.WriteLine("deve-remote-fixture-stage: start-backend")
Set-Content -LiteralPath (Join-Path $stateDirectory ".fixture-owner") -Value "feedfacefeedfacefeedfacefeedface" -NoNewline
@{ username = "deve-ci-test"; password = $env:DEVE_TEST_SECRET; auth_secret = $env:DEVE_TEST_SECRET } |
    ConvertTo-Json | Set-Content -LiteralPath (Join-Path $stateDirectory "credentials.json") -Encoding utf8
Set-Content -LiteralPath (Join-Path $stateDirectory ".password") -Value $env:DEVE_TEST_SECRET -NoNewline -Encoding utf8
$state = [ordered]@{
    schema = 1; fixture_id = "feedfacefeedfacefeedfacefeedface"; stage = "start-backend"
    updated_at = [DateTimeOffset]::UtcNow.ToString("O"); source_kind = "executable"
    backend_pid = $null; backend_process_token = $null
    tunnel_pid = $null; tunnel_process_token = $null; container_name = $null
    credentials_file = $null; environment_file = $null
}
$state | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $stateDirectory "startup-state.json") -Encoding utf8
$childScript = Join-Path $stateDirectory "child.ps1"
Set-Content -LiteralPath $childScript -Encoding utf8 -Value @"
`$grandchild = Start-Process -FilePath (Get-Process -Id `$PID).Path -ArgumentList @('-NoProfile', '-Command', 'Start-Sleep -Seconds 120') -PassThru -WindowStyle Hidden
Set-Content -LiteralPath '$stateDirectory\grandchild.pid' -Value `$grandchild.Id -NoNewline
Start-Sleep -Seconds 120
"@
$child = Start-Process -FilePath (Get-Process -Id $PID).Path -ArgumentList @('-NoProfile', '-File', $childScript) -PassThru -WindowStyle Hidden
Set-Content -LiteralPath (Join-Path $stateDirectory "child.pid") -Value $child.Id -NoNewline
Start-Sleep -Seconds 120
'@
    $stopwatch = [Diagnostics.Stopwatch]::StartNew()
    $result = Invoke-Wrapper -CaseDirectory $hangDirectory `
        -Arguments @("--state-dir", $hangDirectory, "--worker-script", $hangWorker, "--total-deadline-seconds", "3") `
        -Environment @{ DEVE_TEST_STATE_DIR = $hangDirectory; DEVE_TEST_SECRET = $SecretSentinel }
    $stopwatch.Stop()
    if ($result.ExitCode -eq 0) { throw "hanging worker unexpectedly succeeded" }
    if ($stopwatch.Elapsed.TotalSeconds -ge 60) { throw "timeout handling was not bounded" }
    if ($result.Stderr -notmatch "total deadline at stage 'start-backend'") {
        throw "timeout error did not name the last stage: $($result.Stderr)"
    }
    foreach ($pidFile in @("child.pid", "grandchild.pid")) {
        $path = Join-Path $hangDirectory $pidFile
        if (-not (Test-Path -LiteralPath $path)) { throw "hanging worker did not record $pidFile" }
        if (-not (Wait-ProcessGone -ProcessId ([int](Get-Content -Raw -LiteralPath $path)))) {
            throw "worker descendant survived the total deadline: $pidFile"
        }
    }
    foreach ($cleaned in @("credentials.json", ".password", "startup-state.json", ".fixture-owner")) {
        if (Test-Path -LiteralPath (Join-Path $hangDirectory $cleaned)) {
            throw "timeout cleanup left owned state or secret material behind: $cleaned"
        }
    }
    if (($result.Stdout + $result.Stderr).Contains($SecretSentinel)) {
        throw "timeout output exposed an injected credential value"
    }

    # Cleanup failure: a recorded PID with a mismatched process token is never
    # killed, the recovery state is preserved, and the wrapper fails closed.
    $decoy = Start-Process -FilePath (Get-Process -Id $PID).Path `
        -ArgumentList @('-NoProfile', '-Command', 'Start-Sleep -Seconds 120') -PassThru -WindowStyle Hidden
    $mismatchDirectory = Join-Path $temporary "mismatch"
    New-Item -ItemType Directory -Force $mismatchDirectory | Out-Null
    $mismatchWorker = Join-Path $temporary "mismatch-worker.ps1"
    Set-Content -LiteralPath $mismatchWorker -Encoding utf8 -Value @'
param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)
[Console]::Error.WriteLine("deve-remote-fixture-stage: start-backend")
$state = [ordered]@{
    schema = 1; fixture_id = "feedfacefeedfacefeedfacefeedface"; stage = "start-backend"
    updated_at = [DateTimeOffset]::UtcNow.ToString("O"); source_kind = "executable"
    backend_pid = [int]$env:DEVE_TEST_DECOY_PID; backend_process_token = "mismatched-token"
    tunnel_pid = $null; tunnel_process_token = $null; container_name = $null
    credentials_file = $null; environment_file = $null
}
$state | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $env:DEVE_TEST_STATE_DIR "startup-state.json") -Encoding utf8
Start-Sleep -Seconds 120
'@
    $result = Invoke-Wrapper -CaseDirectory $mismatchDirectory `
        -Arguments @("--state-dir", $mismatchDirectory, "--worker-script", $mismatchWorker, "--total-deadline-seconds", "3") `
        -Environment @{ DEVE_TEST_STATE_DIR = $mismatchDirectory; DEVE_TEST_DECOY_PID = [string]$decoy.Id }
    if ($result.ExitCode -eq 0) { throw "mismatched-token cleanup unexpectedly succeeded" }
    if ($result.Stderr -notmatch "ownership state was preserved") {
        throw "cleanup failure did not report preserved ownership state"
    }
    $decoy.Refresh()
    if ($decoy.HasExited) { throw "a mismatched process token was killed during recovery" }
    if (-not (Test-Path -LiteralPath (Join-Path $mismatchDirectory "startup-state.json"))) {
        throw "failed recovery removed the preserved startup state"
    }
    $decoy.Kill($true)
    $decoy = $null

    # Container owner mismatch: an unowned container is never removed.
    $containerDirectory = Join-Path $temporary "container"
    New-Item -ItemType Directory -Force $containerDirectory | Out-Null
    $fakeBin = Join-Path $temporary "fake-bin"
    New-Item -ItemType Directory -Force $fakeBin | Out-Null
    $dockerLog = Join-Path $containerDirectory "docker.log"
    Set-Content -LiteralPath (Join-Path $fakeBin "docker.cmd") -Encoding ascii -Value @'
@echo %* >> "%FAKE_DOCKER_LOG%"
@if "%1"=="ps" ( echo deve-remote-fixture-decoy & exit /b 0 )
@if "%1"=="inspect" ( echo other-owner & exit /b 0 )
@exit /b 0
'@
    $containerWorker = Join-Path $temporary "container-worker.ps1"
    Set-Content -LiteralPath $containerWorker -Encoding utf8 -Value @'
param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)
[Console]::Error.WriteLine("deve-remote-fixture-stage: start-backend")
$state = [ordered]@{
    schema = 1; fixture_id = "feedfacefeedfacefeedfacefeedface"; stage = "start-backend"
    updated_at = [DateTimeOffset]::UtcNow.ToString("O"); source_kind = "container"
    backend_pid = $null; backend_process_token = $null
    tunnel_pid = $null; tunnel_process_token = $null; container_name = "deve-remote-fixture-decoy"
    credentials_file = $null; environment_file = $null
}
$state | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $env:DEVE_TEST_STATE_DIR "startup-state.json") -Encoding utf8
Start-Sleep -Seconds 120
'@
    $result = Invoke-Wrapper -CaseDirectory $containerDirectory `
        -Arguments @("--state-dir", $containerDirectory, "--worker-script", $containerWorker, "--total-deadline-seconds", "3") `
        -Environment @{
            DEVE_TEST_STATE_DIR = $containerDirectory; FAKE_DOCKER_LOG = $dockerLog
            PATH = "$fakeBin;$env:PATH"
        }
    if ($result.ExitCode -eq 0) { throw "unowned-container cleanup unexpectedly succeeded" }
    if ((Test-Path -LiteralPath $dockerLog) -and ((Get-Content -Raw -LiteralPath $dockerLog) -match '(^|\s)rm(\s|$)')) {
        throw "cleanup removed a container without a matching owner label"
    }
    if (-not (Test-Path -LiteralPath (Join-Path $containerDirectory "startup-state.json"))) {
        throw "failed container recovery removed the preserved startup state"
    }

    # Partial or corrupted startup state is never consumed as valid state.
    $partialDirectory = Join-Path $temporary "partial"
    New-Item -ItemType Directory -Force $partialDirectory | Out-Null
    $partialWorker = Join-Path $temporary "partial-worker.ps1"
    Set-Content -LiteralPath $partialWorker -Encoding utf8 -Value @'
param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)
[Console]::Error.WriteLine("deve-remote-fixture-stage: start-backend")
Set-Content -LiteralPath (Join-Path $env:DEVE_TEST_STATE_DIR "startup-state.json") -Value '{"schema":1,"fixture_id":"x","truncated' -Encoding utf8
Start-Sleep -Seconds 120
'@
    $result = Invoke-Wrapper -CaseDirectory $partialDirectory `
        -Arguments @("--state-dir", $partialDirectory, "--worker-script", $partialWorker, "--total-deadline-seconds", "3") `
        -Environment @{ DEVE_TEST_STATE_DIR = $partialDirectory }
    if ($result.ExitCode -eq 0) { throw "corrupted startup state was consumed as valid" }
    if ($result.Stderr -notmatch "ownership state was preserved") {
        throw "corrupted-state recovery did not fail closed"
    }
    if (-not (Test-Path -LiteralPath (Join-Path $partialDirectory "startup-state.json"))) {
        throw "corrupted startup state evidence was deleted"
    }

    # Failure output stays bounded and redacts injected credential values.
    $leakDirectory = Join-Path $temporary "leak"
    New-Item -ItemType Directory -Force $leakDirectory | Out-Null
    $leakWorker = Join-Path $temporary "leak-worker.ps1"
    Set-Content -LiteralPath $leakWorker -Encoding utf8 -Value @'
param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)
@{ username = "deve-ci-test"; password = $env:DEVE_TEST_SECRET; auth_secret = $env:DEVE_TEST_SECRET } |
    ConvertTo-Json | Set-Content -LiteralPath (Join-Path $env:DEVE_TEST_STATE_DIR "credentials.json") -Encoding utf8
[Console]::Error.WriteLine("worker failure detail includes $env:DEVE_TEST_SECRET")
exit 3
'@
    $result = Invoke-Wrapper -CaseDirectory $leakDirectory `
        -Arguments @("--state-dir", $leakDirectory, "--worker-script", $leakWorker, "--total-deadline-seconds", "60") `
        -Environment @{ DEVE_TEST_STATE_DIR = $leakDirectory; DEVE_TEST_SECRET = $SecretSentinel }
    if ($result.ExitCode -eq 0) { throw "failing worker unexpectedly succeeded" }
    if ($result.Stderr -notmatch "exit code 3") { throw "worker failure exit code was not reported" }
    if (($result.Stdout + $result.Stderr).Contains($SecretSentinel)) {
        throw "failure output exposed an injected credential value"
    }
    if ($result.Stderr -notmatch '\[redacted\]') { throw "failure tail was not redacted" }

    # Oversized success output is rejected without echoing it.
    $noisyDirectory = Join-Path $temporary "noisy"
    New-Item -ItemType Directory -Force $noisyDirectory | Out-Null
    $noisyWorker = Join-Path $temporary "noisy-worker.ps1"
    Set-Content -LiteralPath $noisyWorker -Encoding utf8 -Value @'
param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)
[Console]::Out.Write(('o' * 1048576))
exit 0
'@
    $result = Invoke-Wrapper -CaseDirectory $noisyDirectory `
        -Arguments @("--state-dir", $noisyDirectory, "--worker-script", $noisyWorker, "--total-deadline-seconds", "60")
    if ($result.ExitCode -eq 0) { throw "oversized success output was accepted" }
    if (($result.Stdout.Length + $result.Stderr.Length) -gt 16384) {
        throw "wrapper output was not bounded on oversized worker output"
    }

    # A worker that fails after deleting its own credential files (the real
    # worker's finally semantics) still gets a stage-named exit-code report.
    $cleanFailDirectory = Join-Path $temporary "clean-fail"
    New-Item -ItemType Directory -Force $cleanFailDirectory | Out-Null
    $cleanFailWorker = Join-Path $temporary "clean-fail-worker.ps1"
    Set-Content -LiteralPath $cleanFailWorker -Encoding utf8 -Value @'
param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)
[Console]::Error.WriteLine("deve-remote-fixture-stage: validate-head")
[Console]::Error.WriteLine("worker failure detail without any credential files on disk")
exit 4
'@
    $result = Invoke-Wrapper -CaseDirectory $cleanFailDirectory `
        -Arguments @("--state-dir", $cleanFailDirectory, "--worker-script", $cleanFailWorker, "--total-deadline-seconds", "60")
    if ($result.ExitCode -eq 0) { throw "failing worker without credential files unexpectedly succeeded" }
    if ($result.Stderr -notmatch "failed with exit code 4 at stage 'validate-head'") {
        throw "zero-secret failure lost its stage-named exit-code report: $($result.Stderr)"
    }

    # Multi-line success output is rejected without echoing the extra line.
    $multiDirectory = Join-Path $temporary "multi"
    New-Item -ItemType Directory -Force $multiDirectory | Out-Null
    Set-Content -LiteralPath (Join-Path $multiDirectory "fixture-env.json") -Value "{}" -Encoding utf8
    $multiWorker = Join-Path $temporary "multi-worker.ps1"
    Set-Content -LiteralPath $multiWorker -Encoding utf8 -Value @'
param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)
Write-Output $env:DEVE_TEST_SECRET
Write-Output (Join-Path $env:DEVE_TEST_STATE_DIR "fixture-env.json")
exit 0
'@
    $result = Invoke-Wrapper -CaseDirectory $multiDirectory `
        -Arguments @("--state-dir", $multiDirectory, "--worker-script", $multiWorker, "--total-deadline-seconds", "60") `
        -Environment @{ DEVE_TEST_STATE_DIR = $multiDirectory; DEVE_TEST_SECRET = $SecretSentinel }
    if ($result.ExitCode -eq 0) { throw "multi-line success output was accepted" }
    if (($result.Stdout + $result.Stderr).Contains($SecretSentinel)) {
        throw "multi-line success output echoed the extra line"
    }

    # Integration: the real fixture worker (no --worker-script override) fails
    # at hash-password and the wrapper relays its real stage instrumentation.
    $realDirectory = Join-Path $temporary "real"
    New-Item -ItemType Directory -Force $realDirectory | Out-Null
    $realHead = (& git -C $RootDirectory rev-parse HEAD).Trim()
    $realHeadProof = Join-Path $temporary "real-head-proof.txt"
    Set-Content -LiteralPath $realHeadProof -Value $realHead -NoNewline -Encoding utf8
    $realFailHasher = Join-Path $temporary "real-fail-hasher.cmd"
    Set-Content -LiteralPath $realFailHasher -Value '@exit /b 1' -Encoding ascii
    $result = Invoke-Wrapper -CaseDirectory $realDirectory -Arguments @(
        "--state-dir", $realDirectory,
        "--expected-head", $realHead,
        "--backend-executable", (Get-Process -Id $PID).Path,
        "--backend-head-file", $realHeadProof,
        "--password-hasher", $realFailHasher,
        "--total-deadline-seconds", "120"
    )
    if ($result.ExitCode -eq 0) { throw "real worker with a failing hasher unexpectedly succeeded" }
    if ($result.Stderr -notmatch 'deve-remote-fixture-stage: hash-password') {
        throw "real worker stage lines were not relayed: $($result.Stderr)"
    }
    if ($result.Stderr -notmatch "at stage 'hash-password'") {
        throw "real worker failure did not name its last stage: $($result.Stderr)"
    }
    foreach ($leaked in @('.fixture-owner', 'fixture-state.json', 'fixture-env.json', 'credentials.json', '.password', 'startup-state.json')) {
        if (Test-Path -LiteralPath (Join-Path $realDirectory $leaked)) {
            throw "real worker failure through the wrapper leaked $leaked"
        }
    }
} finally {
    if ($null -ne $decoy -and -not $decoy.HasExited) { $decoy.Kill($true) }
    Remove-Item -LiteralPath $temporary -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Output "remote-browser-fixture-bounded-start.test: ok"
