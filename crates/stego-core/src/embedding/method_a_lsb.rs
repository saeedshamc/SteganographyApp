//! Method A — keyed random LSB for PNG/BMP covers.
//!
//! Bit layout in RGB channels (alpha unused):
//! 1. First 128 LSBs (sequential) store the 16-byte salt from the crypto envelope
//! 2. Remaining bit slots are shuffled with ChaCha20Rng seeded by LocatorKey
//! 3. Shuffled stream holds `envelope_len (u64 LE)` followed by the encrypted envelope
//!
//! Output is always PNG. BMP input is accepted and re-encoded as PNG.

use image::{DynamicImage, ImageFormat, RgbaImage};
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use crate::crypto::{self, SALT_LEN};
use crate::error::{StegoError, StegoResult};

const LEN_BYTES: usize = 8;

/// Maximum payload (encrypted envelope) size in bytes for RGB LSB embedding.
pub fn capacity_bytes(width: u32, height: u32) -> StegoResult<usize> {
    let total_bits = (width as usize)
        .checked_mul(height as usize)
        .and_then(|p| p.checked_mul(3))
        .ok_or_else(|| StegoError::Message("image dimensions overflow".into()))?;
    if total_bits <= SALT_LEN * 8 + LEN_BYTES * 8 {
        return Ok(0);
    }
    let payload_bits = total_bits - SALT_LEN * 8;
    // reserve 8 bytes for length prefix inside shuffled region
    Ok(payload_bits / 8 - LEN_BYTES)
}

fn load_rgba(cover: &[u8]) -> StegoResult<RgbaImage> {
    let img = image::load_from_memory(cover)
        .map_err(|e| StegoError::UnsupportedCover(format!("cannot decode image: {e}")))?;
    Ok(img.to_rgba8())
}

fn bit_slots(width: u32, height: u32) -> Vec<(u32, u32, usize)> {
    // (x, y, channel 0=R 1=G 2=B)
    let mut slots = Vec::with_capacity((width * height * 3) as usize);
    for y in 0..height {
        for x in 0..width {
            for c in 0..3usize {
                slots.push((x, y, c));
            }
        }
    }
    slots
}

fn set_lsb(img: &mut RgbaImage, x: u32, y: u32, channel: usize, bit: u8) {
    let px = img.get_pixel_mut(x, y);
    px.0[channel] = (px.0[channel] & 0xFE) | (bit & 1);
}

fn get_lsb(img: &RgbaImage, x: u32, y: u32, channel: usize) -> u8 {
    img.get_pixel(x, y).0[channel] & 1
}

fn write_bits_to_slots(
    img: &mut RgbaImage,
    slots: &[(u32, u32, usize)],
    data: &[u8],
) -> StegoResult<()> {
    let need = data.len() * 8;
    if need > slots.len() {
        return Err(StegoError::CapacityExceeded {
            need: data.len(),
            capacity: slots.len() / 8,
        });
    }
    for (i, byte) in data.iter().enumerate() {
        for b in 0..8 {
            let bit = (byte >> b) & 1;
            let (x, y, c) = slots[i * 8 + b];
            set_lsb(img, x, y, c, bit);
        }
    }
    Ok(())
}

fn read_bits_from_slots(img: &RgbaImage, slots: &[(u32, u32, usize)], nbytes: usize) -> Vec<u8> {
    let mut out = vec![0u8; nbytes];
    for i in 0..nbytes {
        let mut byte = 0u8;
        for b in 0..8 {
            let (x, y, c) = slots[i * 8 + b];
            byte |= get_lsb(img, x, y, c) << b;
        }
        out[i] = byte;
    }
    out
}

fn shuffled_payload_slots(
    all: &[(u32, u32, usize)],
    locator_key: &[u8; 32],
) -> Vec<(u32, u32, usize)> {
    let salt_slots = SALT_LEN * 8;
    let mut rest: Vec<_> = all[salt_slots..].to_vec();
    let mut seed = [0u8; 32];
    seed.copy_from_slice(locator_key);
    let mut rng = ChaCha20Rng::from_seed(seed);
    rest.shuffle(&mut rng);
    rest
}

fn encode_png(img: &RgbaImage) -> StegoResult<Vec<u8>> {
    let mut buf = Vec::new();
    let dynimg = DynamicImage::ImageRgba8(img.clone());
    dynimg
        .write_to(&mut std::io::Cursor::new(&mut buf), ImageFormat::Png)
        .map_err(|e| StegoError::Message(format!("png encode failed: {e}")))?;
    Ok(buf)
}

