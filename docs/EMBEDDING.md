# Embedding methods

## Method A — keyed random LSB (PNG / BMP)

- Embed into least-significant bits of pixel channels
- Bit/pixel order is a password-derived permutation (not sequential)
- Optional **adaptive** mode prefers high-variance pixels (lower capacity)
- **LSB depth** 1 (default) or 2 (educational denser capacity estimate; noisier) with capacity-risk warnings
- Output always **PNG** (lossless). JPEG covers are rejected or converted with a clear warning
- Capacity is finite and shown to the user
- Implemented in `stego_core::embedding::method_a_lsb`

## Method B — keyed EOF append (everything else)

Layout (from start of file):

```text
cover_bytes || encrypted_envelope || salt(16) || hmac(32) || envelope_len(u64 LE)
```

- `hmac = HMAC-SHA256(LocatorKey, envelope)`
- `LocatorKey` from Argon2id + HKDF (same KDF as encryption; salt is the envelope salt, also mirrored in the footer for discovery)
- No fixed plaintext magic string in the clear
- Capacity effectively unbounded; show final output size
- **Format-aware helpers** (`format_aware`):
  - PDF: terminal `%%EOF` (last ≤16 bytes) is moved after the footer when present
  - ZIP / MP3: plain EOF append with explicit caveats
  - Unknown: same EOF fallback
- **Caveat:** rare formats that validate “no trailing bytes” may break — disclosed in the UI
- Tested against PDF-/MP3-/ZIP-like fixtures in unit tests

## Auto-detection

Chosen from cover extension / type before hide proceeds (`stego_core::detection`).
