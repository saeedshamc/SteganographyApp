//! Tauri desktop shell for Open Stego.

use serde::Serialize;
use stego_core::{hide, plan_hide, PayloadMeta, VERSION};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanDto {
    method: String,
    capacity: Option<usize>,
    jpeg_warning: Option<String>,
    eof_caveat: Option<String>,
    cover_path: String,
    cover_size: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HideResultDto {
    output_path: String,
    output_size: usize,
    extension: String,
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
    inspect_cover(path)
}

fn inspect_cover(path: PathBuf) -> Result<PlanDto, String> {
    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    let path_str = path.to_string_lossy().to_string();
    let plan = plan_hide(&bytes, &path_str).map_err(|e| e.to_string())?;
    Ok(PlanDto {
        method: plan.method.as_str().to_string(),
        capacity: plan.capacity,
        jpeg_warning: plan.jpeg_warning,
        eof_caveat: plan.eof_caveat,
        cover_path: path_str,
        cover_size: bytes.len(),
    })
}

#[tauri::command]
fn pick_payload_file() -> Result<(String, usize), String> {
    let path = rfd::FileDialog::new()
        .set_title("Choose payload file to hide")
        .pick_file()
        .ok_or_else(|| "cancelled".to_string())?;
    let meta = fs::metadata(&path).map_err(|e| e.to_string())?;
    Ok((path.to_string_lossy().to_string(), meta.len() as usize))
}

#[tauri::command]
fn hide_payload(
    cover_path: String,
    payload_path: Option<String>,
    text_payload: Option<String>,
    password: String,
    verify: bool,
) -> Result<HideResultDto, String> {
    if password.is_empty() {
        return Err("password is required".into());
    }
    let cover = fs::read(&cover_path).map_err(|e| e.to_string())?;

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
        (PayloadMeta::for_file(name, &data), data)
    } else {
        return Err("provide a payload file or text".into());
    };

    let (stego, ext) = hide(&cover, &cover_path, &meta, &payload_bytes, &password)
        .map_err(|e| e.to_string())?;

    if verify {
        let check = stego_core::extract(&stego, &format!("verify.{ext}"), &password)
            .map_err(|e| format!("round-trip verify failed: {e}"))?;
        if check.data != payload_bytes {
            return Err("round-trip verify failed: payload mismatch".into());
        }
    }

    let default_name = {
        let stem = std::path::Path::new(&cover_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("stego");
        format!("{stem}_stego.{ext}")
    };

    let out = rfd::FileDialog::new()
        .set_title("Save stego output")
        .set_file_name(&default_name)
        .save_file()
        .ok_or_else(|| "cancelled".to_string())?;

    fs::write(&out, &stego).map_err(|e| e.to_string())?;
    Ok(HideResultDto {
        output_path: out.to_string_lossy().to_string(),
        output_size: stego.len(),
        extension: ext,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            app_version,
            password_strength,
            pick_cover,
            pick_payload_file,
            hide_payload,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
