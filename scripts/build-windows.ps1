# Build Open Stego for Windows: CLI + NSIS/MSI installers
# Usage (from repo root):
#   .\scripts\build-windows.ps1
#   .\scripts\build-windows.ps1 -CliOnly
#   .\scripts\build-windows.ps1 -GuiOnly

param(
    [switch]$CliOnly,
    [switch]$GuiOnly
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

Write-Host "==> Repo: $Root" -ForegroundColor Cyan

if (-not $GuiOnly) {
    Write-Host "==> Building CLI (release)..." -ForegroundColor Cyan
    cargo build -p stego-cli --release
    $cli = Join-Path $Root "target\release\stego.exe"
    if (-not (Test-Path $cli)) {
        throw "CLI binary not found at $cli"
    }
    Write-Host "    OK: $cli" -ForegroundColor Green
}

if (-not $CliOnly) {
    Write-Host "==> Installing npm deps + building Windows installers (NSIS, MSI)..." -ForegroundColor Cyan
    Push-Location (Join-Path $Root "apps\desktop")
    try {
        if (-not (Test-Path "node_modules")) {
            npm install
        }
        npm run build:win
    }
    finally {
        Pop-Location
    }

    $bundleCandidates = @(
        (Join-Path $Root "apps\desktop\src-tauri\target\release\bundle"),
        (Join-Path $Root "target\release\bundle")
    )
    Write-Host "==> Looking for installers..." -ForegroundColor Cyan
    $found = $false
    foreach ($dir in $bundleCandidates) {
        if (Test-Path $dir) {
            Get-ChildItem -Recurse $dir -Include *.exe,*.msi | ForEach-Object {
                Write-Host "    $($_.FullName)" -ForegroundColor Green
                $found = $true
            }
        }
    }
    if (-not $found) {
        Write-Host "    No .exe/.msi under bundle\ yet — check Tauri build log above." -ForegroundColor Yellow
    }
}

Write-Host "==> Done." -ForegroundColor Cyan
