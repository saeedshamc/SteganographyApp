# Build Linux .deb from this machine via Docker (works on Windows with Docker Desktop).
# Usage (repo root, PowerShell):
#   .\scripts\build-linux-deb-docker.ps1

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

Write-Host "==> Building Docker image openstego-linux-build ..." -ForegroundColor Cyan
docker build -f scripts/Dockerfile.linux-build -t openstego-linux-build .

Write-Host "==> Running Linux .deb build inside container ..." -ForegroundColor Cyan
docker run --rm `
  -v "${Root}:/app" `
  -w /app `
  openstego-linux-build `
  bash -lc "chmod +x scripts/build-linux.sh && ./scripts/build-linux.sh --deb-only"

Write-Host "==> Search host for .deb artifacts:" -ForegroundColor Cyan
Get-ChildItem -Recurse -Filter *.deb `
  -Path (Join-Path $Root "apps\desktop\src-tauri\target"), (Join-Path $Root "target") `
  -ErrorAction SilentlyContinue |
  ForEach-Object { Write-Host "    $($_.FullName)" -ForegroundColor Green }

Write-Host "==> Done." -ForegroundColor Cyan
