$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
$RootDirectory = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
. (Join-Path $PSScriptRoot "lib/remote-browser-fixture.ps1")

function Assert-Throws {
    param([Parameter(Mandatory)][scriptblock]$Action)
    try {
        & $Action
        throw "action unexpectedly succeeded"
    } catch {
        if ($_.Exception.Message -eq "action unexpectedly succeeded") { throw }
    }
}

if ($script:RemoteFixtureCloudflaredVersion -ne "2026.7.2") { throw "cloudflared version drift" }
if ($script:RemoteFixtureCloudflaredWindowsAmd64Sha256 -notmatch '^[0-9A-F]{64}$') { throw "invalid pinned SHA-256" }
Assert-RemoteFixtureHttpsOrigin "https://fixture.example.invalid"
Assert-RemoteFixtureHttpsOrigin "https://fixture.example.invalid:8443"
Assert-Throws { Assert-RemoteFixtureHttpsOrigin "http://fixture.example.invalid" }
Assert-Throws { Assert-RemoteFixtureHttpsOrigin "https://fixture.example.invalid/path" }
Assert-Throws { Assert-RemoteFixtureHttpsOrigin "https://user@fixture.example.invalid" }

$temporary = Join-Path ([IO.Path]::GetTempPath()) "deve-remote-fixture-test-$(New-RemoteFixtureRandomHex -Bytes 8)"
New-Item -ItemType Directory -Force $temporary | Out-Null
$process = $null
$secondaryProcess = $null
try {
    $argumentDirectory = Join-Path $temporary "argument space"
    New-Item -ItemType Directory -Force $argumentDirectory | Out-Null
    $childScript = Join-Path $argumentDirectory "child.ps1"
    Set-Content -LiteralPath $childScript -Encoding utf8 -Value 'param([string]$Value); Write-Output "$env:DEVE_FIXTURE_TEST_VALUE|$Value"'
    $argumentProcess = Start-RemoteFixtureProcess -FilePath (Get-Process -Id $PID).Path `
        -ArgumentList @("-NoProfile", "-File", $childScript, "argument with spaces") `
        -WorkingDirectory $temporary -Environment @{ DEVE_FIXTURE_TEST_VALUE = "environment value" } `
        -StdoutPath (Join-Path $temporary "argument.stdout.log") `
        -StderrPath (Join-Path $temporary "argument.stderr.log")
    $argumentProcess.WaitForExit()
    if ($argumentProcess.ExitCode -ne 0) { throw "quoted argument fixture process failed" }
    $argumentResult = (Get-Content -Raw -LiteralPath (Join-Path $temporary "argument.stdout.log")).Trim()
    if ($argumentResult -ne "environment value|argument with spaces") {
        throw "argument/environment propagation changed: $argumentResult"
    }

    $tunnelWriter = Join-Path $temporary "tunnel-writer.ps1"
    $tunnelStdout = Join-Path $temporary "tunnel.stdout.log"
    $tunnelStderr = Join-Path $temporary "tunnel.stderr.log"
    Set-Content -LiteralPath $tunnelWriter -Encoding utf8 -Value @'
Start-Sleep -Milliseconds 300
Write-Output "https://fixture-race.trycloudflare.com"
'@
    $process = Start-RemoteFixtureProcess -FilePath (Get-Process -Id $PID).Path `
        -ArgumentList @("-NoProfile", "-File", $tunnelWriter) `
        -WorkingDirectory $temporary -StdoutPath $tunnelStdout -StderrPath $tunnelStderr
    $origin = Wait-RemoteFixtureTunnelOrigin -Process $process -LogPaths @($tunnelStdout, $tunnelStderr)
    if ($origin -ne "https://fixture-race.trycloudflare.com") {
        throw "empty tunnel log race returned unexpected origin: $origin"
    }
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) { throw "tunnel log race fixture process failed" }
    $process = $null

    $drainScript = Join-Path $temporary "bounded-drain.ps1"
    Set-Content -LiteralPath $drainScript -Encoding utf8 -Value @'
