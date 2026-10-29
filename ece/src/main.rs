// main.rs — CLI entry point for Evidence Checkpoint Engine.

use clap::{Parser, Subcommand};
use ece_lib::verifier::{
    default_schema_dir, inspect_payload, verify_envelope, VerificationOutput, MAX_INPUT_BYTES,
};
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "ece",
    version,
    about = "Evidence Checkpoint Engine — offline verifier CLI"
)]
struct Cli {
    #[arg(long, global = true, help = "Path to directory containing draft schemas")]
    schema_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Verify a canonical evidence checkpoint envelope")]
    VerifyEnvelope {
        #[arg(long, help = "Path to envelope JSON file or '-' to read from stdin")]
        input: String,
    },

    #[command(about = "Inspect an untrusted inner checkpoint input payload (diagnostic only)")]
    InspectUntrustedPayload {
        #[arg(long, help = "Path to payload JSON file or '-' to read from stdin")]
        input: String,
    },
}

fn resolve_schema_dir(explicit: Option<PathBuf>) -> PathBuf {
    if let Some(dir) = explicit {
        return dir;
    }

    if let Ok(env_dir) = std::env::var("ECE_SCHEMA_DIR") {
        return PathBuf::from(env_dir);
    }

    // Try standard candidate paths
    let candidates = [
        PathBuf::from("docs/gov-00/schemas"),
        PathBuf::from("../docs/gov-00/schemas"),
        PathBuf::from("../../docs/gov-00/schemas"),
    ];

    for c in &candidates {
        if c.join("checkpoint-envelope.schema.json").exists() {
            return c.clone();
        }
    }

    // If running alongside executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let candidate = parent.join("../docs/gov-00/schemas");
            if candidate.join("checkpoint-envelope.schema.json").exists() {
                return candidate;
            }
        }
    }

    default_schema_dir()
}

fn read_bounded_input(input_source: &str) -> Result<Vec<u8>, (String, i32)> {
    let mut buffer = Vec::new();

    if input_source == "-" {
        let mut handle = io::stdin().take((MAX_INPUT_BYTES + 1024) as u64);
        handle.read_to_end(&mut buffer).map_err(|e| {
            (
                format!("Failed to read from standard input: {e}"),
                4, // Operational error
            )
        })?;
    } else {
        let path = Path::new(input_source);
        if !path.exists() {
            return Err((
                format!("Input file not found: {}", path.display()),
                4, // Operational error
            ));
        }

        let file = File::open(path).map_err(|e| {
            (
                format!("Failed to open input file {}: {e}", path.display()),
                4,
            )
        })?;

        let mut handle = file.take((MAX_INPUT_BYTES + 1024) as u64);
        handle.read_to_end(&mut buffer).map_err(|e| {
            (
                format!("Failed to read input file {}: {e}", path.display()),
                4,
            )
        })?;
    }

    Ok(buffer)
}

fn main() {
    let cli = Cli::parse();
    let schema_dir = resolve_schema_dir(cli.schema_dir);

    let (input_path, is_verify) = match cli.command {
        Commands::VerifyEnvelope { input } => (input, true),
        Commands::InspectUntrustedPayload { input } => (input, false),
    };

    let input_bytes = match read_bounded_input(&input_path) {
        Ok(bytes) => bytes,
        Err((err_msg, exit_code)) => {
            eprintln!("ECE_OPERATIONAL_ERROR: {err_msg}");
            std::process::exit(exit_code);
        }
    };

    let output: VerificationOutput = if is_verify {
        verify_envelope(&input_bytes, &schema_dir)
    } else {
        inspect_payload(&input_bytes, &schema_dir)
    };

    // Emit structured assessment JSON to stdout
    match serde_json::to_string_pretty(&output.assessment) {
        Ok(json_str) => {
            println!("{json_str}");
        }
        Err(e) => {
            eprintln!("ECE_INTERNAL_ERROR: Failed to serialize assessment output: {e}");
            std::process::exit(4);
        }
    }

    // Diagnostics to stderr
    let verdict = output
        .assessment
        .pointer("/derived_summary/verdict")
        .and_then(|v| v.as_str())
        .unwrap_or("UNKNOWN");

    eprintln!(
        "ECE: mode={} verdict={} exit_code={}",
        if is_verify {
            "verify-envelope"
        } else {
            "inspect-untrusted-payload"
        },
        verdict,
        output.exit_code
    );

    std::process::exit(output.exit_code);
}
