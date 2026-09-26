# Embedding methods

> Outline — Method B in stage 4, Method A in stage 5, detection in stage 6.

## Method A — keyed random LSB (PNG / BMP)

- Embed into least-significant bits of pixel channels
- Bit/pixel order is a password-derived permutation (not sequential)
- Output always **PNG** (lossless). JPEG covers are rejected or converted with a clear warning
- Capacity is finite and shown to the user

## Method B — keyed EOF append (everything else)

- Append ciphertext after the cover’s legitimate end-of-data
- Locator uses a **keyed** marker (HMAC), not a fixed plaintext magic string
- Capacity effectively unbounded; show final output size
- **Caveat:** rare formats that validate “no trailing bytes” may break — disclosed in the UI

## Auto-detection

Chosen from cover extension / type before hide proceeds.
