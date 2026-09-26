# Flutter ↔ stego-core FFI

1. Build `stego-core` as a `cdylib` (or use `flutter_rust_bridge`).
2. Expose `hide_with` / `extract_with` / `plan_hide_with` matching `docs/FORMAT.md`.
3. Mirror desktop UX: Extract → Save → optional Run with explicit confirm.
4. No OS auto-run of cover files.

Until the bridge is wired, `lib/stego_ffi.dart` remains a stub so the Flutter app can compile once a Flutter SDK is available (`flutter create .` may still be needed for platform folders).
