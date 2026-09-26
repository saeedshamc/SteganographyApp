//! Lightweight fuzz-style parsing checks (no external fuzzer required for CI).

#[cfg(test)]
mod fuzz_parse {
    use stego_core::crypto::{decrypt_blob, encrypt_blob, parse_envelope_header};
    use stego_core::metadata::{unwrap_payload, wrap_payload, PayloadMeta};

    /// Tiny deterministic LCG so CI stays reproducible.
    struct Lcg(u64);
    impl Lcg {
        fn next_u32(&mut self) -> u32 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
            (self.0 >> 32) as u32
        }
        fn fill(&mut self, buf: &mut [u8]) {
            for b in buf.iter_mut() {
                *b = (self.next_u32() & 0xff) as u8;
            }
        }
        fn gen_len(&mut self, max: usize) -> usize {
            if max == 0 {
                return 0;
            }
            (self.next_u32() as usize) % (max + 1)
        }
    }

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

    #[test]
    fn seeded_pseudo_random_envelopes_and_metadata_do_not_panic() {
        let mut rng = Lcg(0x4F_50_45_4E_53_54_45_47); // "OPENSTEG"
        let mut buf = vec![0u8; 512];
        for _ in 0..400 {
            let n = rng.gen_len(buf.len());
            rng.fill(&mut buf[..n]);
            let slice = &buf[..n];
            let _ = unwrap_payload(slice);
            let _ = parse_envelope_header(slice);
        }
        // Mutate valid envelopes / metadata; avoid Argon2 on every random blob.
        let good_env = encrypt_blob(b"payload-seed", "fuzz-pw").unwrap();
        let meta = PayloadMeta::for_path_file("seed.bin", b"payload-seed");
        let good_meta = wrap_payload(&meta, b"payload-seed").unwrap();
        for i in 0..200 {
            let mut env = good_env.clone();
            let mut meta_blob = good_meta.clone();
            let idx = (rng.next_u32() as usize) % env.len().max(1);
            if !env.is_empty() {
                env[idx] ^= (i as u8) | 1;
            }
            let j = (rng.next_u32() as usize) % meta_blob.len().max(1);
            if !meta_blob.is_empty() {
                meta_blob[j] ^= 0xa5;
            }
            let _ = parse_envelope_header(&env);
            let _ = unwrap_payload(&meta_blob);
        }
        // A handful of decrypt attempts on mutated envelopes (Argon2 is expensive).
        for i in 0..4 {
            let mut env = good_env.clone();
            let idx = i % env.len();
            env[idx] ^= 0x3c;
            let _ = decrypt_blob(&env, "fuzz-pw");
        }
    }
}
