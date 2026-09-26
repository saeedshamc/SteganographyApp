/// Placeholder FFI surface for stego-core.
///
/// Next step: generate bindings (flutter_rust_bridge / cbindgen) that call
/// `hide_with` / `extract_with` from the Rust workspace crate.
class StegoFfi {
  static String statusMessage() =>
      'stub — link stego-core via FFI (see apps/mobile/rust/README.md)';
}
