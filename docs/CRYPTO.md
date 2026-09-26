# Cryptography

## Key derivation

- **Argon2id** (RFC 9106), version `0x13`
- Parameters: memory **19456 KiB** (~19 MiB), time cost **2**, parallelism **1**
- Random **16-byte salt** per encryption
- Argon2 output: 64 bytes master secret
- **HKDF-SHA256** expands master into:
  - `EncKey` (32 B) — info `OpenStego/enc/v1`
  - `LocatorKey` (32 B) — info `OpenStego/locator/v1`

## Encryption

- **AES-256-GCM**
- Random **12-byte nonce** per encryption
- Envelope layout:

```text
version (1 byte = 1) || salt (16) || nonce (12) || ciphertext || tag (16)
```

## Wrong password / tampering

AES-GCM authentication failure surfaces as `StegoError::WrongPassword` (indistinguishable from a wrong passphrase by design). Never returns plaintext on failure.
