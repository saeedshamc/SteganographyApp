use clap::{Parser, Subcommand};
use stego_core::{extract, hide, plan_hide, PayloadMeta, VERSION};
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
    },
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

fn cmd_plan(cover: PathBuf) -> Result<(), String> {
    let cover = normalize_path(&cover);
    let bytes = fs::read(&cover).map_err(|e| format!("read cover: {e}"))?;
    let path_str = cover.to_string_lossy();
    let plan = plan_hide(&bytes, &path_str).map_err(|e| e.to_string())?;
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

fn cmd_hide(
    cover: PathBuf,
    payload: Option<PathBuf>,
    text: Option<String>,
    text_file: Option<PathBuf>,
    output: PathBuf,
    password: Option<String>,
    verify: bool,
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
        (PayloadMeta::for_file(name, &data), data)
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

    let password = resolve_password(password)?;
    let (stego, ext) = hide(&cover_bytes, &cover_str, &meta, &data, &password)
        .map_err(|e| e.to_string())?;

    // If user omitted a known extension, keep the core's suggested one when possible.
    let out_path = if output.extension().is_none() && !ext.is_empty() {
        output.with_extension(&ext)
    } else {
        output
    };

    if verify {
        let check = extract(&stego, &out_path.to_string_lossy(), &password)
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
) -> Result<(), String> {
    let input = normalize_path(&input);
    let stego = fs::read(&input).map_err(|e| format!("read input: {e}"))?;
    let password = resolve_password(password)?;
    let recovered =
        extract(&stego, &input.to_string_lossy(), &password).map_err(|e| e.to_string())?;

    eprintln!("method: {}", recovered.method.as_str());
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
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Plan { cover } => cmd_plan(cover),
        Commands::Hide {
            cover,
            payload,
            text,
            text_file,
            output,
            password,
            verify,
        } => cmd_hide(cover, payload, text, text_file, output, password, verify),
        Commands::Extract {
            input,
            output,
            stdout,
            password,
        } => cmd_extract(input, output, stdout, password),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
