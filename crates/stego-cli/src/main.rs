use clap::{Parser, Subcommand};
use stego_core::VERSION;

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
    /// Hide a payload inside a cover file (wired in later stages)
    Hide {
        /// Path to the cover file
        #[arg(long)]
        cover: String,
        /// Path to the payload file (optional if --text is used later)
        #[arg(long)]
        payload: Option<String>,
        /// Output path for the stego file
        #[arg(long, short)]
        output: String,
    },
    /// Extract a hidden payload from a stego file (wired in later stages)
    Extract {
        /// Path to the stego file
        #[arg(long)]
        input: String,
        /// Output path for the recovered payload
        #[arg(long, short)]
        output: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Hide { cover, payload, output } => {
            eprintln!(
                "hide: cover={cover} payload={payload:?} output={output} (core not wired yet)"
            );
            eprintln!("stego-core {} — embedding arrives in later stages", VERSION);
            std::process::exit(2);
        }
        Commands::Extract { input, output } => {
            eprintln!("extract: input={input} output={output:?} (core not wired yet)");
            eprintln!("stego-core {} — extraction arrives in later stages", VERSION);
            std::process::exit(2);
        }
    }
}
