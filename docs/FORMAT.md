# Container format (OpenStego)

> Outline — detailed byte layout lands with crypto/metadata/embedding stages.

## Goals

- Single encrypted blob embedding for both Method A (LSB) and Method B (EOF)
- Recover original filename / extension or raw text flag
- Integrity check (SHA-256) after decryption

## High-level envelope (after encryption)

```
version || salt || nonce || AES-GCM(ciphertext of metadata blob)
```

## Metadata blob (inside ciphertext) — magic `OSMP`

```
magic "OSMP" | version | flags | name_len | name | ext_len | ext | size | sha256 | payload
```

See `stego_core::metadata` for exact endianness and flag bits.