[Console]::Out.Write(('o' * 32768))
[Console]::Error.Write(('e' * 32768))
'@
    $drainResult = Invoke-RemoteFixtureBoundedProcess -Label "parallel drain test" `
        -FilePath (Get-Process -Id $PID).Path `
        -ArgumentList @("-NoProfile", "-File", $drainScript) `
        -WorkingDirectory $temporary `
        -StdoutPath (Join-Path $temporary "bounded-drain.stdout") `
        -StderrPath (Join-Path $temporary "bounded-drain.stderr") `
        -TimeoutSeconds 5 -OutputLimitBytes 100000
    if ($drainResult.ExitCode -ne 0 -or $drainResult.OutputBytes -ne 65536) {
        throw "bounded process did not drain stdout/stderr concurrently"
    }

    $outputLimitScript = Join-Path $temporary "bounded-output-limit.ps1"
    Set-Content -LiteralPath $outputLimitScript -Encoding utf8 -Value @'
while ($true) {
    [Console]::Out.Write(('o' * 1024))
    [Console]::Out.Flush()
    [Console]::Error.Write(('e' * 1024))
    [Console]::Error.Flush()
    Start-Sleep -Milliseconds 10
}
'@
    Assert-Throws {
        Invoke-RemoteFixtureBoundedProcess -Label "output limit test" `
            -FilePath (Get-Process -Id $PID).Path `
            -ArgumentList @("-NoProfile", "-File", $outputLimitScript) `
            -WorkingDirectory $temporary `
            -StdoutPath (Join-Path $temporary "bounded-limit.stdout") `
            -StderrPath (Join-Path $temporary "bounded-limit.stderr") `
            -TimeoutSeconds 10 -OutputLimitBytes 4096 | Out-Null
    }
    $boundedLimitBytes = Get-RemoteFixtureOutputBytes -Paths @(
        (Join-Path $temporary "bounded-limit.stdout"),
        (Join-Path $temporary "bounded-limit.stderr")
    )
    if ($boundedLimitBytes -gt 4096) { throw "bounded process retained output beyond its cap" }

    $grandchildScript = Join-Path $temporary "bounded-grandchild.ps1"
    $timeoutParentScript = Join-Path $temporary "bounded-timeout-parent.ps1"
    $grandchildPidFile = Join-Path $temporary "bounded-grandchild.pid"
    Set-Content -LiteralPath $grandchildScript -Encoding utf8 -Value 'Start-Sleep -Seconds 60'
    Set-Content -LiteralPath $timeoutParentScript -Encoding utf8 -Value @'
