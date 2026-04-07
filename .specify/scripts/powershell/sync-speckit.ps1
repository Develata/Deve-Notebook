param(
    [string]$SourceDir = ".opencode/command"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)

$targets = @(
    @{ Dir = ".codex/prompts"; Agent = "codex" },
    @{ Dir = ".claude/commands"; Agent = "claude" }
)

function Assert-Dir([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        throw "Directory not found: $Path"
    }
}

Assert-Dir -Path $SourceDir
foreach ($target in $targets) {
    Assert-Dir -Path $target.Dir
}

$sourceFiles = Get-ChildItem -LiteralPath $SourceDir -Filter "speckit.*.md" -File
if ($sourceFiles.Count -eq 0) {
    throw "No speckit.*.md files found in: $SourceDir"
}

$normalFiles = $sourceFiles | Where-Object { $_.Name -ne "speckit.plan.md" }
foreach ($file in $normalFiles) {
    foreach ($target in $targets) {
        $dest = Join-Path -Path $target.Dir -ChildPath $file.Name
        Copy-Item -LiteralPath $file.FullName -Destination $dest -Force
    }
}

$planPath = Join-Path -Path $SourceDir -ChildPath "speckit.plan.md"
if (-not (Test-Path -LiteralPath $planPath -PathType Leaf)) {
    throw "Missing plan file in source: $planPath"
}

$planContent = Get-Content -LiteralPath $planPath -Raw -Encoding UTF8
foreach ($target in $targets) {
    $destPlan = Join-Path -Path $target.Dir -ChildPath "speckit.plan.md"
    $adapted = $planContent -replace "-AgentType\s+\w+", "-AgentType $($target.Agent)"
    [System.IO.File]::WriteAllText($destPlan, $adapted, $utf8NoBom)
}

Write-Host "Synced $($sourceFiles.Count) speckit files from $SourceDir"
foreach ($target in $targets) {
    Write-Host "  -> $($target.Dir)"
}
