// main.rs — CLI entry point for Evidence Checkpoint Engine.

use clap::{Parser, Subcommand};
use ece_lib::github::{
    evaluate_github_evidence, DeclaredArtifactSpec, HttpGitHubClient, TargetEvidenceRequest,
    TrustedCheckPolicy,
};
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
    about = "Evidence Checkpoint Engine — offline verifier & GitHub adapter CLI"
)]
struct Cli {
    #[arg(
        long,
        global = true,
        help = "Path to directory containing draft schemas"
    )]
    schema_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Verify a canonical evidence checkpoint envelope (offline)")]
    VerifyEnvelope {
        #[arg(long, help = "Path to envelope JSON file or '-' to read from stdin")]
        input: String,
    },

    #[command(about = "Inspect untrusted inner checkpoint payload (offline)")]
    InspectUntrustedPayload {
        #[arg(long, help = "Path to payload JSON file or '-' to read from stdin")]
        input: String,
    },

    #[command(about = "Verify a checkpoint against live GitHub REST API evidence")]
    VerifyGithubEvidence {
        #[arg(long, help = "Path to envelope or payload JSON file or '-' to read from stdin")]
        input: String,

        #[arg(long, help = "Path to custom trusted policy JSON file (optional)")]
        policy: Option<PathBuf>,

        #[arg(long, help = "Explicit GitHub token (optional, or GITHUB_TOKEN env)")]
        token: Option<String>,
    },

    #[command(about = "Directly assess a GitHub workflow run against trusted policy")]
    AssessGithubRun {
        #[arg(long, help = "Repository owner (e.g. ErenTahiroglu)")]
        owner: String,

        #[arg(long, help = "Repository name (e.g. Evidence-Checkpoint-Engine)")]
        repo: String,

        #[arg(long, help = "Expected numeric repository REST ID (e.g. 1432704795)")]
        expected_repo_id: String,

        #[arg(long, help = "Expected target commit SHA")]
        commit_sha: String,

        #[arg(long, help = "Workflow run ID")]
        run_id: u64,

        #[arg(long, default_value_t = 1, help = "Workflow run attempt number")]
        run_attempt: u32,

        #[arg(long, help = "Path to custom trusted policy JSON file (optional)")]
        policy: Option<PathBuf>,

        #[arg(long, help = "Explicit GitHub token (optional, or GITHUB_TOKEN env)")]
        token: Option<String>,
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

fn load_policy(path: Option<PathBuf>) -> Result<TrustedCheckPolicy, (String, i32)> {
    if let Some(p) = path {
        let content = std::fs::read_to_string(&p).map_err(|e| {
            (
                format!("Failed to read policy file {}: {e}", p.display()),
                4,
            )
        })?;
        serde_json::from_str(&content).map_err(|e| {
            (
                format!("Failed to parse policy JSON {}: {e}", p.display()),
                4,
            )
        })
    } else {
        Ok(TrustedCheckPolicy::ece_pilot_policy())
    }
}

fn extract_target_from_json(
    val: &serde_json::Value,
) -> Result<TargetEvidenceRequest, (String, i32)> {
    let payload = if val.get("payload").is_some() {
        &val["payload"]
    } else {
        val
    };

    let owner = payload
        .pointer("/repository/owner")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ("Missing /repository/owner in checkpoint payload".to_string(), 3))?
        .to_string();

    let repo = payload
        .pointer("/repository/name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ("Missing /repository/name in checkpoint payload".to_string(), 3))?
        .to_string();

    let expected_repo_id = payload
        .pointer("/repository/github_rest_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            (
                "Missing /repository/github_rest_id in checkpoint payload".to_string(),
                3,
            )
        })?
        .to_string();

    let commit_sha = payload
        .pointer("/source/commit_sha")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ("Missing /source/commit_sha in checkpoint payload".to_string(), 3))?
        .to_string();

    let run_id = payload
        .pointer("/ci_evidence/run_id")
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                s.parse::<u64>().ok()
            } else {
                v.as_u64()
            }
        })
        .ok_or_else(|| {
            (
                "Missing or invalid /ci_evidence/run_id in checkpoint payload".to_string(),
                3,
            )
        })?;

    let run_attempt = payload
        .pointer("/ci_evidence/run_attempt")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as u32;

    let expected_workflow_path = payload
        .pointer("/ci_evidence/workflow_path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let checkpoint_id = payload
        .pointer("/checkpoint_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let declared_phase = payload
        .pointer("/phase")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let declared_producer = payload
        .pointer("/producer/declared_by")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mut declared_artifacts = Vec::new();
    if let Some(arr) = payload.pointer("/artifact_evidence").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(name) = item.get("artifact_name").and_then(|v| v.as_str()) {
                let id = item.get("artifact_id").and_then(|v| {
                    if let Some(s) = v.as_str() {
                        s.parse::<u64>().ok()
                    } else {
                        v.as_u64()
                    }
                });
                let digest = item
                    .get("digest_value")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                declared_artifacts.push(DeclaredArtifactSpec {
                    artifact_name: name.to_string(),
                    artifact_id: id,
                    expected_digest: digest,
                    verify_bytes: false,
                });
            }
        }
    }

    Ok(TargetEvidenceRequest {
        owner,
        repo,
        expected_repo_id,
        commit_sha,
        run_id,
        run_attempt,
        expected_workflow_path,
        checkpoint_id,
        declared_phase,
        declared_producer,
        declared_artifacts,
    })
}

