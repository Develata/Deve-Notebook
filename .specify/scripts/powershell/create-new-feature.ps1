#!/usr/bin/env pwsh
[CmdletBinding()]
param(
    [switch]$Json,
    [string]$ShortName,
    [int]$Number = 0,
    [switch]$Help,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$FeatureDescription
)
$ErrorActionPreference = 'Stop'

 . "$PSScriptRoot/common.ps1"
 . "$PSScriptRoot/create-new-feature-support.ps1"

if ($Help) {
    Write-Host "Usage: ./create-new-feature.ps1 [-Json] [-ShortName <name>] [-Number N] <feature description>"
    Write-Host "  -Json               Output results in JSON"
    Write-Host "  -ShortName <name>   Use an explicit branch suffix"
    Write-Host "  -Number N           Override auto-numbering"
    Write-Host "  -Help               Show this help message"
    exit 0
}

if (-not $FeatureDescription -or $FeatureDescription.Count -eq 0) {
    Write-Error "Usage: ./create-new-feature.ps1 [-Json] [-ShortName <name>] <feature description>"
    exit 1
}

$featureDesc = ($FeatureDescription -join ' ').Trim()
$repoRoot = Get-RepoRoot
$hasGit = Test-HasGit
if (-not $repoRoot) {
    Write-Error "Could not determine repository root."
    exit 1
}

Set-Location $repoRoot

$specsDir = Join-Path $repoRoot 'specs'
New-Item -ItemType Directory -Path $specsDir -Force | Out-Null

$branchSuffix = if ($ShortName) {
    ConvertTo-CleanBranchName -Name $ShortName
} else {
    Get-BranchSuffixFromDescription -Description $featureDesc
}
if (-not $branchSuffix) {
    Write-Error "Could not derive a valid short name from feature description."
    exit 1
}

if ($Number -eq 0) {
    $Number = Get-NextBranchNumber -SpecsDir $specsDir -ShortName $branchSuffix -HasGit:$hasGit
}

$featureNum = ('{0:000}' -f $Number)
$branchName = "$featureNum-$branchSuffix"

$maxBranchLength = 244
if ($branchName.Length -gt $maxBranchLength) {
    $maxSuffixLength = $maxBranchLength - 4
    $truncatedSuffix = $branchSuffix.Substring(0, [Math]::Min($branchSuffix.Length, $maxSuffixLength))
    $truncatedSuffix = $truncatedSuffix -replace '-$', ''
    $originalBranchName = $branchName
    $branchName = "$featureNum-$truncatedSuffix"
    Write-Warning "[specify] Branch name exceeded GitHub's 244-byte limit"
    Write-Warning "[specify] Original: $originalBranchName ($($originalBranchName.Length) bytes)"
    Write-Warning "[specify] Truncated to: $branchName ($($branchName.Length) bytes)"
}

if ($hasGit) {
    try { git checkout -b $branchName | Out-Null } catch {
        Write-Warning "Failed to create git branch: $branchName"
    }
} else {
    Write-Warning "[specify] Warning: Git repository not detected; skipped branch creation for $branchName"
}

$featureDir = Join-Path $specsDir $branchName
New-Item -ItemType Directory -Path $featureDir -Force | Out-Null

$template = Join-Path $repoRoot '.specify/templates/spec-template.md'
$specFile = Join-Path $featureDir 'spec.md'
if (Test-Path $template) {
    Copy-Item $template $specFile -Force
} else {
    New-Item -ItemType File -Path $specFile | Out-Null
}

$env:SPECIFY_FEATURE = $branchName

if ($Json) {
    [PSCustomObject]@{
        REPO_ROOT = $repoRoot
        BRANCH_NAME = $branchName
        FEATURE_DIR = $featureDir
        SPEC_FILE = $specFile
        FEATURE_NUM = $featureNum
        SHORT_NAME = $branchSuffix
        HAS_GIT = $hasGit
    } | ConvertTo-Json -Compress
} else {
    Write-Output "REPO_ROOT: $repoRoot"
    Write-Output "BRANCH_NAME: $branchName"
    Write-Output "FEATURE_DIR: $featureDir"
    Write-Output "SPEC_FILE: $specFile"
    Write-Output "FEATURE_NUM: $featureNum"
    Write-Output "SHORT_NAME: $branchSuffix"
    Write-Output "HAS_GIT: $hasGit"
    Write-Output "SPECIFY_FEATURE environment variable set to: $branchName"
}
