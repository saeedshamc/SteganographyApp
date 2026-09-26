# Flutter ↔ stego-core FFI

Native library: workspace crate `crates/stego-ffi` (`cdylib` / `staticlib`).

## Quick build (Windows host)

From the repo root:

```powershell
.\scripts\build-mobile-native.ps1
cd apps\mobile
flutter pub get
flutter run -d windows
```

The script builds `stego-ffi` in release and copies `stego_ffi.dll` into `apps/mobile/native/` so the Windows Flutter runner can install it next to the exe.

## Android (optional NDK)

```powershell
# requires Android NDK + cargo-ndk
cargo install cargo-ndk
.\scripts\build-mobile-native.ps1 -Android
cd apps\mobile
flutter run
```

This places `libstego_ffi.so` under `android/app/src/main/jniLibs/arm64-v8a` (and armeabi-v7a when built).

## C API

| Symbol | Role |
|--------|------|
| `stego_version` | Version string (free with `stego_string_free`) |
| `stego_plan` | Plan hide → JSON |
| `stego_hide` | Hide file payload → writes output + JSON |
| `stego_extract` | Extract → file or text JSON |
| `stego_string_free` | Free returned C strings |

Dart: `lib/stego_ffi.dart`. UI: Hide / Extract / About in `lib/main.dart`.

## UX rules (same as desktop)

1. Extract → Save.
2. Optional Open/Run only after explicit confirm(s).
3. No OS auto-run of cover files in gallery/Photos.
