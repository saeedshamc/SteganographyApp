//! Method A — keyed random LSB for PNG/BMP covers.
//!
//! Bit layout in RGB channels (alpha unused):
//! 1. First `SALT_LEN * 8` bit-slots (sequential) store the 16-byte salt
//! 2. Remaining bit-slots are shuffled with ChaCha20Rng(LocatorKey)
//! 3. Shuffled stream holds `envelope_len (u64 LE)` followed by the encrypted envelope
//!
//! Each RGB channel contributes `depth` low bits (1 or 2). Output is always PNG.

use image::{DynamicImage, ImageFormat, RgbaImage};
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use crate::crypto::{self, SALT_LEN};
use crate::error::{StegoError, StegoResult};

const LEN_BYTES: usize = 8;

/// One bit position: pixel (x,y), channel 0..2, bit index 0..(depth-1).
type BitSlot = (u32, u32, usize, u8);

/// Maximum encrypted-envelope size for given dimensions (depth 1).
pub fn capacity_bytes(width: u32, height: u32) -> StegoResult<usize> {
    capacity_bytes_depth(width, height, 1)
}

pub fn capacity_bytes_depth(width: u32, height: u32, depth: u8) -> StegoResult<usize> {
    let depth = normalize_depth(depth)?;
    let total_bits = (width as usize)
        .checked_mul(height as usize)
        .and_then(|p| p.checked_mul(3))
        .and_then(|p| p.checked_mul(depth as usize))
        .ok_or_else(|| StegoError::Message("image dimensions overflow".into()))?;
    let salt_bits = SALT_LEN * 8;
    if total_bits <= salt_bits + LEN_BYTES * 8 {
        return Ok(0);
    }
    Ok((total_bits - salt_bits) / 8 - LEN_BYTES)
}

fn normalize_depth(depth: u8) -> StegoResult<u8> {
    match depth {
        0 | 1 => Ok(1),
        2 => Ok(2),
        d => Err(StegoError::Message(format!(
            "lsb depth must be 1 or 2 (got {d})"
        ))),
    }
}

fn load_rgba(cover: &[u8]) -> StegoResult<RgbaImage> {
    let img = image::load_from_memory(cover)
        .map_err(|e| StegoError::UnsupportedCover(format!("cannot decode image: {e}")))?;
    Ok(img.to_rgba8())
}

fn bit_slots(width: u32, height: u32, depth: u8) -> Vec<BitSlot> {
    let mut slots = Vec::with_capacity((width * height * 3 * depth as u32) as usize);
    for y in 0..height {
        for x in 0..width {
            for c in 0..3usize {
                for b in 0..depth {
                    slots.push((x, y, c, b));
                }
            }
        }
    }
    slots
}

fn set_bit(img: &mut RgbaImage, x: u32, y: u32, channel: usize, bit_i: u8, bit: u8) {
    let px = img.get_pixel_mut(x, y);
    let mask = !(1u8 << bit_i);
    px.0[channel] = (px.0[channel] & mask) | ((bit & 1) << bit_i);
}

fn get_bit(img: &RgbaImage, x: u32, y: u32, channel: usize, bit_i: u8) -> u8 {
    (img.get_pixel(x, y).0[channel] >> bit_i) & 1
}

fn write_bits_to_slots(img: &mut RgbaImage, slots: &[BitSlot], data: &[u8]) -> StegoResult<()> {
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
            let (x, y, c, bi) = slots[i * 8 + b];
            set_bit(img, x, y, c, bi, bit);
        }
    }
    Ok(())
}

fn read_bits_from_slots(img: &RgbaImage, slots: &[BitSlot], nbytes: usize) -> Vec<u8> {
    let mut out = vec![0u8; nbytes];
    for i in 0..nbytes {
        let mut byte = 0u8;
        for b in 0..8 {
            let (x, y, c, bi) = slots[i * 8 + b];
            byte |= get_bit(img, x, y, c, bi) << b;
        }
        out[i] = byte;
    }
    out
}

fn shuffled_payload_slots(all: &[BitSlot], locator_key: &[u8; 32]) -> Vec<BitSlot> {
    let salt_slots = SALT_LEN * 8;
    let mut rest: Vec<_> = all[salt_slots..].to_vec();
    let mut seed = [0u8; 32];
    seed.copy_from_slice(locator_key);
    let mut rng = ChaCha20Rng::from_seed(seed);
    rest.shuffle(&mut rng);
    rest
}

