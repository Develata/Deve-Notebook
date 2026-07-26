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

function Invoke-RepoChecked {
    param(
        [Parameter(Mandatory = $true)]
        [string]$FilePath,
        [Parameter(Mandatory = $true)]
        [string]$RepoSelector,
        [Parameter(ValueFromRemainingArguments = $true)]
        [string[]]$Arguments
    )

    $repoId = [guid]::Empty
    if (-not [guid]::TryParse($RepoSelector, [ref]$repoId)) {
        throw "repo-scoped command requires a UUID execution selector"
    }
    Invoke-Checked $FilePath @Arguments --repo $RepoSelector
}

function Read-IdentityUuid {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Identity,
        [Parameter(Mandatory = $true)]
        [ValidateSet("repo_id", "repo_name")]
        [string]$Key,
        [Parameter(Mandatory = $true)]
        [string]$IdentityPath
    )

    $pattern = "(?m)^$Key\s*=\s*(['""])([0-9A-Fa-f-]+)\1\s*$"
    $matches = [regex]::Matches($Identity, $pattern)
    if ($matches.Count -ne 1) {
        throw "initialized workspace identity must contain exactly one quoted ${Key}: $IdentityPath"
    }
    return $matches[0].Groups[2].Value
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
    $identityPath = Join-Path $workspace ".notegit\identity.toml"
    if (-not (Test-Path -LiteralPath $identityPath -PathType Leaf)) {
        throw "initialized workspace identity is missing: $identityPath"
    }
    $identity = Get-Content -Raw -LiteralPath $identityPath
    $repoIdText = Read-IdentityUuid $identity "repo_id" $identityPath
    $repoSelector = Read-IdentityUuid $identity "repo_name" $identityPath
    $repoId = [guid]::Empty
    if (
        -not [guid]::TryParse($repoIdText, [ref]$repoId) -or
        -not [string]::Equals($repoIdText, $repoSelector, [System.StringComparison]::Ordinal)
    ) {
        throw "initialized workspace identity does not bind one UUID execution selector: $identityPath"
    }

    Invoke-Checked $git -C $workspace init | Out-Null
    Invoke-Checked $git -C $workspace config user.email deve-installer@example.invalid | Out-Null
    Invoke-Checked $git -C $workspace config user.name "Deve Installer Smoke" | Out-Null

    Set-Content -LiteralPath (Join-Path $workspace "note.md") -Value "installed initial" -NoNewline
    Invoke-Checked $cli scan | Out-Null
    $initialStage = Invoke-RepoChecked $cli $repoSelector sc stage --all
    $initialStage | Out-Null
    $initialApply = Invoke-RepoChecked $cli $repoSelector sc apply
    $initialApply | Out-Null
    Invoke-RepoChecked $cli $repoSelector sc commit --message "installed NoteGit commit" | Out-Null

    $env:DEVE_GIT_EXECUTABLE = Join-Path $scenario "missing-git.exe"
    $failureReport = (Invoke-RepoChecked $cli $repoSelector ngit export) -join "`n"
    if ($failureReport -notmatch "git executable is invalid") {
        throw "invalid trusted Git path did not fail closed with the expected diagnostic:`n$failureReport"
    }

    $env:DEVE_GIT_EXECUTABLE = $git
    Invoke-RepoChecked $cli $repoSelector ngit mirror --retry-out-of-sync | Out-Null
    New-Item -ItemType Directory -Path $remote -Force | Out-Null
    Invoke-Checked $git -C $remote init --bare | Out-Null
    Invoke-Checked $git -C $workspace remote add origin $remote | Out-Null
    $branch = ((Invoke-Checked $git -C $workspace branch --show-current) -join "").Trim()
    if ([string]::IsNullOrWhiteSpace($branch)) {
        throw "exported Git mirror has no named branch"
    }
    $initialPush = Invoke-RepoChecked $cli $repoSelector ngit push --remote origin --branch $branch
    $initialPush | Out-Null

    Set-Content -LiteralPath (Join-Path $workspace "note.md") -Value "installed import" -NoNewline
    Invoke-RepoChecked $cli $repoSelector ngit import --apply | Out-Null
    $importedStage = Invoke-RepoChecked $cli $repoSelector sc stage --all
    $importedStage | Out-Null
    $importedApply = Invoke-RepoChecked $cli $repoSelector sc apply
    $importedApply | Out-Null
    Invoke-RepoChecked $cli $repoSelector sc commit --message "installed imported commit" | Out-Null
    Invoke-RepoChecked $cli $repoSelector ngit export | Out-Null
    $importedPush = Invoke-RepoChecked $cli $repoSelector ngit push --remote origin --branch $branch
    $importedPush | Out-Null

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
