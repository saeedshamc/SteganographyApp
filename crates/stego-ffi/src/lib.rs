//! C FFI for stego-core (Flutter / mobile / other languages).
//!
//! Opaque string results are UTF-8 JSON. Free with [`stego_string_free`].

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;

use stego_core::{
    extract_with, hide_with, plan_hide_with, CryptoOptions, PayloadMeta, StegoOptions, VERSION,
};

const OK: c_int = 0;
const ERR: c_int = 1;

unsafe fn cstr(p: *const c_char) -> Result<&'static str, String> {
    if p.is_null() {
        return Err("null pointer".into());
    }
    CStr::from_ptr(p)
        .to_str()
        .map_err(|e| e.to_string())
}

fn alloc_str(s: impl Into<String>) -> *mut c_char {
    match CString::new(s.into()) {
        Ok(c) => c.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

fn write_out(out: *mut *mut c_char, s: String) {
    if !out.is_null() {
        unsafe {
            *out = alloc_str(s);
        }
    }
}

/// Library version string (caller must free).
#[no_mangle]
pub extern "C" fn stego_version() -> *mut c_char {
    alloc_str(VERSION)
}

/// Free a string returned by this library.
#[no_mangle]
pub unsafe extern "C" fn stego_string_free(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    drop(CString::from_raw(s));
}

/// Plan hide. On success writes JSON to `out_json` (free with stego_string_free).
#[no_mangle]
pub unsafe extern "C" fn stego_plan(
    cover_path: *const c_char,
    lsb_depth: c_int,
    out_json: *mut *mut c_char,
) -> c_int {
    let run = || -> Result<String, String> {
        let path = unsafe { cstr(cover_path) }?;
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let opts = StegoOptions {
            crypto: CryptoOptions::default(),
            adaptive_lsb: false,
            lsb_depth: if lsb_depth <= 0 { 1 } else { lsb_depth as u8 },
        };
        let plan = plan_hide_with(&bytes, path, &opts, None).map_err(|e| e.to_string())?;
        Ok(serde_json::json!({
            "ok": true,
            "command": "plan",
            "method": plan.method.as_str(),
            "capacity_bytes": plan.capacity,
            "eof_caveat": plan.eof_caveat,
            "capacity_risk": plan.capacity_risk,
            "jpeg_warning": plan.jpeg_warning,
        })
        .to_string())
    };
    match run() {
        Ok(j) => {
            write_out(out_json, j);
            OK
        }
        Err(e) => {
            write_out(
                out_json,
                serde_json::json!({"ok":false,"command":"plan","error":e}).to_string(),
            );
            ERR
        }
    }
}

/// Hide payload file into cover. Writes stego to `output_path`.
#[no_mangle]
pub unsafe extern "C" fn stego_hide(
    cover_path: *const c_char,
    payload_path: *const c_char,
    output_path: *const c_char,
    password: *const c_char,
    lsb_depth: c_int,
    out_json: *mut *mut c_char,
) -> c_int {
    let run = || -> Result<String, String> {
        let cover_p = unsafe { cstr(cover_path) }?;
        let payload_p = unsafe { cstr(payload_path) }?;
        let out_p = unsafe { cstr(output_path) }?;
        let pw = unsafe { cstr(password) }?;
        if pw.is_empty() {
            return Err("password required".into());
        }
        let cover = std::fs::read(cover_p).map_err(|e| e.to_string())?;
        let data = std::fs::read(payload_p).map_err(|e| e.to_string())?;
        let name = std::path::Path::new(payload_p)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("payload.bin")
            .to_string();
        let meta = PayloadMeta::for_path_file(name, &data);
        let opts = StegoOptions {
            crypto: CryptoOptions::default(),
            adaptive_lsb: false,
            lsb_depth: if lsb_depth <= 0 { 1 } else { lsb_depth as u8 },
        };
        let (stego, ext) =
            hide_with(&cover, cover_p, &meta, &data, pw, &opts).map_err(|e| e.to_string())?;
        std::fs::write(out_p, &stego).map_err(|e| e.to_string())?;
        Ok(serde_json::json!({
            "ok": true,
            "command": "hide",
            "output": out_p,
            "bytes": stego.len(),
            "extension": ext,
            "kind": meta.kind().as_str(),
        })
        .to_string())
    };
    match run() {
        Ok(j) => {
            write_out(out_json, j);
            OK
        }
        Err(e) => {
            write_out(
                out_json,
                serde_json::json!({"ok":false,"command":"hide","error":e}).to_string(),
            );
            ERR
        }
    }
}

/// Extract payload from stego to `output_path` (file payloads) or return text in JSON.
#[no_mangle]
pub unsafe extern "C" fn stego_extract(
    stego_path: *const c_char,
    output_path: *const c_char,
    password: *const c_char,
    lsb_depth: c_int,
    out_json: *mut *mut c_char,
) -> c_int {
    let run = || -> Result<String, String> {
        let stego_p = unsafe { cstr(stego_path) }?;
        let out_p = if output_path.is_null() {
            None
        } else {
            Some(unsafe { cstr(output_path) }?)
        };
        let pw = unsafe { cstr(password) }?;
        if pw.is_empty() {
            return Err("password required".into());
        }
        let stego = std::fs::read(stego_p).map_err(|e| e.to_string())?;
        let opts = StegoOptions {
            crypto: CryptoOptions::default(),
            adaptive_lsb: false,
            lsb_depth: if lsb_depth <= 0 { 1 } else { lsb_depth as u8 },
        };
        let recovered =
            extract_with(&stego, stego_p, pw, &opts).map_err(|e| e.to_string())?;
        if recovered.meta.is_text {
            let text = String::from_utf8(recovered.data)
                .map_err(|_| "payload is not valid UTF-8".to_string())?;
            return Ok(serde_json::json!({
                "ok": true,
                "command": "extract",
                "kind": "text",
                "method": recovered.method.as_str(),
                "size": text.len(),
                "text": text,
            })
            .to_string());
        }
        let dest = out_p.ok_or_else(|| "output_path required for file payloads".to_string())?;
        std::fs::write(dest, &recovered.data).map_err(|e| e.to_string())?;
        Ok(serde_json::json!({
            "ok": true,
            "command": "extract",
            "kind": recovered.meta.kind().as_str(),
            "method": recovered.method.as_str(),
            "size": recovered.data.len(),
            "output": dest,
            "filename": recovered.meta.filename,
        })
        .to_string())
    };
    match run() {
        Ok(j) => {
            write_out(out_json, j);
            OK
        }
        Err(e) => {
            write_out(
                out_json,
                serde_json::json!({"ok":false,"command":"extract","error":e}).to_string(),
            );
            ERR
        }
    }
}
