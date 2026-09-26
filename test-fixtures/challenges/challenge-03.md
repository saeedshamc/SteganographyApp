# Challenge 03 — Method B ZIP caveat

**Goal:** Understand EOF append on archives.

1. Hide a tiny payload in a `.zip` cover with Method B.
2. Confirm `unzip -t` (or Explorer) still lists original members.
3. Extract with Open Stego and verify the payload checksum.

**Instructor notes:** Most unzip tools ignore bytes after EOCD; strict validators / AV may complain. Open Stego does **not** rewrite ZIP central-directory offsets — documented in `format_aware` caveats.