fn main() {
    let cli = Cli::parse();
    let schema_dir = resolve_schema_dir(cli.schema_dir);

    let mode_str: &'static str;
    let output: VerificationOutput = match cli.command {
        Commands::VerifyEnvelope { input } => {
            mode_str = "verify-envelope";
            let input_bytes = match read_bounded_input(&input) {
                Ok(b) => b,
                Err((msg, code)) => {
                    eprintln!("ECE_OPERATIONAL_ERROR: {msg}");
                    std::process::exit(code);
                }
            };
            match verify_envelope(&input_bytes, &schema_dir) {
                Ok(out) => out,
                Err(e) => {
                    eprintln!("ECE_OPERATIONAL_ERROR: [{}] {}", e.class(), e);
                    std::process::exit(e.exit_code());
                }
            }
        }
        Commands::InspectUntrustedPayload { input } => {
            mode_str = "inspect-untrusted-payload";
            let input_bytes = match read_bounded_input(&input) {
                Ok(b) => b,
                Err((msg, code)) => {
                    eprintln!("ECE_OPERATIONAL_ERROR: {msg}");
                    std::process::exit(code);
                }
            };
            match inspect_payload(&input_bytes, &schema_dir) {
                Ok(out) => out,
                Err(e) => {
                    eprintln!("ECE_OPERATIONAL_ERROR: [{}] {}", e.class(), e);
                    std::process::exit(e.exit_code());
                }
            }
        }
        Commands::VerifyGithubEvidence {
            input,
            policy: policy_path,
            token,
        } => {
            mode_str = "verify-github-evidence";
            let input_bytes = match read_bounded_input(&input) {
                Ok(b) => b,
                Err((msg, code)) => {
                    eprintln!("ECE_OPERATIONAL_ERROR: {msg}");
                    std::process::exit(code);
                }
            };
            let parsed_json: serde_json::Value = match serde_json::from_slice(&input_bytes) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("ECE_OPERATIONAL_ERROR: Malformed input JSON: {e}");
                    std::process::exit(3);
                }
            };
            let target = match extract_target_from_json(&parsed_json) {
                Ok(t) => t,
                Err((msg, code)) => {
                    eprintln!("ECE_OPERATIONAL_ERROR: {msg}");
                    std::process::exit(code);
                }
            };
            let policy = match load_policy(policy_path) {
                Ok(p) => p,
                Err((msg, code)) => {
                    eprintln!("ECE_OPERATIONAL_ERROR: {msg}");
                    std::process::exit(code);
                }
            };
            let client = match HttpGitHubClient::new(token) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("ECE_OPERATIONAL_ERROR: Failed to initialize GitHub client: {e}");
                    std::process::exit(4);
                }
            };
            match evaluate_github_evidence(&client, &target, &policy, &schema_dir) {
                Ok(out) => out,
                Err(e) => {
                    eprintln!("ECE_OPERATIONAL_ERROR: [{}] {}", e.class(), e);
                    std::process::exit(e.exit_code());
                }
            }
        }
        Commands::AssessGithubRun {
            owner,
            repo,
            expected_repo_id,
            commit_sha,
            run_id,
            run_attempt,
            policy: policy_path,
            token,
        } => {
            mode_str = "assess-github-run";
            let target = TargetEvidenceRequest {
                owner,
                repo,
                expected_repo_id,
                commit_sha,
                run_id,
                run_attempt,
                expected_workflow_path: None,
                checkpoint_id: None,
                declared_phase: Some("GOV-02".to_string()),
                declared_producer: Some("CLI-invoker".to_string()),
                declared_artifacts: Vec::new(),
            };
            let policy = match load_policy(policy_path) {
                Ok(p) => p,
                Err((msg, code)) => {
                    eprintln!("ECE_OPERATIONAL_ERROR: {msg}");
                    std::process::exit(code);
                }
            };
            let client = match HttpGitHubClient::new(token) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("ECE_OPERATIONAL_ERROR: Failed to initialize GitHub client: {e}");
                    std::process::exit(4);
                }
            };
            match evaluate_github_evidence(&client, &target, &policy, &schema_dir) {
                Ok(out) => out,
                Err(e) => {
                    eprintln!("ECE_OPERATIONAL_ERROR: [{}] {}", e.class(), e);
                    std::process::exit(e.exit_code());
                }
            }
        }
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
        mode_str, verdict, output.exit_code
    );

    std::process::exit(output.exit_code);
}