fn adaptive_payload_slots(
    img: &RgbaImage,
    all: &[BitSlot],
    locator_key: &[u8; 32],
) -> Vec<BitSlot> {
    let salt_slots = SALT_LEN * 8;
    let rest = &all[salt_slots..];
    let mut scored: Vec<(u32, BitSlot)> = rest
        .iter()
        .copied()
        .map(|(x, y, c, bi)| (local_variance(img, x, y, c), (x, y, c, bi)))
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    let keep = (scored.len() / 2).max(LEN_BYTES * 8 + 256).min(scored.len());
    let mut pool: Vec<_> = scored.into_iter().take(keep).map(|(_, s)| s).collect();
    let mut seed = [0u8; 32];
    seed.copy_from_slice(locator_key);
    seed[0] ^= 0xa5;
    let mut rng = ChaCha20Rng::from_seed(seed);
    pool.shuffle(&mut rng);
    pool
}

fn local_variance(img: &RgbaImage, x: u32, y: u32, channel: usize) -> u32 {
    let (w, h) = img.dimensions();
    let center = (img.get_pixel(x, y).0[channel] >> 1) as i32;
    let mut acc = 0u32;
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                continue;
            }
            let v = (img.get_pixel(nx as u32, ny as u32).0[channel] >> 1) as i32;
            acc += (v - center).unsigned_abs();
        }
    }
    acc
}

fn encode_png(img: &RgbaImage) -> StegoResult<Vec<u8>> {
    let mut buf = Vec::new();
    let dynimg = DynamicImage::ImageRgba8(img.clone());
    dynimg
        .write_to(&mut std::io::Cursor::new(&mut buf), ImageFormat::Png)
        .map_err(|e| StegoError::Message(format!("png encode failed: {e}")))?;
    Ok(buf)
}

pub fn embed(cover_png_or_bmp: &[u8], ciphertext: &[u8], password: &str) -> StegoResult<Vec<u8>> {
    embed_ex(
        cover_png_or_bmp,
        ciphertext,
        password,
        &crypto::CryptoOptions::default(),
        1,
        false,
    )
}

pub fn embed_with(
    cover_png_or_bmp: &[u8],
    ciphertext: &[u8],
    password: &str,
    opts: &crypto::CryptoOptions,
) -> StegoResult<Vec<u8>> {
    embed_ex(cover_png_or_bmp, ciphertext, password, opts, 1, false)
}

/// Full Method A embed with depth and optional adaptive slot selection.
pub fn embed_ex(
    cover_png_or_bmp: &[u8],
    ciphertext: &[u8],
    password: &str,
    opts: &crypto::CryptoOptions,
    depth: u8,
    adaptive: bool,
) -> StegoResult<Vec<u8>> {
    let depth = normalize_depth(depth)?;
    let salt = crypto::salt_from_envelope(ciphertext)?;
    let mut img = load_rgba(cover_png_or_bmp)?;
    let (w, h) = img.dimensions();
    let mut cap = capacity_bytes_depth(w, h, depth)?;
    if adaptive {
        cap /= 2;
    }
    if ciphertext.len() > cap {
        return Err(StegoError::CapacityExceeded {
            need: ciphertext.len(),
            capacity: cap,
        });
    }

    let keys = crypto::derive_keys_with(password, &salt, opts)?;
    let all = bit_slots(w, h, depth);
    let salt_slots: Vec<_> = all[..SALT_LEN * 8].to_vec();
    write_bits_to_slots(&mut img, &salt_slots, &salt)?;

    let payload_slots = if adaptive {
        adaptive_payload_slots(&img, &all, &keys.locator_key)
    } else {
        shuffled_payload_slots(&all, &keys.locator_key)
    };
    let mut stream = Vec::with_capacity(LEN_BYTES + ciphertext.len());
    stream.extend_from_slice(&(ciphertext.len() as u64).to_le_bytes());
    stream.extend_from_slice(ciphertext);
    write_bits_to_slots(&mut img, &payload_slots, &stream)?;

    encode_png(&img)
}

pub fn extract(stego_png: &[u8], password: &str) -> StegoResult<Vec<u8>> {
    extract_ex(
        stego_png,
        password,
        &crypto::CryptoOptions::default(),
        1,
        false,
    )
}

