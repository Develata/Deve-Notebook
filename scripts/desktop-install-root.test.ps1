$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot "lib/desktop-install-root.ps1")

function Assert-Throws {
    param([Parameter(Mandatory)][scriptblock]$Action, [Parameter(Mandatory)][string]$Pattern)
    try {
        & $Action
        throw "action unexpectedly succeeded"
    } catch {
        if ($_.Exception.Message -eq "action unexpectedly succeeded") { throw }
        if ($_.Exception.Message -notmatch $Pattern) {
            throw "unexpected error '$($_.Exception.Message)', expected /$Pattern/"
        }
    }
}

$temporary = Join-Path ([IO.Path]::GetTempPath()) "deve-install-root-test-$([Guid]::NewGuid().ToString('N'))"
$junction = $null
try {
    $root = Join-Path $temporary "candidate"
    $nested = Join-Path $root "bin"
    New-Item -ItemType Directory -Force $nested | Out-Null
    $desktop = Join-Path $nested "deve_desktop.exe"
    $sidecar = Join-Path $nested "deve_cli.exe"
    Set-Content -LiteralPath $desktop -Value "desktop" -NoNewline
    Set-Content -LiteralPath $sidecar -Value "sidecar" -NoNewline
    Set-Content -LiteralPath (Join-Path $root ".deve-desktop-install-root.json") `
        -Value '{"schema":1,"marker":"deve-desktop-remote-browser-smoke"}' -NoNewline

    $validated = Assert-DeveDesktopInstallRoot -InstallRoot $root -DesktopBinary $desktop
    if ($validated.DesktopBinary -ne (Resolve-Path $desktop).Path) { throw "valid nested binary changed" }

    $wrongRoot = Join-Path $temporary "wrong"
    New-Item -ItemType Directory -Force $wrongRoot | Out-Null
    Set-Content -LiteralPath (Join-Path $wrongRoot ".deve-desktop-install-root.json") `
        -Value '{"schema":1,"marker":"deve-desktop-remote-browser-smoke"}' -NoNewline
    Assert-Throws { Assert-DeveDesktopInstallRoot -InstallRoot $wrongRoot -DesktopBinary $desktop } "real child"

    $prefixRoot = Join-Path $temporary "prefix"
    $prefixEscape = Join-Path $temporary "prefix-escape"
    New-Item -ItemType Directory -Force $prefixRoot, $prefixEscape | Out-Null
    Set-Content -LiteralPath (Join-Path $prefixRoot ".deve-desktop-install-root.json") `
        -Value '{"schema":1,"marker":"deve-desktop-remote-browser-smoke"}' -NoNewline
    Set-Content -LiteralPath (Join-Path $prefixEscape "deve_desktop.exe") -Value "desktop" -NoNewline
    Set-Content -LiteralPath (Join-Path $prefixEscape "deve_cli.exe") -Value "sidecar" -NoNewline
    Assert-Throws {
        Assert-DeveDesktopInstallRoot -InstallRoot $prefixRoot -DesktopBinary (Join-Path $prefixEscape "deve_desktop.exe")
    } "real child"

    $outside = Join-Path $temporary "outside"
    New-Item -ItemType Directory -Force $outside | Out-Null
    Set-Content -LiteralPath (Join-Path $outside "deve_desktop.exe") -Value "desktop" -NoNewline
    Set-Content -LiteralPath (Join-Path $outside "deve_cli.exe") -Value "sidecar" -NoNewline
    $junction = Join-Path $root "escaped"
    & cmd.exe /d /c "mklink /J `"$junction`" `"$outside`"" | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "failed to create junction regression fixture" }
    Assert-Throws {
        Assert-DeveDesktopInstallRoot -InstallRoot $root -DesktopBinary (Join-Path $junction "deve_desktop.exe")
    } "real child"
} finally {
    if ($junction -and (Test-Path -LiteralPath $junction)) { Remove-Item -LiteralPath $junction -Force }
    Remove-Item -LiteralPath $temporary -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Output "desktop-install-root.test: ok"
