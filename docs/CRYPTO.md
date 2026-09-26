# Cryptography

## Key derivation

- **Argon2id** (RFC 9106), version `0x13`
- Profiles (`--profile` / GUI later):

| Profile | Memory (KiB) | Time | Notes |
|---------|--------------|------|--------|
| `fast` | 8192 | 1 | demos / weak machines |
| `balanced` (default) | 19456 | 2 | historical v1 envelopes |
| `paranoid` | 65536 | 3 | slower, stronger |

- Optional **keyfile**: Argon2 password input = `password_bytes || 0xFF || keyfile_bytes`
- Random **16-byte salt** per encryption
- Argon2 output: 64 bytes master secret
- **HKDF-SHA256** expands master into:
  - `EncKey` (32 B) — info `OpenStego/enc/v1`
  - `LocatorKey` (32 B) — info `OpenStego/locator/v1`

## Encryption

- **AES-256-GCM**
- Random **12-byte nonce** per encryption
- Envelope layouts:

```text
v1 (balanced, no keyfile — backward compatible):
  version=1 || salt(16) || nonce(12) || ciphertext || tag(16)

v2 (fast/paranoid and/or keyfile):
  version=2 || profile_id(u8) || salt(16) || nonce(12) || ciphertext || tag(16)
```

`profile_id`: 1=fast, 2=balanced, 3=paranoid.

## Wrong password / tampering

AES-GCM authentication failure surfaces as `StegoError::WrongPassword` (indistinguishable from a wrong passphrase by design). Never returns plaintext on failure.
