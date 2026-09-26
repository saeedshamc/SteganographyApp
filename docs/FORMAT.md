# Container format (OpenStego)

`FORMAT_VERSION` = `1` (balanced, no keyfile) or `2` (explicit KDF profile / keyfile). See [CRYPTO.md](CRYPTO.md).

## Encrypted envelope

```text
v1: version(1) || salt(16) || nonce(12) || ciphertext||tag
v2: version(2) || profile(u8) || salt(16) || nonce(12) || ciphertext||tag
```

Plaintext inside the ciphertext is the **metadata blob** below.

## Metadata blob (magic `OSMP`)

```text
"OSMP" | version(u8=1) | flags(u8) | name_len(u16 LE) | name UTF-8
       | ext_len(u16 LE) | extension UTF-8 | size(u64 LE)
       | sha256(32) | payload[size]
```

- `flags bit0` = payload is raw text/code (no filename)
- `flags bit1` = payload is an executable/script (educational extract-then-run demos); mutually exclusive with bit0
- Older files without bit1 remain ordinary file payloads
- `sha256` covers payload bytes only; verified on unwrap

## Method B footer (after cover + envelope)

```text
cover || envelope || salt(16) || hmac(32) || envelope_len(u64 LE)
```

`hmac = HMAC-SHA256(LocatorKey, envelope)`. Salt mirrors the envelope salt so the locator can be checked before decrypt.

## Method A bit stream (RGB LSBs)

1. First 128 sequential channel LSBs = salt (16 bytes)
2. Remaining channel bits shuffled with ChaCha20Rng(LocatorKey)
3. Shuffled stream = `envelope_len(u64 LE) || envelope`

Output image is always PNG.

## Semver / format versions

- App crate versions follow workspace `0.4.x` (Phase 3 product polish).
- Envelope: `FORMAT_VERSION` 1 (balanced) or 2 (profile/keyfile) — see [CRYPTO.md](CRYPTO.md).
- Metadata OSMP version remains `1` with additive flag bits.
- Breaking on-disk changes require a new version byte and a migration note here.
