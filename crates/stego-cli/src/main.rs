use clap::{Parser, Subcommand};
use stego_core::{
    extract_with, hide_with, plan_hide, parse_kdf_profile, CryptoOptions, PayloadMeta,
    StegoOptions, VERSION,
};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(name = "stego")]
#[command(about = "Open steganography CLI (hide / extract)", long_about = None)]
#[command(version = VERSION)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Inspect a cover and print which embedding method would be used
    Plan {
        /// Path to the cover file
        #[arg(long)]
        cover: PathBuf,
        /// Machine-readable JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Structural inspect without password (magic / size / method guess)
    Inspect {
        #[arg(long)]
        input: PathBuf,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Environment / version self-check
    Doctor,
    /// Tiny Argon2 micro-benchmark for profiles
    Bench,
    /// Hide a payload inside a cover file
    Hide {
        /// Path to the cover file
        #[arg(long)]
        cover: PathBuf,
        /// Path to the payload file (mutually exclusive with --text / --text-file)
        #[arg(long)]
        payload: Option<PathBuf>,
        /// Hide this UTF-8 string as text payload
        #[arg(long)]
        text: Option<String>,
        /// Read text payload from a file (stored as text, not as a named file)
        #[arg(long)]
        text_file: Option<PathBuf>,
        /// Output path for the stego file
        #[arg(long, short)]
        output: PathBuf,
        /// Password (or set STEGO_PASSWORD). Prefer env in scripts.
        #[arg(long)]
        password: Option<String>,
        /// Decrypt immediately after hide to verify integrity
        #[arg(long)]
        verify: bool,
        /// Argon2 profile: fast | balanced | paranoid
        #[arg(long, default_value = "balanced")]
        profile: String,
        /// Optional keyfile mixed into password derivation
        #[arg(long)]
        keyfile: Option<PathBuf>,
        /// Method A: prefer high-variance pixels (lower capacity)
        #[arg(long, default_value_t = false)]
        adaptive_lsb: bool,
    },
    /// Extract a hidden payload from a stego file
    Extract {
        /// Path to the stego file
        #[arg(long)]
        input: PathBuf,
        /// Output path for a recovered file payload (required unless payload is text and --stdout)
        #[arg(long, short)]
        output: Option<PathBuf>,
        /// Print recovered text to stdout
        #[arg(long)]
        stdout: bool,
        /// Password (or set STEGO_PASSWORD)
        #[arg(long)]
        password: Option<String>,
        /// After saving an executable payload, run it (requires --i-understand)
        #[arg(long, default_value_t = false)]
        run: bool,
        /// Confirm you intentionally want to run a recovered executable (educational demos)
        #[arg(long = "i-understand", default_value_t = false)]
        i_understand: bool,
        /// Keyfile used at hide time (if any)
        #[arg(long)]
        keyfile: Option<PathBuf>,
        /// Try adaptive LSB layout first
        #[arg(long, default_value_t = false)]
        adaptive_lsb: bool,
    },
}

fn resolve_password(cli_pw: Option<String>) -> Result<String, String> {
    if let Some(p) = cli_pw {
        if p.is_empty() {
            return Err("password must not be empty".into());
        }
        return Ok(p);
    }
    if let Ok(p) = std::env::var("STEGO_PASSWORD") {
        if p.is_empty() {
            return Err("STEGO_PASSWORD is empty".into());
        }
        return Ok(p);
    }
    eprint!("Password: ");
    let _ = io::stderr().flush();
    // Non-echoing prompt is platform-specific; read a line for Stage 9 portability.
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    let p = line.trim_end_matches(['\r', '\n']).to_string();
    if p.is_empty() {
        return Err("password must not be empty".into());
    }
    Ok(p)
}

fn normalize_path(path: &Path) -> PathBuf {
    // Accept both Windows and Unix separators as provided by the shell.
    path.to_path_buf()
}

fn cmd_plan(cover: PathBuf, json: bool) -> Result<(), String> {
    let cover = normalize_path(&cover);
    let bytes = fs::read(&cover).map_err(|e| format!("read cover: {e}"))?;
    let path_str = cover.to_string_lossy().to_string();
    let plan = plan_hide(&bytes, &path_str).map_err(|e| e.to_string())?;
    if json {
        println!(
            "{{\"cover\":{},\"method\":{},\"capacity_bytes\":{},\"caveat\":{}}}",
            serde_json_str(&path_str),
            serde_json_str(plan.method.as_str()),
            plan
                .capacity
                .map(|c| c.to_string())
                .unwrap_or_else(|| "null".into()),
            plan
                .eof_caveat
                .as_deref()
                .map(serde_json_str)
                .unwrap_or_else(|| "null".into())
        );
        return Ok(());
    }
    println!("cover: {}", path_str);
    println!("method: {}", plan.method.as_str());
    if let Some(cap) = plan.capacity {
        println!("capacity_bytes: {cap}");
    } else {
        println!("capacity_bytes: unbounded (EOF method)");
        println!("output_size_estimate: {}", bytes.len());
        println!("note: final size = cover + encrypted envelope + footer");
    }
    if let Some(c) = plan.eof_caveat {
        println!("caveat: {c}");
    }
    Ok(())
}

fn serde_json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn cmd_inspect(input: PathBuf, json: bool) -> Result<(), String> {
    let input = normalize_path(&input);
    let bytes = fs::read(&input).map_err(|e| format!("read: {e}"))?;
    let path_str = input.to_string_lossy().to_string();
    let plan = plan_hide(&bytes, &path_str).map_err(|e| e.to_string())?;
    let magic = if bytes.len() >= 8 && &bytes[..8] == b"\x89PNG\r\n\x1a\n" {
        "png"
    } else if bytes.len() >= 4 && &bytes[..4] == b"%PDF" {
        "pdf"
    } else if bytes.len() >= 2 && bytes[0] == b'B' && bytes[1] == b'M' {
        "bmp"
    } else {
        "unknown/generic"
    };
    if json {
        println!(
            "{{\"path\":{},\"size\":{},\"magic_guess\":{},\"plan_method\":{}}}",
            serde_json_str(&path_str),
            bytes.len(),
            serde_json_str(magic),
            serde_json_str(plan.method.as_str())
        );
    } else {
        println!("path: {path_str}");
        println!("size: {}", bytes.len());
        println!("magic_guess: {magic}");
        println!("plan_method: {}", plan.method.as_str());
        println!("note: inspect does not decrypt; password not required");
    }
    Ok(())
}

fn cmd_doctor() -> Result<(), String> {
    println!("stego-cli {}", VERSION);
    println!("ok: binary runs");
    println!("hint: cargo test -p stego-core --lib");
    Ok(())
}

fn cmd_bench() -> Result<(), String> {
    use stego_core::{derive_keys_with, CryptoOptions, SALT_LEN};
    let salt = [9u8; SALT_LEN];
    for name in ["fast", "balanced"] {
        let profile = parse_kdf_profile(name).map_err(|e| e.to_string())?;
        let opts = CryptoOptions {
            profile,
            keyfile: None,
        };
        let t0 = std::time::Instant::now();
        let _ = derive_keys_with("bench-pass", &salt, &opts).map_err(|e| e.to_string())?;
        println!("{name}: {:?}", t0.elapsed());
    }
    Ok(())
}

fn load_keyfile(path: Option<PathBuf>) -> Result<Option<Vec<u8>>, String> {
    match path {
        None => Ok(None),
        Some(p) => {
            let bytes = fs::read(&p).map_err(|e| format!("read keyfile: {e}"))?;
            if bytes.is_empty() {
                return Err("keyfile is empty".into());
            }
            Ok(Some(bytes))
        }
    }
}

fn build_opts(
    profile: &str,
    keyfile: Option<PathBuf>,
    adaptive_lsb: bool,
) -> Result<StegoOptions, String> {
    let profile = parse_kdf_profile(profile).map_err(|e| e.to_string())?;
    Ok(StegoOptions {
        crypto: CryptoOptions {
            profile,
            keyfile: load_keyfile(keyfile)?,
        },
        adaptive_lsb,
    })
}

fn cmd_hide(
    cover: PathBuf,
    payload: Option<PathBuf>,
    text: Option<String>,
    text_file: Option<PathBuf>,
    output: PathBuf,
    password: Option<String>,
    verify: bool,
    profile: String,
    keyfile: Option<PathBuf>,
    adaptive_lsb: bool,
) -> Result<(), String> {
    let cover = normalize_path(&cover);
    let output = normalize_path(&output);
    let modes = [
        payload.is_some(),
        text.is_some(),
        text_file.is_some(),
    ]
    .iter()
    .filter(|&&x| x)
    .count();
    if modes != 1 {
        return Err("specify exactly one of --payload, --text, or --text-file".into());
    }

    let cover_bytes = fs::read(&cover).map_err(|e| format!("read cover: {e}"))?;
    let cover_str = cover.to_string_lossy().to_string();
    let plan = plan_hide(&cover_bytes, &cover_str).map_err(|e| e.to_string())?;
    eprintln!("using {}", plan.method.as_str());
    if let Some(cap) = plan.capacity {
        eprintln!("capacity: {cap} bytes");
    }
    if let Some(c) = &plan.eof_caveat {
        eprintln!("caveat: {c}");
    }

    let (meta, data) = if let Some(p) = payload {
        let data = fs::read(&p).map_err(|e| format!("read payload: {e}"))?;
        let name = p
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("payload.bin")
            .to_string();
        (PayloadMeta::for_path_file(name, &data), data)
    } else if let Some(t) = text {
        let data = t.into_bytes();
        (PayloadMeta::for_text(&data), data)
    } else {
        let path = text_file.unwrap();
        let mut data = Vec::new();
        fs::File::open(&path)
            .and_then(|mut f| f.read_to_end(&mut data))
            .map_err(|e| format!("read text-file: {e}"))?;
        (PayloadMeta::for_text(&data), data)
    };

    let opts = build_opts(&profile, keyfile, adaptive_lsb)?;
    eprintln!(
        "kdf: {}{}",
        opts.crypto.profile.as_str(),
        if opts.crypto.keyfile.is_some() {
            " + keyfile"
        } else {
            ""
        }
    );

    let password = resolve_password(password)?;
    let (stego, ext) = hide_with(&cover_bytes, &cover_str, &meta, &data, &password, &opts)
        .map_err(|e| e.to_string())?;

    let out_path = if output.extension().is_none() && !ext.is_empty() {
        output.with_extension(&ext)
    } else {
        output
    };

    if verify {
        let check = extract_with(&stego, &out_path.to_string_lossy(), &password, &opts)
            .map_err(|e| format!("verify failed: {e}"))?;
        if check.data != data {
            return Err("verify failed: payload mismatch".into());
        }
        eprintln!("verify: ok");
    }

    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("create dirs: {e}"))?;
        }
    }
    fs::write(&out_path, &stego).map_err(|e| format!("write output: {e}"))?;
    println!("{}", out_path.display());
    eprintln!("wrote {} bytes", stego.len());
    Ok(())
}

