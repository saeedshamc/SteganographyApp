# Threat model

## Intended use

Privacy, education, research, and personal encrypted “hide in a normal-looking file” workflows. The repository documents every byte so students and auditors can follow along.

## Assets

- Confidentiality of the payload against someone who has the stego file but not the password
- Integrity of the recovered payload (AES-GCM + SHA-256)

## Adversaries considered

| Adversary | Expectation |
|-----------|-------------|
| Casual viewer | Unlikely to notice Method B trailing bytes or LSB noise without tools |
| Offline password guesser | Slowed by Argon2id (~19 MiB, t=2); weak passwords still fail |
| Bit-flip / truncate attacker | AES-GCM auth fails; checksum fails |
| Format-aware tool looking for fixed magic | No fixed cleartext magic for Method B locator |

## Explicit non-goals

- Undetectability against a skilled forensic analyst with statistical LSB detectors
- Surviving lossy recompression (re-encoded JPEG, aggressive PDF optimizers that strip trailing data)
- Protection if the password is short, reused, or observed
- Hiding the *existence* of a security tool on disk (this is an open project)

## Method B caveat

Some rare format validators reject unexpected trailing bytes. The UI and CLI surface this caveat; it is a known trade-off for generic-file support.

## Responsible use

Do not use this project to conceal malware, evade lawful investigation, or violate others’ rights. Transparency cuts both ways: defenders can study the format too.
