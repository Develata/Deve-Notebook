function Resolve-DeveNpmCliPath {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [string]$NodePath
    )

    $resolvedNode = (Resolve-Path -LiteralPath $NodePath -ErrorAction Stop).Path
    $nodeRoot = Split-Path -Parent $resolvedNode
    $npmCli = Join-Path $nodeRoot "node_modules\npm\bin\npm-cli.js"
    if (-not (Test-Path -LiteralPath $npmCli -PathType Leaf)) {
        throw "npm CLI script is missing next to node.exe: $npmCli"
    }
    return (Resolve-Path -LiteralPath $npmCli -ErrorAction Stop).Path
}

function Assert-DevePlaywrightCorePackage {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [string]$ModulePath,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedVersion
    )

    if (-not (Test-Path -LiteralPath $ModulePath -PathType Leaf)) {
        throw "playwright-core package metadata is missing: $ModulePath"
    }
    try {
        $package = Get-Content -Raw -LiteralPath $ModulePath -ErrorAction Stop | ConvertFrom-Json
    } catch {
        throw "playwright-core package metadata is invalid JSON: $ModulePath"
    }
    if ($package.name -cne "playwright-core") {
        throw "playwright-core package name mismatch: $($package.name)"
    }
    if ($package.version -cne $ExpectedVersion) {
        throw "playwright-core package version mismatch: expected $ExpectedVersion, got $($package.version)"
    }
}

function ConvertTo-DeveWindowsProcessArgument {
    [CmdletBinding()]
    param(
        [AllowEmptyString()]
        [Parameter(Mandatory = $true)]
        [string]$Argument
    )

    if ($Argument.Length -gt 0 -and $Argument -notmatch '[\s"]') {
        return $Argument
    }

    $quoted = [System.Text.StringBuilder]::new()
    [void]$quoted.Append('"')
    $backslashes = 0
    foreach ($character in $Argument.ToCharArray()) {
        if ($character -eq '\') {
            $backslashes += 1
            continue
        }
        if ($character -eq '"') {
            [void]$quoted.Append(('\' * (($backslashes * 2) + 1)))
            [void]$quoted.Append('"')
        } else {
            [void]$quoted.Append(('\' * $backslashes))
            [void]$quoted.Append($character)
        }
        $backslashes = 0
    }
    [void]$quoted.Append(('\' * ($backslashes * 2)))
    [void]$quoted.Append('"')
    return $quoted.ToString()
}

function Invoke-DeveNodeProcessWithTimeout {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [string]$NodePath,
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments,
        [Parameter(Mandatory = $true)]
        [ValidateRange(1, 86400)]
        [int]$TimeoutSeconds,
        [Parameter(Mandatory = $true)]
        [string]$Operation
    )

    $resolvedNode = [System.IO.Path]::GetFullPath($NodePath)
    if (-not (Test-Path -LiteralPath $resolvedNode -PathType Leaf)) {
        throw "$Operation node executable is missing: $resolvedNode"
    }

    $startInfo = New-Object System.Diagnostics.ProcessStartInfo
    $startInfo.FileName = $resolvedNode
    $startInfo.UseShellExecute = $false
    $startInfo.Arguments = (($Arguments | ForEach-Object {
        ConvertTo-DeveWindowsProcessArgument -Argument $_
    }) -join ' ')

    $process = [System.Diagnostics.Process]::Start($startInfo)
    if ($null -eq $process) {
        throw "$Operation failed to start"
    }
    try {
        if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
            $taskkill = Join-Path $env:SystemRoot "System32\taskkill.exe"
            if (-not (Test-Path -LiteralPath $taskkill -PathType Leaf)) {
                throw "$Operation exceeded ${TimeoutSeconds}s and taskkill.exe is unavailable"
            }
            & $taskkill /PID $process.Id /T /F 2>$null | Out-Null
            $taskkillExit = $LASTEXITCODE
            if (-not $process.WaitForExit(10000)) {
                throw "$Operation exceeded ${TimeoutSeconds}s and its process tree did not stop"
            }
            $process.WaitForExit()
            if ($taskkillExit -ne 0) {
                throw "$Operation exceeded ${TimeoutSeconds}s and taskkill.exe failed (exit $taskkillExit)"
            }
            throw "$Operation exceeded ${TimeoutSeconds}s"
        }
        $process.WaitForExit()
        if ($process.ExitCode -ne 0) {
            throw "$Operation failed (exit $($process.ExitCode))"
        }
    } finally {
        $process.Dispose()
    }
}

function Install-DevePlaywrightCore {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [string]$PlaywrightRoot,
        [Parameter(Mandatory = $true)]
        [ValidateRange(1, 86400)]
        [int]$TimeoutSeconds,
        [string]$Version = "1.58.2"
    )

    $modulePath = Join-Path $PlaywrightRoot "node_modules\playwright-core\package.json"
    if (Test-Path -LiteralPath $modulePath -PathType Leaf) {
        Assert-DevePlaywrightCorePackage `
            -ModulePath $modulePath `
            -ExpectedVersion $Version
        return
    }

    $node = (Get-Command node.exe -ErrorAction Stop).Source
    $npmCli = Resolve-DeveNpmCliPath -NodePath $node
    Invoke-DeveNodeProcessWithTimeout `
        -NodePath $node `
        -Arguments @(
            $npmCli,
            "--prefix", $PlaywrightRoot,
            "install", "--no-audit", "--no-fund", "playwright-core@$Version"
        ) `
        -TimeoutSeconds $TimeoutSeconds `
        -Operation "playwright-core install"

    Assert-DevePlaywrightCorePackage `
        -ModulePath $modulePath `
        -ExpectedVersion $Version
}
