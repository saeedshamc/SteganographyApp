# Build stego-ffi for the Flutter mobile / Windows runner.
# Usage (repo root):
#   .\scripts\build-mobile-native.ps1
#   .\scripts\build-mobile-native.ps1 -Android

param(
    [switch]$Android
)

$ErrorActionPreference = "Continue"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

Write-Host "==> cargo build -p stego-ffi --release"
cargo build -p stego-ffi --release
if ($LASTEXITCODE -ne 0) {
    throw "cargo build failed with exit $LASTEXITCODE"
}

$targetDir = if ($env:CARGO_TARGET_DIR -and $env:CARGO_TARGET_DIR.Trim()) {
    $env:CARGO_TARGET_DIR
} else {
    Join-Path $root "target"
}
$dll = Join-Path $targetDir "release\stego_ffi.dll"
$nativeDir = Join-Path $root "apps\mobile\native"
New-Item -ItemType Directory -Force -Path $nativeDir | Out-Null
if (Test-Path $dll) {
    Copy-Item $dll (Join-Path $nativeDir "stego_ffi.dll") -Force
    Write-Host "Copied stego_ffi.dll -> apps/mobile/native/ (from $dll)"
} else {
    Write-Warning "stego_ffi.dll not found at $dll"
}

if ($Android) {
    Write-Host "==> Android NDK build via cargo-ndk"
    if (-not (Get-Command cargo-ndk -ErrorAction SilentlyContinue)) {
        throw "cargo-ndk not found. Install with: cargo install cargo-ndk"
    }
    $jni = Join-Path $root "apps\mobile\android\app\src\main\jniLibs"
    New-Item -ItemType Directory -Force -Path (Join-Path $jni "arm64-v8a") | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path $jni "armeabi-v7a") | Out-Null
    cargo ndk -t arm64-v8a -t armeabi-v7a -o $jni build -p stego-ffi --release
    if ($LASTEXITCODE -ne 0) {
        throw "cargo ndk failed with exit $LASTEXITCODE"
    }
    Write-Host "Android .so libraries written under apps/mobile/android/app/src/main/jniLibs/"
}

Write-Host "Done."
