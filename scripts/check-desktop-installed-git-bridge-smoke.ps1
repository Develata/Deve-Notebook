param(
    [Parameter(Mandatory = $true)]
    [string]$DesktopBinary,
    [Parameter(Mandatory = $true)]
    [string]$WorkRoot
)

$ErrorActionPreference = "Stop"

function Invoke-Checked {
    param(
        [Parameter(Mandatory = $true)]
        [string]$FilePath,
        [Parameter(ValueFromRemainingArguments = $true)]
        [string[]]$Arguments
    )

    $output = & $FilePath @Arguments 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "command failed ($LASTEXITCODE): $FilePath $($Arguments -join ' ')`n$($output -join "`n")"
    }
    return $output
}

$desktop = (Resolve-Path -LiteralPath $DesktopBinary).Path
$installDir = Split-Path -Parent $desktop
$cli = Join-Path $installDir "deve_cli.exe"
if (-not (Test-Path -LiteralPath $cli -PathType Leaf)) {
    throw "installed sidecar is missing: $cli"
}
$cli = (Resolve-Path -LiteralPath $cli).Path

$gitCommand = Get-Command git.exe -CommandType Application -ErrorAction Stop | Select-Object -First 1
$git = (Resolve-Path -LiteralPath $gitCommand.Source).Path
if (-not [System.IO.Path]::IsPathRooted($git)) {
    throw "resolved Git executable is not absolute: $git"
}

New-Item -ItemType Directory -Path $WorkRoot -Force | Out-Null
$resolvedWorkRoot = (Resolve-Path -LiteralPath $WorkRoot).Path
$scenario = Join-Path $resolvedWorkRoot ("git-bridge-" + [guid]::NewGuid().ToString("N"))
$projectionBase = Join-Path $scenario "projection"
$ledgerDir = Join-Path $scenario "ledger"
$remote = Join-Path $scenario "remote.git"
New-Item -ItemType Directory -Path $scenario -Force | Out-Null

$oldLedger = $env:DEVE_LEDGER_DIR
$oldGit = $env:DEVE_GIT_EXECUTABLE
try {
    $env:DEVE_LEDGER_DIR = $ledgerDir
    $env:DEVE_GIT_EXECUTABLE = $git

    Invoke-Checked $cli init --path $scenario --repo default --projection-base $projectionBase | Out-Null
    $workspaces = @(Get-ChildItem -LiteralPath $projectionBase -Directory)
    if ($workspaces.Count -ne 1) {
        throw "expected one projection workspace, found $($workspaces.Count)"
    }
    $workspace = $workspaces[0].FullName

    Invoke-Checked $git -C $workspace init | Out-Null
    Invoke-Checked $git -C $workspace config user.email deve-installer@example.invalid | Out-Null
    Invoke-Checked $git -C $workspace config user.name "Deve Installer Smoke" | Out-Null

    Set-Content -LiteralPath (Join-Path $workspace "note.md") -Value "installed initial" -NoNewline
    Invoke-Checked $cli scan | Out-Null
    Invoke-Checked $cli sc stage --repo default --all | Out-Null
    Invoke-Checked $cli sc apply --repo default | Out-Null
    Invoke-Checked $cli sc commit --repo default --message "installed NoteGit commit" | Out-Null

    $env:DEVE_GIT_EXECUTABLE = Join-Path $scenario "missing-git.exe"
    $failureReport = (Invoke-Checked $cli ngit export --repo default) -join "`n"
    if ($failureReport -notmatch "git executable is invalid") {
        throw "invalid trusted Git path did not fail closed with the expected diagnostic:`n$failureReport"
    }

    $env:DEVE_GIT_EXECUTABLE = $git
    Invoke-Checked $cli ngit mirror --repo default --retry-out-of-sync | Out-Null
    New-Item -ItemType Directory -Path $remote -Force | Out-Null
    Invoke-Checked $git -C $remote init --bare | Out-Null
    Invoke-Checked $git -C $workspace remote add origin $remote | Out-Null
    $branch = ((Invoke-Checked $git -C $workspace branch --show-current) -join "").Trim()
    if ([string]::IsNullOrWhiteSpace($branch)) {
        throw "exported Git mirror has no named branch"
    }
    Invoke-Checked $cli ngit push --repo default --remote origin --branch $branch | Out-Null

    Set-Content -LiteralPath (Join-Path $workspace "note.md") -Value "installed import" -NoNewline
    Invoke-Checked $cli ngit import --repo default --apply | Out-Null
    Invoke-Checked $cli sc stage --repo default --all | Out-Null
    Invoke-Checked $cli sc apply --repo default | Out-Null
    Invoke-Checked $cli sc commit --repo default --message "installed imported commit" | Out-Null
    Invoke-Checked $cli ngit export --repo default | Out-Null
    Invoke-Checked $cli ngit push --repo default --remote origin --branch $branch | Out-Null

    $remoteBody = ((Invoke-Checked $git -C $remote show "refs/heads/${branch}:note.md") -join "`n").Trim()
    if ($remoteBody -ne "installed import") {
        throw "remote mirror content mismatch: $remoteBody"
    }
    Write-Output "desktop-installed-git-bridge-smoke: ok"
}
finally {
    $env:DEVE_LEDGER_DIR = $oldLedger
    $env:DEVE_GIT_EXECUTABLE = $oldGit
}
