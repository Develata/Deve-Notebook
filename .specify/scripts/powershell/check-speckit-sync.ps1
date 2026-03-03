Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$dirs = @(
    ".opencode/command",
    ".codex/prompts",
    ".claude/commands"
)

function Assert-Dir([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        throw "Directory not found: $Path"
    }
}

function Normalize-Content([string]$Path) {
    $content = Get-Content -LiteralPath $Path -Raw
    $content = $content -replace "`r`n", "`n"
    $content = $content.TrimEnd("`r", "`n")
    if ([System.IO.Path]::GetFileName($Path) -eq "speckit.plan.md") {
        $content = $content -replace "-AgentType\s+(opencode|codex|claude)", "-AgentType __AGENT__"
    }
    return $content
}

foreach ($dir in $dirs) {
    Assert-Dir -Path $dir
}

$baseNames = Get-ChildItem -LiteralPath $dirs[0] -Filter "speckit.*.md" -File |
    Select-Object -ExpandProperty Name |
    Sort-Object

if ($baseNames.Count -eq 0) {
    throw "No speckit.*.md files found in $($dirs[0])"
}

foreach ($dir in $dirs[1..($dirs.Count - 1)]) {
    $names = Get-ChildItem -LiteralPath $dir -Filter "speckit.*.md" -File |
        Select-Object -ExpandProperty Name |
        Sort-Object
    if ((@($baseNames) -join "`n") -ne (@($names) -join "`n")) {
        throw "File set mismatch in $dir"
    }
}

$mismatches = @()
foreach ($name in $baseNames) {
    $normalized = foreach ($dir in $dirs) {
        Normalize-Content -Path (Join-Path -Path $dir -ChildPath $name)
    }
    $uniqueNormalized = @($normalized | Select-Object -Unique)
    if ($uniqueNormalized.Count -ne 1) {
        $mismatches += $name
    }
}

if ($mismatches.Count -gt 0) {
    Write-Host "Speckit sync mismatch:" -ForegroundColor Red
    foreach ($file in $mismatches) {
        Write-Host "  - $file" -ForegroundColor Red
    }
    exit 1
}

Write-Host "Speckit directories are synchronized (plan AgentType variance allowed)." -ForegroundColor Green