param([string]$ChildScript, [string]$PidFile)
$child = Start-Process -FilePath (Get-Process -Id $PID).Path -ArgumentList @('-NoProfile', '-File', $ChildScript) -PassThru
Set-Content -LiteralPath $PidFile -Value $child.Id -NoNewline
Start-Sleep -Seconds 60
'@
    Assert-Throws {
        Invoke-RemoteFixtureBoundedProcess -Label "timeout tree test" `
            -FilePath (Get-Process -Id $PID).Path `
            -ArgumentList @("-NoProfile", "-File", $timeoutParentScript, $grandchildScript, $grandchildPidFile) `
            -WorkingDirectory $temporary `
            -StdoutPath (Join-Path $temporary "bounded-timeout.stdout") `
            -StderrPath (Join-Path $temporary "bounded-timeout.stderr") `
            -TimeoutSeconds 1 -OutputLimitBytes 4096 | Out-Null
    }
    if (-not (Test-Path -LiteralPath $grandchildPidFile)) { throw "timeout tree test did not start its grandchild" }
    $grandchildPid = [int](Get-Content -Raw -LiteralPath $grandchildPidFile)
    Start-Sleep -Milliseconds 200
    if (Get-Process -Id $grandchildPid -ErrorAction SilentlyContinue) {
        throw "timed-out bounded process left a grandchild alive"
    }

    $process = Start-RemoteFixtureProcess -FilePath (Get-Process -Id $PID).Path `
        -ArgumentList @("-NoProfile", "-Command", "Start-Sleep -Seconds 60") `
        -WorkingDirectory $temporary `
        -StdoutPath (Join-Path $temporary "child.stdout.log") `
        -StderrPath (Join-Path $temporary "child.stderr.log")
    $token = Get-RemoteFixtureProcessToken $process.Id
    Assert-Throws { Stop-RemoteFixtureProcess -Label "test" -ProcessId $process.Id -ExpectedToken "wrong-token" }
    if ($process.HasExited) { throw "mismatched token stopped an unowned process" }
    Stop-RemoteFixtureProcess -Label "test" -ProcessId $process.Id -ExpectedToken $token
    if (-not $process.HasExited) { throw "owned process survived cleanup" }
    $process = $null

    $tokens = $null
    $errors = $null
    [Management.Automation.Language.Parser]::ParseFile(
        (Join-Path $PSScriptRoot "remote-browser-fixture.ps1"),
        [ref]$tokens,
        [ref]$errors
    ) | Out-Null
    if ($errors.Count -ne 0) { throw "PowerShell wrapper contains parser errors: $($errors -join '; ')" }

    $source = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "remote-browser-fixture.ps1")
    if ($source -notmatch '--env-file \$dockerEnvFile') { throw "Docker backend must consume secrets through an env file" }
    if ($source -match '--env\s+"AUTH_(USER|PASS|SECRET)=') { throw "secret-bearing Docker argv regression" }
    if ($source -notmatch 'serve", "--port", "\{port\}", "--loopback-only') { throw "executable fixture must use loopback-only release serve" }
    if ($source -notmatch 'Invoke-RemoteFixtureBoundedProcess -Label "password hasher"') { throw "password hasher must use bounded process infra" }
    if ($source -notmatch 'Invoke-RemoteFixtureBoundedProcess -Label "exact-HEAD backend init"') { throw "backend init must use bounded process infra" }
    $librarySource = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "lib/remote-browser-fixture.ps1")
    if ($librarySource -notmatch 'Invoke-RemoteFixtureBoundedDownload -Url \$url') { throw "cloudflared download must use bounded streaming infra" }

    $failureState = Join-Path $temporary "failed-start"
    $headProof = Join-Path $temporary "head-proof.txt"
    $failHasher = Join-Path $temporary "fail-hasher.cmd"
    $head = (& git -C $RootDirectory rev-parse HEAD).Trim()
    Set-Content -LiteralPath $headProof -Value $head -NoNewline -Encoding utf8
    Set-Content -LiteralPath $failHasher -Value '@exit /b 1' -Encoding ascii
    Assert-Throws {
        & pwsh -NoProfile -File (Join-Path $PSScriptRoot "remote-browser-fixture.ps1") start `
            --state-dir $failureState `
            --expected-head $head `
            --backend-executable (Get-Process -Id $PID).Path `
            --backend-head-file $headProof `
            --password-hasher $failHasher 2>$null
        if ($LASTEXITCODE -ne 0) { throw "child failed as expected" }
        throw "action unexpectedly succeeded"
    }
    foreach ($leaked in @('.fixture-owner', 'fixture-state.json', 'fixture-env.json', 'credentials.json', '.password', '.backend.env', 'startup-state.json', 'startup-state.json.tmp')) {
        if (Test-Path -LiteralPath (Join-Path $failureState $leaked)) { throw "failed start leaked $leaked" }
    }

    $multiState = Join-Path $temporary "multi-stop"
    New-Item -ItemType Directory -Force $multiState | Out-Null
    $fixtureId = New-RemoteFixtureRandomHex -Bytes 16
    Set-Content -LiteralPath (Join-Path $multiState '.fixture-owner') -Value $fixtureId -NoNewline
    Set-Content -LiteralPath (Join-Path $multiState 'credentials.json') -Value '{}'
    Set-Content -LiteralPath (Join-Path $multiState 'fixture-env.json') -Value '{}'
    $process = Start-RemoteFixtureProcess -FilePath (Get-Process -Id $PID).Path `
        -ArgumentList @('-NoProfile', '-Command', 'Start-Sleep -Seconds 60') `
        -WorkingDirectory $multiState -StdoutPath (Join-Path $multiState 'backend.out') `
        -StderrPath (Join-Path $multiState 'backend.err')
    $secondaryProcess = Start-RemoteFixtureProcess -FilePath (Get-Process -Id $PID).Path `
        -ArgumentList @('-NoProfile', '-Command', 'Start-Sleep -Seconds 60') `
        -WorkingDirectory $multiState -StdoutPath (Join-Path $multiState 'tunnel.out') `
        -StderrPath (Join-Path $multiState 'tunnel.err')
    $state = [ordered]@{
        schema = 1; fixture_id = $fixtureId; expected_head = ('a' * 40); source_kind = 'test'
        https_origin = 'https://fixture.example.invalid'
        credentials_file = Join-Path $multiState 'credentials.json'
        environment_file = Join-Path $multiState 'fixture-env.json'
        backend_pid = $process.Id; backend_process_token = Get-RemoteFixtureProcessToken $process.Id
        tunnel_pid = $secondaryProcess.Id; tunnel_process_token = "wrong-$(Get-RemoteFixtureProcessToken $secondaryProcess.Id)"
        container_name = $null; created_at = [DateTimeOffset]::UtcNow.ToString('O')
    }
    $state | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $multiState 'fixture-state.json')
    Assert-Throws {
        & pwsh -NoProfile -File (Join-Path $PSScriptRoot 'remote-browser-fixture.ps1') stop --state-dir $multiState 2>$null
        if ($LASTEXITCODE -ne 0) { throw 'child failed as expected' }
        throw 'action unexpectedly succeeded'
    }
    $process.Refresh()
    $secondaryProcess.Refresh()
    if (-not $process.HasExited) { throw 'later owned backend cleanup was skipped' }
    $process = $null
    if ($secondaryProcess.HasExited) { throw 'mismatched first resource was unexpectedly stopped' }
    if (-not (Test-Path (Join-Path $multiState '.fixture-owner')) -or -not (Test-Path (Join-Path $multiState 'fixture-state.json'))) {
        throw 'failed multi-resource cleanup removed ownership state'
    }
    if ((Test-Path (Join-Path $multiState 'credentials.json')) -or (Test-Path (Join-Path $multiState 'fixture-env.json'))) {
        throw 'normal stop did not remove fixed secret files first'
    }
    $secondaryProcess.Kill($true)
    $secondaryProcess = $null

    $fakeBin = Join-Path $temporary 'fake-bin'
    New-Item -ItemType Directory -Force $fakeBin | Out-Null
    Set-Content -LiteralPath (Join-Path $fakeBin 'docker.cmd') -Encoding ascii -Value @'
@if "%FAKE_DOCKER_MODE%"=="present" (
  echo fixture
  exit /b 0
)
@if "%FAKE_DOCKER_MODE%"=="absent" exit /b 0
@exit /b 42
'@
    $priorPath = $env:PATH
    try {
        $env:PATH = "$fakeBin;$priorPath"
        $env:FAKE_DOCKER_MODE = 'absent'
        if (Test-RemoteFixtureContainerExists 'fixture') { throw 'absent container was reported present' }
        $env:FAKE_DOCKER_MODE = 'present'
        if (-not (Test-RemoteFixtureContainerExists 'fixture')) { throw 'present container was not detected' }
        $env:FAKE_DOCKER_MODE = 'error'
        Assert-Throws { [void](Test-RemoteFixtureContainerExists 'fixture') }
    } finally {
        Remove-Item Env:FAKE_DOCKER_MODE -ErrorAction SilentlyContinue
        $env:PATH = $priorPath
    }
} finally {
    if ($null -ne $process -and -not $process.HasExited) { $process.Kill($true) }
    if ($null -ne $secondaryProcess -and -not $secondaryProcess.HasExited) { $secondaryProcess.Kill($true) }
    Remove-Item -LiteralPath $temporary -Recurse -Force -ErrorAction SilentlyContinue
}

& pwsh -NoProfile -File (Join-Path $PSScriptRoot "remote-browser-fixture-bounded-start.test.ps1")
if ($LASTEXITCODE -ne 0) { throw "bounded-start wrapper regression failed" }

Write-Output "remote-browser-fixture.test: ok"
