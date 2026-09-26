# Container format (OpenStego)

`FORMAT_VERSION` = `1` (`stego_core::container`).

## Encrypted envelope (common to Method A and B)

Produced by `stego_core::crypto::encrypt_blob`:

```text
offset  size  field
0       1     version (= 1)
1       16    salt (Argon2id)
17      12    AES-GCM nonce
29      n+16  ciphertext || 16-byte GCM tag
```

Plaintext inside the ciphertext is the **metadata blob** below.

## Metadata blob (magic `OSMP`)

```text
"OSMP" | version(u8=1) | flags(u8) | name_len(u16 LE) | name UTF-8
       | ext_len(u16 LE) | extension UTF-8 | size(u64 LE)
       | sha256(32) | payload[size]
```

- `flags bit0` = payload is raw text/code (no filename)
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
