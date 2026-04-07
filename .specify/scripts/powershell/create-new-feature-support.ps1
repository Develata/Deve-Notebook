#!/usr/bin/env pwsh

function ConvertTo-CleanBranchName {
    param([string]$Name)
    $Name.ToLower() -replace '[^a-z0-9]', '-' -replace '-{2,}', '-' -replace '^-', '' -replace '-$', ''
}

function Get-BranchSuffixFromDescription {
    param([string]$Description)

    $stopWords = @(
        'i','a','an','the','to','for','of','in','on','at','by','with','from',
        'is','are','was','were','be','been','being','have','has','had',
        'do','does','did','will','would','should','could','can','may','might','must','shall',
        'this','that','these','those','my','your','our','their','want','need','add','get','set'
    )
    $words = (($Description.ToLower() -replace '[^a-z0-9\s]', ' ') -split '\s+') | Where-Object { $_ }
    $meaningfulWords = foreach ($word in $words) {
        if ($stopWords -contains $word) { continue }
        if ($word.Length -ge 3 -or $Description -match "\b$($word.ToUpper())\b") { $word }
    }
    if ($meaningfulWords.Count -gt 0) {
        $maxWords = if ($meaningfulWords.Count -eq 4) { 4 } else { 3 }
        return (($meaningfulWords | Select-Object -First $maxWords) -join '-')
    }
    $fallbackWords = (ConvertTo-CleanBranchName -Name $Description) -split '-' | Where-Object { $_ } | Select-Object -First 3
    [string]::Join('-', $fallbackWords)
}

function Get-HighestNumberFromSpecs {
    param([string]$SpecsDir, [string]$ShortName)
    $highest = 0
    if (-not (Test-Path $SpecsDir)) { return $highest }
    Get-ChildItem -Path $SpecsDir -Directory | ForEach-Object {
        if ($_.Name -match '^(\d+)-(.+)$' -and $Matches[2] -eq $ShortName) {
            $highest = [Math]::Max($highest, [int]$Matches[1])
        }
    }
    $highest
}

function Get-HighestNumberFromBranches {
    param([string]$ShortName)
    $highest = 0
    try {
        $branches = git branch -a 2>$null
        if ($LASTEXITCODE -ne 0) { return $highest }
        foreach ($branch in $branches) {
            $cleanBranch = $branch.Trim() -replace '^\*?\s+', '' -replace '^remotes/[^/]+/', ''
            if ($cleanBranch -match '^(\d+)-(.+)$' -and $Matches[2] -eq $ShortName) {
                $highest = [Math]::Max($highest, [int]$Matches[1])
            }
        }
    } catch {
        Write-Verbose "Could not inspect Git branches: $_"
    }
    $highest
}

function Get-NextBranchNumber {
    param([string]$SpecsDir, [string]$ShortName, [bool]$HasGit)

    if ($HasGit) {
        try { git fetch --all --prune 2>$null | Out-Null } catch {}
    }
    $highestSpec = Get-HighestNumberFromSpecs -SpecsDir $SpecsDir -ShortName $ShortName
    $highestBranch = if ($HasGit) { Get-HighestNumberFromBranches -ShortName $ShortName } else { 0 }
    ([Math]::Max($highestSpec, $highestBranch) + 1)
}
