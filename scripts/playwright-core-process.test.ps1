$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "lib\playwright-core.ps1")

function Assert-ThrowsLike {
    param(
        [Parameter(Mandatory = $true)]
        [scriptblock]$Operation,
        [Parameter(Mandatory = $true)]
        [string]$Pattern
    )

    try {
        & $Operation
    } catch {
        if ($_.Exception.Message -notmatch $Pattern) {
            throw "expected error matching '$Pattern', got '$($_.Exception.Message)'"
        }
        return
    }
    throw "expected operation to fail with '$Pattern'"
}

$node = (Get-Command node.exe -ErrorAction Stop).Source
$npmCli = Resolve-DeveNpmCliPath -NodePath $node
if (-not (Test-Path -LiteralPath $npmCli -PathType Leaf)) {
    throw "resolved npm CLI path is not a file: $npmCli"
}

Invoke-DeveNodeProcessWithTimeout `
    -NodePath $node `
    -Arguments @("-e", "process.exit(0)") `
    -TimeoutSeconds 5 `
    -Operation "zero-exit probe"

$workRoot = Join-Path $env:TEMP ("deve playwright process " + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $workRoot -Force | Out-Null
try {
    $argvPath = Join-Path $workRoot "argv result.json"
    Invoke-DeveNodeProcessWithTimeout `
        -NodePath $node `
        -Arguments @(
            "-e",
            'require("fs").writeFileSync(process.argv[1], JSON.stringify(process.argv.slice(2)))',
            $argvPath,
            "space value",
            'quote"inside',
            "trailing\"
        ) `
        -TimeoutSeconds 5 `
        -Operation "argv probe"
    $observedArguments = Get-Content -Raw -LiteralPath $argvPath
    $expectedArguments = '["space value","quote\"inside","trailing\\"]'
    if ($observedArguments -ne $expectedArguments) {
        throw "Win32 argv encoding changed arguments: $observedArguments"
    }

    $childPidPath = Join-Path $workRoot "child pid.txt"
    Assert-ThrowsLike -Pattern "process-tree timeout probe exceeded 1s" -Operation {
        Invoke-DeveNodeProcessWithTimeout `
            -NodePath $node `
            -Arguments @(
                "-e",
                'const fs=require("fs"),{spawn}=require("child_process");const child=spawn(process.execPath,["-e","setInterval(()=>{},1000)"],{stdio:"ignore"});fs.writeFileSync(process.argv[1],String(child.pid));setInterval(()=>{},1000)',
                $childPidPath
            ) `
            -TimeoutSeconds 1 `
            -Operation "process-tree timeout probe"
    }
    $childPid = [int](Get-Content -Raw -LiteralPath $childPidPath)
    Start-Sleep -Milliseconds 250
    if ($null -ne (Get-Process -Id $childPid -ErrorAction SilentlyContinue)) {
        throw "timeout cleanup left child process $childPid running"
    }

    $packagePath = Join-Path $workRoot "package.json"
    [IO.File]::WriteAllText($packagePath, '{"name":"playwright-core","version":"0.0.0"}')
    Assert-ThrowsLike -Pattern "package version mismatch" -Operation {
        Assert-DevePlaywrightCorePackage -ModulePath $packagePath -ExpectedVersion "1.58.2"
    }
    [IO.File]::WriteAllText($packagePath, '{invalid')
    Assert-ThrowsLike -Pattern "package metadata is invalid JSON" -Operation {
        Assert-DevePlaywrightCorePackage -ModulePath $packagePath -ExpectedVersion "1.58.2"
    }
    Remove-Item -LiteralPath $packagePath -Force
    Assert-ThrowsLike -Pattern "package metadata is missing" -Operation {
        Assert-DevePlaywrightCorePackage -ModulePath $packagePath -ExpectedVersion "1.58.2"
    }

    $fakeNodeRoot = Join-Path $workRoot "missing node root"
    New-Item -ItemType Directory -Path $fakeNodeRoot | Out-Null
    $fakeNode = Join-Path $fakeNodeRoot "node.exe"
    Copy-Item -LiteralPath $node -Destination $fakeNode
    Assert-ThrowsLike -Pattern "npm CLI script is missing next to node.exe" -Operation {
        Resolve-DeveNpmCliPath -NodePath $fakeNode
    }
} finally {
    Remove-Item -LiteralPath $workRoot -Recurse -Force -ErrorAction SilentlyContinue
}

Assert-ThrowsLike -Pattern "nonzero-exit probe failed \(exit 7\)" -Operation {
    Invoke-DeveNodeProcessWithTimeout `
        -NodePath $node `
        -Arguments @("-e", "process.exit(7)") `
        -TimeoutSeconds 5 `
        -Operation "nonzero-exit probe"
}

Write-Host "playwright-core-process-test: ok"
