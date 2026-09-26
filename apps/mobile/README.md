# Mobile (Flutter) — Phase F scaffold

Desktop (Rust `stego-core` + CLI + Tauri) is the source of truth for the wire format in [`docs/FORMAT.md`](../../docs/FORMAT.md).

## Status

Scaffold present:

- `pubspec.yaml` + `lib/main.dart` + `lib/stego_ffi.dart` (FFI stub)
- `rust/README.md` — how to link `stego-core`

Run locally once Flutter SDK is installed:

```text
cd apps/mobile
flutter create . --project-name open_stego_mobile
flutter pub get
flutter run
```

(`flutter create` may add `android/` / `ios/` / `windows/` folders; keep the existing `lib/` sources.)

## UX rules

Same as desktop: Hide / Extract / optional Run only after explicit confirm. No OS auto-run of cover files.
