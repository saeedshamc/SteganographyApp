# Threat model

> Outline — expanded in stage 10.

## Intended use

Privacy, education, research, and personal encrypted backups via steganography. Not malware delivery or abuse tooling.

## Protects against

- Casual inspection of cover files
- Offline password guessing limited by Argon2id cost
- Bit-flip / truncation of ciphertext (AES-GCM + checksum)

## Does not claim

- Undetectability against a determined forensic analyst
- Security if the password is weak or reused
- Survival through lossy recompression (e.g. re-encoded JPEG) for Method A
- Compatibility with every container format for Method B trailing data
