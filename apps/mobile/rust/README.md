# Flutter ↔ stego-core FFI

Native library: workspace crate `crates/stego-ffi` (`cdylib` / `staticlib`).

## Build the shared library

```powershell
cargo build -p stego-ffi --release
# Windows: target\release\stego_ffi.dll
# Linux:   target/release/libstego_ffi.so
# macOS:   target/release/libstego_ffi.dylib
```

Copy/link the artifact next to the Flutter runner or set an absolute path in `StegoFfi.load(path)`.

## C API

| Symbol | Role |
|--------|------|
| `stego_version` | Version string (free with `stego_string_free`) |
| `stego_plan` | Plan hide → JSON |
| `stego_hide` | Hide file payload → writes output + JSON |
| `stego_extract` | Extract → file or text JSON |
| `stego_string_free` | Free returned C strings |

Dart bindings: `lib/stego_ffi.dart`. UI: Hide / Extract / About in `lib/main.dart`.

## UX rules (same as desktop)

1. Extract → Save.
2. Optional Run only after explicit confirm (mobile shows a dialog; does not auto-start).
3. No OS auto-run of cover files in gallery/Photos.

If Flutter SDK is missing, keep editing `lib/` here; run `flutter create .` once to generate platform folders, then `flutter run`.
