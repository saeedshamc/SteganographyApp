//! Lightweight fuzz-style parsing checks (no external fuzzer required for CI).

#[cfg(test)]
mod fuzz_parse {
    use stego_core::crypto::{decrypt_blob, encrypt_blob};
    use stego_core::metadata::{unwrap_payload, wrap_payload, PayloadMeta};

    #[test]
    fn randomish_blobs_do_not_panic_on_unwrap() {
        let samples: &[&[u8]] = &[
            b"",
            b"OSMP",
            b"OSMP\x01\x00",
            &[0xff; 64],
            b"OSMP\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
        ];
        for s in samples {
            let _ = unwrap_payload(s);
        }
    }

    #[test]
    fn decrypt_garbage_is_err_not_panic() {
        let _ = decrypt_blob(&[1, 2, 3, 4, 5], "x");
        let good = encrypt_blob(b"hi", "pw").unwrap();
        let mut bad = good.clone();
        if let Some(last) = bad.last_mut() {
            *last ^= 0x5a;
        }
        assert!(decrypt_blob(&bad, "pw").is_err());
        let meta = PayloadMeta::for_text(b"a");
        let wrapped = wrap_payload(&meta, b"a").unwrap();
        assert!(unwrap_payload(&wrapped).is_ok());
    }
}
