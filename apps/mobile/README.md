# Open Stego Mobile

Flutter client over `stego-ffi` → `stego-core`. Same encrypted format as desktop/CLI.

## Setup

```powershell
# repo root
.\scripts\build-mobile-native.ps1
cd apps\mobile
flutter pub get
flutter analyze
flutter run -d windows
```

Android (after NDK `.so` is built with `-Android`):

```powershell
flutter run
```

## Features

- **Hide** — pick cover + payload, password, LSB depth, plan summary, save stego
- **Extract** — recover text or file; optional Open/Run with confirm (educational)
- **About** — native bridge status / version

Version tracks workspace semver via `pubspec.yaml` (`0.4.0+40` = 0.4.0, build 40).
