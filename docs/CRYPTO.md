# Cryptography

> Outline — implemented in stage 2.

## Key derivation

- **Argon2id** over the user password with a random salt
- Derive two keys: **EncKey** (AES-256) and **LocatorKey** (HMAC / PRNG seed)

## Encryption

- **AES-256-GCM** over the metadata+payload blob
- Random 12-byte nonce per encryption
- Authentication tag prevents silent tampering

## Wrong password

Decryption must fail with a clear error — never return garbage as a “successful” extract.