pub fn extract_with(
    stego_png: &[u8],
    password: &str,
    opts: &crypto::CryptoOptions,
) -> StegoResult<Vec<u8>> {
    extract_ex(stego_png, password, opts, 1, false)
}

pub fn extract_ex(
    stego_png: &[u8],
    password: &str,
    opts: &crypto::CryptoOptions,
    preferred_depth: u8,
    adaptive: bool,
) -> StegoResult<Vec<u8>> {
    let preferred = normalize_depth(preferred_depth).unwrap_or(1);
    let mut depths = vec![preferred];
    for d in [1u8, 2] {
        if !depths.contains(&d) {
            depths.push(d);
        }
    }
    let mut last = StegoError::WrongPassword;
    for depth in depths {
        match try_extract_depth(stego_png, password, opts, depth, adaptive).or_else(|_| {
            try_extract_depth(stego_png, password, opts, depth, !adaptive)
        }) {
            Ok(v) => return Ok(v),
            Err(e) => last = e,
        }
    }
    Err(last)
}

fn try_extract_depth(
    stego_png: &[u8],
    password: &str,
    opts: &crypto::CryptoOptions,
    depth: u8,
    adaptive: bool,
) -> StegoResult<Vec<u8>> {
    let img = load_rgba(stego_png)?;
    let (w, h) = img.dimensions();
    let all = bit_slots(w, h, depth);
    if all.len() < SALT_LEN * 8 + LEN_BYTES * 8 {
        return Err(StegoError::InvalidFormat("image too small for LSB".into()));
    }
    let salt_slots: Vec<_> = all[..SALT_LEN * 8].to_vec();
    let salt_vec = read_bits_from_slots(&img, &salt_slots, SALT_LEN);
    let salt: [u8; SALT_LEN] = salt_vec.try_into().unwrap();

    let mut profiles = vec![opts.profile];
    for p in crypto::KdfProfile::all() {
        if !profiles.contains(&p) {
            profiles.push(p);
        }
    }
    let mut last_err = StegoError::WrongPassword;
    for profile in profiles {
        let try_opts = crypto::CryptoOptions {
            profile,
            keyfile: opts.keyfile.clone(),
        };
        match try_extract_lsb(&img, &all, &salt, password, &try_opts, depth, adaptive) {
            Ok(env) => return Ok(env),
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

fn try_extract_lsb(
    img: &RgbaImage,
    all: &[BitSlot],
    salt: &[u8; SALT_LEN],
    password: &str,
    opts: &crypto::CryptoOptions,
    depth: u8,
    adaptive: bool,
) -> StegoResult<Vec<u8>> {
    let keys = crypto::derive_keys_with(password, salt, opts)?;
    let payload_slots = if adaptive {
        adaptive_payload_slots(img, all, &keys.locator_key)
    } else {
        shuffled_payload_slots(all, &keys.locator_key)
    };
    if payload_slots.len() < LEN_BYTES * 8 {
        return Err(StegoError::WrongPassword);
    }
    let len_bytes = read_bits_from_slots(img, &payload_slots[..LEN_BYTES * 8], LEN_BYTES);
    let env_len = u64::from_le_bytes(len_bytes.try_into().unwrap()) as usize;

    let (w, h) = img.dimensions();
    let mut cap = capacity_bytes_depth(w, h, depth)?;
    if adaptive {
        cap /= 2;
    }
    if env_len == 0 || env_len > cap {
        return Err(StegoError::WrongPassword);
    }

    let need_slots = (LEN_BYTES + env_len) * 8;
    if need_slots > payload_slots.len() {
        return Err(StegoError::WrongPassword);
    }
    let body_slots = &payload_slots[LEN_BYTES * 8..need_slots];
    let envelope = read_bits_from_slots(img, body_slots, env_len);

    let header = crypto::parse_envelope_header(&envelope).map_err(|_| StegoError::WrongPassword)?;
    if &header.salt != salt {
        return Err(StegoError::WrongPassword);
    }
    Ok(envelope)
}

pub fn embed_adaptive(
    cover_png_or_bmp: &[u8],
    ciphertext: &[u8],
    password: &str,
    opts: &crypto::CryptoOptions,
) -> StegoResult<Vec<u8>> {
    embed_ex(cover_png_or_bmp, ciphertext, password, opts, 1, true)
}

pub fn extract_adaptive(
    stego_png: &[u8],
    password: &str,
    opts: &crypto::CryptoOptions,
) -> StegoResult<Vec<u8>> {
    extract_ex(stego_png, password, opts, 1, true)
}

/// Build a small PNG data-URL preview (max edge `max_edge`).
pub fn preview_data_url(image_bytes: &[u8], max_edge: u32) -> StegoResult<String> {
    let img = image::load_from_memory(image_bytes)
        .map_err(|e| StegoError::UnsupportedCover(format!("cannot decode: {e}")))?;
    let thumb = img.thumbnail(max_edge, max_edge);
    let mut png = Vec::new();
    thumb
        .write_to(&mut std::io::Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|e| StegoError::Message(format!("png encode: {e}")))?;
    Ok(format!("data:image/png;base64,{}", base64_encode(&png)))
}

/// Educational LSB-diff visualization between two images (same size preferred).
pub fn lsb_diff_data_url(before: &[u8], after: &[u8], max_edge: u32) -> StegoResult<String> {
    let a = load_rgba(before)?;
    let b = load_rgba(after)?;
    let (w, h) = a.dimensions();
    let (w2, h2) = b.dimensions();
    if w != w2 || h != h2 {
        return Err(StegoError::Message(
            "images must match size for LSB diff".into(),
        ));
    }
    let mut out = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let pa = a.get_pixel(x, y);
            let pb = b.get_pixel(x, y);
            let mut changed = false;
            for c in 0..3 {
                if (pa.0[c] & 1) != (pb.0[c] & 1) {
                    changed = true;
                    break;
                }
            }
            let px = if changed {
                image::Rgba([255, 40, 40, 255])
            } else {
                let g = pa.0[0] / 3 + pa.0[1] / 3 + pa.0[2] / 3;
                image::Rgba([g, g, g, 255])
            };
            out.put_pixel(x, y, px);
        }
    }
    let dynimg = DynamicImage::ImageRgba8(out).thumbnail(max_edge, max_edge);
    let mut png = Vec::new();
    dynimg
        .write_to(&mut std::io::Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|e| StegoError::Message(format!("png encode: {e}")))?;
    Ok(format!("data:image/png;base64,{}", base64_encode(&png)))
}

fn base64_encode(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let mut n = (chunk[0] as u32) << 16;
        if chunk.len() > 1 {
            n |= (chunk[1] as u32) << 8;
        }
        if chunk.len() > 2 {
            n |= chunk[2] as u32;
        }
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(T[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(T[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{decrypt_blob, encrypt_blob};
    use image::{ImageBuffer, Rgba};

    fn tiny_png(w: u32, h: u32) -> Vec<u8> {
        let img: RgbaImage = ImageBuffer::from_fn(w, h, |x, y| Rgba([x as u8, y as u8, 128, 255]));
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
    fn depth2_has_more_capacity() {
        let d1 = capacity_bytes_depth(64, 64, 1).unwrap();
        let d2 = capacity_bytes_depth(64, 64, 2).unwrap();
        assert!(d2 > d1);
    }

    #[test]
    fn round_trip_png() {
        let cover = tiny_png(128, 128);
        let pw = "lsb-secret";
        let msg = b"hidden-in-pixels";
        let ct = encrypt_blob(msg, pw).unwrap();
        assert!(ct.len() <= capacity_bytes(128, 128).unwrap());
        let stego = embed(&cover, &ct, pw).unwrap();
        assert_eq!(&stego[..8], b"\x89PNG\r\n\x1a\n");
        let extracted = extract(&stego, pw).unwrap();
        assert_eq!(extracted, ct);
        assert_eq!(decrypt_blob(&extracted, pw).unwrap(), msg);
    }

    #[test]
    fn round_trip_depth2() {
        let cover = tiny_png(96, 96);
        let pw = "d2";
        let msg = b"depth-two";
        let ct = encrypt_blob(msg, pw).unwrap();
        let opts = crypto::CryptoOptions::default();
        let stego = embed_ex(&cover, &ct, pw, &opts, 2, false).unwrap();
        let got = extract_ex(&stego, pw, &opts, 2, false).unwrap();
        assert_eq!(got, ct);
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