fn cmd_extract(
    input: PathBuf,
    output: Option<PathBuf>,
    stdout: bool,
    password: Option<String>,
    run: bool,
    i_understand: bool,
    keyfile: Option<PathBuf>,
    adaptive_lsb: bool,
) -> Result<(), String> {
    let input = normalize_path(&input);
    let stego = fs::read(&input).map_err(|e| format!("read input: {e}"))?;
    let password = resolve_password(password)?;
    let opts = build_opts("balanced", keyfile, adaptive_lsb)?;
    let recovered = extract_with(&stego, &input.to_string_lossy(), &password, &opts)
        .map_err(|e| e.to_string())?;

    eprintln!("method: {}", recovered.method.as_str());
    eprintln!("kind: {}", recovered.meta.kind().as_str());
    if recovered.meta.is_text {
        let text = String::from_utf8(recovered.data)
            .map_err(|_| "payload marked as text but is not valid UTF-8".to_string())?;
        if stdout || output.is_none() {
            print!("{text}");
            if !text.ends_with('\n') {
                println!();
            }
        } else {
            let out = normalize_path(output.as_ref().unwrap());
            fs::write(&out, text.as_bytes()).map_err(|e| format!("write output: {e}"))?;
            println!("{}", out.display());
        }
        if run {
            return Err("--run only applies to executable payloads".into());
        }
        return Ok(());
    }

    let out = match output {
        Some(p) => normalize_path(&p),
        None => {
            let name = recovered
                .meta
                .filename
                .clone()
                .unwrap_or_else(|| "recovered.bin".into());
            PathBuf::from(name)
        }
    };
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("create dirs: {e}"))?;
        }
    }
    fs::write(&out, &recovered.data).map_err(|e| format!("write output: {e}"))?;
    println!("{}", out.display());

    if run {
        if !i_understand {
            return Err(
                "refusing to run: pass --i-understand together with --run (transparent demo only)"
                    .into(),
            );
        }
        if !recovered.meta.is_executable {
            return Err("--run requires an executable-kind payload".into());
        }
        eprintln!(
            "running recovered file (you confirmed with --i-understand): {}",
            out.display()
        );
        let status = std::process::Command::new(&out)
            .status()
            .map_err(|e| format!("run failed: {e}"))?;
        if !status.success() {
            return Err(format!("process exited with {status}"));
        }
    } else if recovered.meta.is_executable {
        eprintln!("hint: executable payload saved; to run it use --run --i-understand");
    }
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Plan { cover, json } => cmd_plan(cover, json),
        Commands::Inspect { input, json } => cmd_inspect(input, json),
        Commands::Doctor => cmd_doctor(),
        Commands::Bench => cmd_bench(),
        Commands::Hide {
            cover,
            payload,
            text,
            text_file,
            output,
            password,
            verify,
            profile,
            keyfile,
            adaptive_lsb,
        } => cmd_hide(
            cover,
            payload,
            text,
            text_file,
            output,
            password,
            verify,
            profile,
            keyfile,
            adaptive_lsb,
        ),
        Commands::Extract {
            input,
            output,
            stdout,
            password,
            run,
            i_understand,
            keyfile,
            adaptive_lsb,
        } => cmd_extract(
            input,
            output,
            stdout,
            password,
            run,
            i_understand,
            keyfile,
            adaptive_lsb,
        ),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
