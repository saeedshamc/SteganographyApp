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
    Write-Warning "stego_ffi.dll not found at $dll (ok on non-Windows hosts)"
}

if ($Android) {
    Write-Host "==> Android NDK build via cargo-ndk"

    if (-not (Get-Command cargo-ndk -ErrorAction SilentlyContinue)) {
        throw "cargo-ndk not found. Install: cargo install cargo-ndk"
    }

    $sdk = $env:ANDROID_HOME
    if (-not $sdk) { $sdk = $env:ANDROID_SDK_ROOT }
    if (-not $sdk) { $sdk = Join-Path $env:LOCALAPPDATA "Android\sdk" }

    if (-not $env:ANDROID_NDK_HOME -or -not (Test-Path $env:ANDROID_NDK_HOME)) {
        $ndkRoot = Join-Path $sdk "ndk"
        if (Test-Path $ndkRoot) {
            $latest = Get-ChildItem $ndkRoot -Directory | Sort-Object Name -Descending | Select-Object -First 1
            if ($latest) {
                $env:ANDROID_NDK_HOME = $latest.FullName
                Write-Host "Using ANDROID_NDK_HOME=$($env:ANDROID_NDK_HOME)"
            }
        }
    }

    if (-not $env:ANDROID_NDK_HOME -or -not (Test-Path $env:ANDROID_NDK_HOME)) {
        throw @"
ANDROID_NDK_HOME is not set and no NDK was found under $sdk\ndk.

Install an NDK (Android Studio SDK Manager) or:
  sdkmanager ""ndk;27.0.12077973""
Then set:
  `$env:ANDROID_NDK_HOME = ""$sdk\ndk\<version>""

See docs/ANDROID.md
"@
    }

    rustup target add aarch64-linux-android armv7-linux-androideabi 2>&1 | Out-Null

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