/// Embed ciphertext envelope into PNG/BMP; returns PNG bytes.
pub fn embed(cover_png_or_bmp: &[u8], ciphertext: &[u8], password: &str) -> StegoResult<Vec<u8>> {
    if ciphertext.len() < 1 + SALT_LEN {
        return Err(StegoError::InvalidFormat("ciphertext too short".into()));
    }
    let mut img = load_rgba(cover_png_or_bmp)?;
    let (w, h) = img.dimensions();
    let cap = capacity_bytes(w, h)?;
    if ciphertext.len() > cap {
        return Err(StegoError::CapacityExceeded {
            need: ciphertext.len(),
            capacity: cap,
        });
    }

    let salt: [u8; SALT_LEN] = ciphertext[1..1 + SALT_LEN].try_into().unwrap();
    let keys = crypto::derive_keys(password, &salt)?;
    let all = bit_slots(w, h);
    let salt_slots: Vec<_> = all[..SALT_LEN * 8].to_vec();
    write_bits_to_slots(&mut img, &salt_slots, &salt)?;

    let payload_slots = shuffled_payload_slots(&all, &keys.locator_key);
    let mut stream = Vec::with_capacity(LEN_BYTES + ciphertext.len());
    stream.extend_from_slice(&(ciphertext.len() as u64).to_le_bytes());
    stream.extend_from_slice(ciphertext);
    write_bits_to_slots(&mut img, &payload_slots, &stream)?;

    encode_png(&img)
}

/// Extract ciphertext envelope from an LSB stego PNG.
pub fn extract(stego_png: &[u8], password: &str) -> StegoResult<Vec<u8>> {
    let img = load_rgba(stego_png)?;
    let (w, h) = img.dimensions();
    let all = bit_slots(w, h);
    if all.len() < SALT_LEN * 8 + LEN_BYTES * 8 {
        return Err(StegoError::InvalidFormat("image too small for LSB".into()));
    }

    let salt_slots: Vec<_> = all[..SALT_LEN * 8].to_vec();
    let salt_vec = read_bits_from_slots(&img, &salt_slots, SALT_LEN);
    let salt: [u8; SALT_LEN] = salt_vec.try_into().unwrap();

    let keys = crypto::derive_keys(password, &salt)?;
    let payload_slots = shuffled_payload_slots(&all, &keys.locator_key);
    let len_bytes = read_bits_from_slots(&img, &payload_slots[..LEN_BYTES * 8], LEN_BYTES);
    let env_len = u64::from_le_bytes(len_bytes.try_into().unwrap()) as usize;

    let cap = capacity_bytes(w, h)?;
    if env_len == 0 || env_len > cap {
        return Err(StegoError::WrongPassword);
    }

    let need_slots = (LEN_BYTES + env_len) * 8;
    if need_slots > payload_slots.len() {
        return Err(StegoError::WrongPassword);
    }
    let body_slots = &payload_slots[LEN_BYTES * 8..need_slots];
    let envelope = read_bits_from_slots(&img, body_slots, env_len);

    if envelope.len() < 1 + SALT_LEN || envelope[1..1 + SALT_LEN] != salt {
        return Err(StegoError::WrongPassword);
    }
    Ok(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{decrypt_blob, encrypt_blob};
    use image::{ImageBuffer, Rgba};

    fn tiny_png(w: u32, h: u32) -> Vec<u8> {
        let img: RgbaImage =
            ImageBuffer::from_fn(w, h, |x, y| Rgba([x as u8, y as u8, 128, 255]));
        encode_png(&img).unwrap()
    }

    #[test]
    fn capacity_grows_with_dimensions() {
        let a = capacity_bytes(64, 64).unwrap();
        let b = capacity_bytes(128, 128).unwrap();
        assert!(b > a);
        assert!(a > 0);
    }

    #[test]
    fn round_trip_png() {
        let cover = tiny_png(128, 128);
        let pw = "lsb-secret";
        let msg = b"hidden-in-pixels";
        let ct = encrypt_blob(msg, pw).unwrap();
        assert!(ct.len() <= capacity_bytes(128, 128).unwrap());
        let stego = embed(&cover, &ct, pw).unwrap();
        // PNG signature
        assert_eq!(&stego[..8], b"\x89PNG\r\n\x1a\n");
        let extracted = extract(&stego, pw).unwrap();
        assert_eq!(extracted, ct);
        assert_eq!(decrypt_blob(&extracted, pw).unwrap(), msg);
    }

    #[test]
    fn wrong_password_fails() {
        let cover = tiny_png(96, 96);
        let ct = encrypt_blob(b"x", "right").unwrap();
        let stego = embed(&cover, &ct, "right").unwrap();
        assert_eq!(
            extract(&stego, "wrong").unwrap_err(),
            StegoError::WrongPassword
        );
    }

    #[test]
    fn capacity_exceeded() {
        let cover = tiny_png(16, 16);
        let cap = capacity_bytes(16, 16).unwrap();
        let huge = vec![1u8; cap + 50];
        let err = embed(&cover, &huge, "pw").unwrap_err();
        match err {
            StegoError::CapacityExceeded { .. } => {}
            other => panic!("{other:?}"),
        }
    }
}
