# Challenge 1 — executable demo

## Task

1. Hide `../demo/demo-hello.bat` (or `.sh`) into any PNG with password `lab1`.
2. Open the stego PNG in a normal image viewer — confirm it still looks like an image.
3. Extract with Open Stego / `stego extract` and optionally Run with confirm.

## Answer notes (instructor)

- Method A (LSB) for PNG; payload kind should be `executable`.
- Viewers do not execute the payload — only Open Stego decrypts it.
- See `docs/LEARNING.md`.
