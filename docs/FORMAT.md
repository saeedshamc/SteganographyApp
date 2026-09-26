# Container format (OpenStego)

> Outline — detailed byte layout lands with crypto/metadata/embedding stages.

## Goals

- Single encrypted blob embedding for both Method A (LSB) and Method B (EOF)
- Recover original filename / extension or raw text flag
- Integrity check (SHA-256) after decryption

## High-level envelope

```
[salt][nonce][AES-GCM ciphertext of metadata+payload]
```

Method B adds a keyed locator footer after the cover bytes. Method A spreads the envelope across image LSBs with a password-derived permutation.

## Version

`FORMAT_VERSION` in `stego-core::container` tracks breaking changes.
