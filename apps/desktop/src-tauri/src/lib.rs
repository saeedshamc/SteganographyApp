//! Tauri desktop shell for Open Stego.

use serde::Serialize;
use stego_core::embedding::method_a_lsb;
use stego_core::{
    extract_with, hide_with, plan_hide_with, parse_kdf_profile, CryptoOptions,
    PayloadMeta, StegoOptions, VERSION,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanDto {
    method: String,
    capacity: Option<usize>,
    jpeg_warning: Option<String>,
    eof_caveat: Option<String>,
    capacity_risk: Option<String>,
    cover_path: String,
    cover_size: usize,
    preview_url: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HideResultDto {
    output_path: String,
    output_size: usize,
    extension: String,
    preview_url: Option<String>,
    diff_url: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExtractResultDto {
    is_text: bool,
    kind: String,
    filename: Option<String>,
    method: String,
    size: usize,
    checksum_hex: String,
    text_preview: Option<String>,
    saved_path: Option<String>,
    reveal_preview_url: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BatchItemDto {
    cover_path: String,
    ok: bool,
    message: String,
    output_path: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BatchResultDto {
    items: Vec<BatchItemDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PayloadInfoDto {
    path: String,
    size: usize,
    kind: String,
}

fn checksum_hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn is_image_path(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| matches!(e.to_ascii_lowercase().as_str(), "png" | "bmp" | "jpg" | "jpeg" | "gif" | "webp"))
        .unwrap_or(false)
}

fn maybe_preview(bytes: &[u8], path: &str) -> Option<String> {
    if !is_image_path(path) && image::load_from_memory(bytes).is_err() {
        return None;
    }
    method_a_lsb::preview_data_url(bytes, 320).ok()
}

#[tauri::command]
fn app_version() -> String {
    VERSION.to_string()
}

#[tauri::command]
fn password_strength(password: String) -> String {
    let len = password.len();
    let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
    let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_other = password.chars().any(|c| !c.is_ascii_alphanumeric());
    let classes = [has_lower, has_upper, has_digit, has_other]
        .iter()
        .filter(|&&x| x)
        .count();
    if len == 0 {
        "empty".into()
    } else if len < 8 || classes < 2 {
        "weak".into()
    } else if len < 12 || classes < 3 {
        "fair".into()
    } else {
        "strong".into()
    }
}

#[tauri::command]
fn pick_cover() -> Result<PlanDto, String> {
    let path = rfd::FileDialog::new()
        .set_title("Choose cover file")
        .pick_file()
        .ok_or_else(|| "cancelled".to_string())?;
    inspect_cover(path, None, 1, false)
}

#[tauri::command]
fn inspect_cover_path(
    path: String,
    payload_len: Option<usize>,
    lsb_depth: Option<u8>,
    adaptive_lsb: Option<bool>,
) -> Result<PlanDto, String> {
    inspect_cover(
        PathBuf::from(path),
        payload_len,
        lsb_depth.unwrap_or(1),
        adaptive_lsb.unwrap_or(false),
    )
}

fn inspect_cover(
    path: PathBuf,
    payload_len: Option<usize>,
    lsb_depth: u8,
    adaptive_lsb: bool,
) -> Result<PlanDto, String> {
    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    let path_str = path.to_string_lossy().to_string();
    let opts = StegoOptions {
        crypto: CryptoOptions::default(),
        adaptive_lsb,
        lsb_depth,
    };
    let plan = plan_hide_with(&bytes, &path_str, &opts, payload_len).map_err(|e| e.to_string())?;
    Ok(PlanDto {
        method: plan.method.as_str().to_string(),
        capacity: plan.capacity,
        jpeg_warning: plan.jpeg_warning,
        eof_caveat: plan.eof_caveat,
        capacity_risk: plan.capacity_risk,
        cover_path: path_str.clone(),
        cover_size: bytes.len(),
        preview_url: maybe_preview(&bytes, &path_str),
    })
}

#[tauri::command]
fn pick_payload_file() -> Result<PayloadInfoDto, String> {
    let path = rfd::FileDialog::new()
        .set_title("Choose payload file to hide")
        .pick_file()
        .ok_or_else(|| "cancelled".to_string())?;
    payload_info(path)
}

#[tauri::command]
fn set_payload_path(path: String) -> Result<PayloadInfoDto, String> {
    payload_info(PathBuf::from(path))
}

fn payload_info(path: PathBuf) -> Result<PayloadInfoDto, String> {
    let meta = fs::metadata(&path).map_err(|e| e.to_string())?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("payload.bin");
    let kind = PayloadMeta::for_path_file(name, &[])
        .kind()
        .as_str()
        .to_string();
    Ok(PayloadInfoDto {
        path: path.to_string_lossy().to_string(),
        size: meta.len() as usize,
        kind,
    })
}

#[tauri::command]
fn pick_keyfile() -> Result<(String, usize), String> {
    let path = rfd::FileDialog::new()
        .set_title("Choose keyfile")
        .pick_file()
        .ok_or_else(|| "cancelled".to_string())?;
    let meta = fs::metadata(&path).map_err(|e| e.to_string())?;
    Ok((path.to_string_lossy().to_string(), meta.len() as usize))
}

fn build_opts(
    profile: String,
    keyfile_path: Option<String>,
    adaptive_lsb: bool,
    lsb_depth: u8,
) -> Result<StegoOptions, String> {
    let profile = parse_kdf_profile(&profile).map_err(|e| e.to_string())?;
    let keyfile = match keyfile_path {
        None => None,
        Some(p) if p.is_empty() => None,
        Some(p) => Some(fs::read(&p).map_err(|e| e.to_string())?),
    };
    Ok(StegoOptions {
        crypto: CryptoOptions { profile, keyfile },
        adaptive_lsb,
        lsb_depth,
    })
}

#[tauri::command]
fn hide_payload(
    cover_path: String,
    payload_path: Option<String>,
    text_payload: Option<String>,
    password: String,
    verify: bool,
    profile: String,
    keyfile_path: Option<String>,
    adaptive_lsb: bool,
    lsb_depth: u8,
    output_path: Option<String>,
) -> Result<HideResultDto, String> {
    if password.is_empty() {
        return Err("password is required".into());
    }
    let cover = fs::read(&cover_path).map_err(|e| e.to_string())?;
    let opts = build_opts(profile, keyfile_path, adaptive_lsb, lsb_depth)?;

    let (meta, payload_bytes) = if let Some(text) = text_payload {
        let data = text.into_bytes();
        (PayloadMeta::for_text(&data), data)
    } else if let Some(p) = payload_path {
        let data = fs::read(&p).map_err(|e| e.to_string())?;
        let name = std::path::Path::new(&p)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("payload.bin")
            .to_string();
        (PayloadMeta::for_path_file(name, &data), data)
    } else {
        return Err("provide a payload file or text".into());
    };

    let (stego, ext) =
        hide_with(&cover, &cover_path, &meta, &payload_bytes, &password, &opts)
            .map_err(|e| e.to_string())?;

    if verify {
        let check = extract_with(&stego, &format!("verify.{ext}"), &password, &opts)
            .map_err(|e| format!("round-trip verify failed: {e}"))?;
        if check.data != payload_bytes {
            return Err("round-trip verify failed: payload mismatch".into());
        }
    }

    let out = if let Some(p) = output_path.filter(|s| !s.is_empty()) {
        PathBuf::from(p)
    } else {
        let default_name = {
            let stem = std::path::Path::new(&cover_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("stego");
            format!("{stem}_stego.{ext}")
        };
        rfd::FileDialog::new()
            .set_title("Save stego output")
            .set_file_name(&default_name)
            .save_file()
            .ok_or_else(|| "cancelled".to_string())?
    };

    fs::write(&out, &stego).map_err(|e| e.to_string())?;
    let out_str = out.to_string_lossy().to_string();
    let preview_url = maybe_preview(&stego, &out_str);
    let diff_url = method_a_lsb::lsb_diff_data_url(&cover, &stego, 320).ok();
    Ok(HideResultDto {
        output_path: out_str,
        output_size: stego.len(),
        extension: ext,
        preview_url,
        diff_url,
    })
}

#[tauri::command]
fn batch_hide(
    cover_paths: Vec<String>,
    payload_path: Option<String>,
    text_payload: Option<String>,
    password: String,
    profile: String,
    keyfile_path: Option<String>,
    adaptive_lsb: bool,
    lsb_depth: u8,
    output_dir: String,
) -> Result<BatchResultDto, String> {
    if password.is_empty() {
        return Err("password is required".into());
    }
    if cover_paths.is_empty() {
        return Err("add at least one cover".into());
    }
    let opts = build_opts(profile, keyfile_path, adaptive_lsb, lsb_depth)?;
    let (meta, payload_bytes) = if let Some(text) = text_payload {
        let data = text.into_bytes();
        (PayloadMeta::for_text(&data), data)
    } else if let Some(p) = payload_path {
        let data = fs::read(&p).map_err(|e| e.to_string())?;
        let name = std::path::Path::new(&p)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("payload.bin")
            .to_string();
        (PayloadMeta::for_path_file(name, &data), data)
    } else {
        return Err("provide a payload file or text".into());
    };

    fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for cover_path in cover_paths {
        let result = (|| -> Result<(String, String), String> {
            let cover = fs::read(&cover_path).map_err(|e| e.to_string())?;
            let (stego, ext) =
                hide_with(&cover, &cover_path, &meta, &payload_bytes, &password, &opts)
                    .map_err(|e| e.to_string())?;
            let stem = Path::new(&cover_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("stego");
            let out = Path::new(&output_dir).join(format!("{stem}_stego.{ext}"));
            fs::write(&out, &stego).map_err(|e| e.to_string())?;
            Ok((out.to_string_lossy().to_string(), ext))
        })();
        match result {
            Ok((out, ext)) => items.push(BatchItemDto {
                cover_path,
                ok: true,
                message: format!("saved .{ext}"),
                output_path: Some(out),
            }),
            Err(e) => items.push(BatchItemDto {
                cover_path,
                ok: false,
                message: e,
                output_path: None,
            }),
        }
    }
    Ok(BatchResultDto { items })
}

#[tauri::command]
fn pick_output_dir() -> Result<String, String> {
    rfd::FileDialog::new()
        .set_title("Choose batch output folder")
        .pick_folder()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "cancelled".to_string())
}

#[tauri::command]
fn pick_stego_file() -> Result<(String, usize, Option<String>), String> {
    let path = rfd::FileDialog::new()
        .set_title("Choose file that may contain hidden data")
        .pick_file()
        .ok_or_else(|| "cancelled".to_string())?;
    set_stego_path(path.to_string_lossy().to_string())
}

#[tauri::command]
fn set_stego_path(path: String) -> Result<(String, usize, Option<String>), String> {
    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    let preview = maybe_preview(&bytes, &path);
    Ok((path, bytes.len(), preview))
}

#[tauri::command]
fn extract_payload(
    stego_path: String,
    password: String,
    keyfile_path: Option<String>,
    adaptive_lsb: bool,
    lsb_depth: u8,
) -> Result<ExtractResultDto, String> {
    if password.is_empty() {
        return Err("password is required".into());
    }
    let stego = fs::read(&stego_path).map_err(|e| e.to_string())?;
    let opts = build_opts("balanced".into(), keyfile_path, adaptive_lsb, lsb_depth)?;
    let recovered =
        extract_with(&stego, &stego_path, &password, &opts).map_err(|e| e.to_string())?;
    let kind = recovered.meta.kind().as_str().to_string();
    let checksum = checksum_hex(&recovered.meta.checksum_sha256);
    let reveal_preview_url = maybe_preview(&stego, &stego_path);

    if recovered.meta.is_text {
        let text = String::from_utf8(recovered.data).map_err(|_| {
            "hidden payload was marked as text but is not valid UTF-8".to_string()
        })?;
        return Ok(ExtractResultDto {
            is_text: true,
            kind,
            filename: None,
            method: recovered.method.as_str().to_string(),
            size: text.len(),
            checksum_hex: checksum,
            text_preview: Some(text),
            saved_path: None,
            reveal_preview_url,
        });
    }

    let suggested = recovered
        .meta
        .filename
        .clone()
        .unwrap_or_else(|| "recovered.bin".into());
    let title = if recovered.meta.is_executable {
        "Save recovered program (then you may Run)"
    } else {
        "Save recovered file"
    };
    let out = rfd::FileDialog::new()
        .set_title(title)
        .set_file_name(&suggested)
        .save_file()
        .ok_or_else(|| "cancelled".to_string())?;
    fs::write(&out, &recovered.data).map_err(|e| e.to_string())?;

    Ok(ExtractResultDto {
        is_text: false,
        kind,
        filename: recovered.meta.filename,
        method: recovered.method.as_str().to_string(),
        size: recovered.data.len(),
        checksum_hex: checksum,
        text_preview: None,
        saved_path: Some(out.to_string_lossy().to_string()),
        reveal_preview_url,
    })
}

#[tauri::command]
fn run_extracted(path: String) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("path is empty".into());
    }
    let p = PathBuf::from(&path);
    if !p.is_file() {
        return Err("file not found".into());
    }
    Command::new(&p)
        .spawn()
        .map_err(|e| format!("failed to start process: {e}"))?;
    Ok(())
}

#[tauri::command]
fn lab_lsb_demo() -> Result<String, String> {
    let mut cover = vec![0u8; 256];
    for (i, b) in cover.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(17);
    }
    let payload = b"LAB!";
    let mut seq = cover.clone();
    for (i, bit) in payload
        .iter()
        .flat_map(|b| (0..8).map(move |k| (b >> k) & 1))
        .enumerate()
    {
        if i < seq.len() {
            seq[i] = (seq[i] & 0xFE) | bit;
        }
    }
    let mut keyed = cover.clone();
    let mut order: Vec<usize> = (0..keyed.len()).collect();
    for i in (1..order.len()).rev() {
        let j = (i * 7 + 3) % (i + 1);
        order.swap(i, j);
    }
    for (i, bit) in payload
        .iter()
        .flat_map(|b| (0..8).map(move |k| (b >> k) & 1))
        .enumerate()
    {
        if i < order.len() {
            let idx = order[i];
            keyed[idx] = (keyed[idx] & 0xFE) | bit;
        }
    }
    let hist = |buf: &[u8]| -> (usize, usize) {
        let ones = buf.iter().map(|b| (b & 1) as usize).sum::<usize>();
        (buf.len() - ones, ones)
    };
    let (c0, c1) = hist(&cover);
    let (s0, s1) = hist(&seq);
    let (k0, k1) = hist(&keyed);
    Ok(format!(
        "LSB histogram (zeros, ones) on 256 sample bytes\n\
         cover:      ({c0}, {c1})\n\
         sequential: ({s0}, {s1})  <- early bytes absorb payload\n\
         keyed-ish:  ({k0}, {k1})  <- bits spread across buffer\n\n\
         Open Stego Method A uses password-keyed shuffle (ChaCha), not sequential order."
    ))
}

/// Live lab on a real PNG/BMP (capped at 256×256 for speed).
#[tauri::command]
fn lab_lsb_on_image(path: String) -> Result<String, String> {
    use image::{ImageBuffer, Rgba};
    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
    let thumb = img.thumbnail(256, 256).to_rgba8();
    let (w, h) = thumb.dimensions();
    let mut seq = thumb.clone();
    let mut keyed = thumb.clone();
    let payload = b"OPENSTEGO-LAB";
    let bits: Vec<u8> = payload
        .iter()
        .flat_map(|b| (0..8).map(move |k| (b >> k) & 1))
        .collect();

    // Sequential LSB on RGB channels
    let mut i = 0usize;
    'seq: for y in 0..h {
        for x in 0..w {
            for c in 0..3usize {
                if i >= bits.len() {
                    break 'seq;
                }
                let px = seq.get_pixel_mut(x, y);
                px.0[c] = (px.0[c] & 0xFE) | bits[i];
                i += 1;
            }
        }
    }

    // Keyed-ish shuffle of pixel slots
    let mut slots: Vec<(u32, u32, usize)> = Vec::new();
    for y in 0..h {
        for x in 0..w {
            for c in 0..3usize {
                slots.push((x, y, c));
            }
        }
    }
    for i in (1..slots.len()).rev() {
        let j = (i * 13 + 7) % (i + 1);
        slots.swap(i, j);
    }
    for (i, bit) in bits.iter().enumerate() {
        if i >= slots.len() {
            break;
        }
        let (x, y, c) = slots[i];
        let px = keyed.get_pixel_mut(x, y);
        px.0[c] = (px.0[c] & 0xFE) | bit;
    }

    let hist = |im: &ImageBuffer<Rgba<u8>, Vec<u8>>| -> (usize, usize) {
        let mut zeros = 0usize;
        let mut ones = 0usize;
        for p in im.pixels() {
            for c in 0..3 {
                if p.0[c] & 1 == 0 {
                    zeros += 1;
                } else {
                    ones += 1;
                }
            }
        }
        (zeros, ones)
    };
    let (c0, c1) = hist(&thumb);
    let (s0, s1) = hist(&seq);
    let (k0, k1) = hist(&keyed);
    Ok(format!(
        "Live LSB lab on {path}\n\
         resized to {w}×{h} for speed\n\n\
         LSB histogram (zeros, ones) across RGB channels:\n\
         cover:      ({c0}, {c1})\n\
         sequential: ({s0}, {s1})  <- clustered early changes\n\
         keyed-ish:  ({k0}, {k1})  <- spread by shuffle\n\n\
         Open Stego Method A uses ChaCha-keyed shuffle of bit slots (not sequential)."
    ))
}

#[tauri::command]
fn pick_lab_image() -> Result<String, String> {
    rfd::FileDialog::new()
        .set_title("Choose a small PNG/BMP for the live lab")
        .add_filter("Images", &["png", "bmp"])
        .pick_file()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "cancelled".to_string())
}

#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    open::that(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn preview_path(path: String) -> Result<Option<String>, String> {
    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    Ok(maybe_preview(&bytes, &path))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            app_version,
            password_strength,
            pick_cover,
            inspect_cover_path,
            pick_payload_file,
            set_payload_path,
            pick_keyfile,
            hide_payload,
            batch_hide,
            pick_output_dir,
            pick_stego_file,
            set_stego_path,
            extract_payload,
            run_extracted,
            lab_lsb_demo,
            lab_lsb_on_image,
            pick_lab_image,
            open_path,
            preview_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
