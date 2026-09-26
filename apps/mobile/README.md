# Open Stego Mobile

Flutter client over `stego-ffi` → `stego-core`.

```text
cargo build -p stego-ffi --release
# place stego_ffi.dll / libstego_ffi.so where the Dart loader can find it
flutter pub get
flutter run   # after `flutter create .` if platform folders are missing
```

See [rust/README.md](rust/README.md) for the C API and UX rules.
