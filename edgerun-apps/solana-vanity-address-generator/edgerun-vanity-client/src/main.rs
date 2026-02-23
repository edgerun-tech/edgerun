// SPDX-License-Identifier: Apache-2.0
use anyhow::{anyhow, Context, Result};
use base64::Engine;
use clap::{Parser, ValueEnum};
use edgerun_vanity_payload::{
    decode_response, derive_keypair, encode_not_found, encode_request, execute_request,
    SearchRequest, SearchResponse,
};
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration, Instant};

#[derive(Debug, Parser)]
#[command(name = "edgerun-vanity-client")]
#[command(about = "Escrow-bounded Solana vanity search orchestrator for edgerun")]
struct Cli {
    #[arg(long, value_enum, default_value_t = SearchMode::SecureLocal)]
    mode: SearchMode,
    #[arg(long)]
    scheduler_url: Option<String>,
    #[arg(long)]
    runtime_id: Option<String>,
    #[arg(long)]
    wasm_path: Option<std::path::PathBuf>,
    #[arg(long)]
    seed_hex: String,
    #[arg(long)]
    prefix: String,
    #[arg(long, default_value_t = 0)]
    start_counter: u64,
    #[arg(long)]
    end_counter: u64,
    #[arg(long, default_value_t = 50_000)]
    chunk_attempts: u64,
    #[arg(long)]
    escrow_per_job_lamports: u64,
    #[arg(long)]
    max_escrow_lamports: u64,
    #[arg(long, default_value_t = 1024 * 1024)]
    max_memory_bytes: u32,
    #[arg(long, default_value_t = 100_000_000)]
    max_instructions: u64,
    #[arg(long, default_value_t = 1000)]
    poll_interval_ms: u64,
    #[arg(long, default_value_t = 120)]
    job_timeout_secs: u64,
    #[arg(long)]
    assignment_worker_pubkey: Option<String>,
    #[arg(long, default_value_t = false)]
    allow_worker_seed_exposure: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
enum SearchMode {
    SecureLocal,
    DistributedInsecure,
}

#[derive(Debug, Serialize)]
struct Limits {
    max_memory_bytes: u32,
    max_instructions: u64,
}

#[derive(Debug, Serialize)]
struct JobCreateRequest {
    runtime_id: String,
    wasm_base64: String,
    input_base64: String,
    limits: Limits,
    escrow_lamports: u64,
    assignment_worker_pubkey: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JobCreateResponse {
    job_id: String,
}

#[derive(Debug, Deserialize)]
struct JobStatusResponse {
    quorum: Option<JobQuorum>,
}

#[derive(Debug, Deserialize)]
struct JobQuorum {
    quorum_reached: bool,
    winning_output_hash: Option<String>,
}

#[derive(Debug)]
enum FinalOutcome {
    Found {
        jobs_submitted: u64,
        escrow_spent_lamports: u64,
        job_id: String,
        counter: u64,
        address: String,
        pubkey_hex: String,
        keypair_hex: String,
    },
    ExhaustedRange {
        jobs_submitted: u64,
        escrow_spent_lamports: u64,
        next_counter: u64,
    },
    ExhaustedEscrow {
        jobs_submitted: u64,
        escrow_spent_lamports: u64,
        next_counter: u64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let cli = Cli::parse();
    validate_cli(&cli)?;
    let seed = parse_seed(&cli.seed_hex)?;
    let prefix = cli.prefix.into_bytes();

    let mut next_counter = cli.start_counter;
    let mut jobs_submitted = 0_u64;
    let mut escrow_spent = 0_u64;

    if cli.mode == SearchMode::SecureLocal {
        while next_counter < cli.end_counter {
            let attempts = (cli.end_counter - next_counter).min(cli.chunk_attempts);
            let req = SearchRequest {
                seed,
                start_counter: next_counter,
                max_attempts: attempts,
                prefix: prefix.clone(),
            };
            let out = execute_request(&req);
            match decode_response(&out).map_err(|e| anyhow!("{e:?}"))? {
                SearchResponse::Found(found) => {
                    let derived = derive_keypair(seed, found.counter);
                    if derived.address != found.address || derived.public_key != found.public_key {
                        return Err(anyhow!(
                            "derived key mismatch for counter {}",
                            found.counter
                        ));
                    }
                    print_outcome(&FinalOutcome::Found {
                        jobs_submitted,
                        escrow_spent_lamports: escrow_spent,
                        job_id: "local-secure-search".to_string(),
                        counter: found.counter,
                        address: found.address,
                        pubkey_hex: hex::encode(found.public_key),
                        keypair_hex: hex::encode(derived.keypair_bytes),
                    })?;
                    return Ok(());
                }
                SearchResponse::NotFound => {
                    next_counter = next_counter.saturating_add(attempts);
                }
                SearchResponse::Error(code) => {
                    return Err(anyhow!("payload returned error code: {code}"));
                }
            }
        }

        print_outcome(&FinalOutcome::ExhaustedRange {
            jobs_submitted,
            escrow_spent_lamports: escrow_spent,
            next_counter,
        })?;
        return Ok(());
    }

    let scheduler_url = cli
        .scheduler_url
        .as_deref()
        .expect("validated scheduler_url")
        .trim_end_matches('/')
        .to_string();
    let runtime_id = cli.runtime_id.clone().expect("validated runtime_id");
    let wasm_path = cli.wasm_path.clone().expect("validated wasm_path");
    let wasm = tokio::fs::read(&wasm_path)
        .await
        .with_context(|| format!("failed to read wasm_path {}", wasm_path.display()))?;
    let not_found_hash = hex::encode(edgerun_crypto::blake3_256(&encode_not_found()));
    let client = reqwest::Client::new();

    while next_counter < cli.end_counter {
        if escrow_spent
            .checked_add(cli.escrow_per_job_lamports)
            .is_none_or(|v| v > cli.max_escrow_lamports)
        {
            print_outcome(&FinalOutcome::ExhaustedEscrow {
                jobs_submitted,
                escrow_spent_lamports: escrow_spent,
                next_counter,
            })?;
            return Ok(());
        }

        let attempts = (cli.end_counter - next_counter).min(cli.chunk_attempts);
        let req = SearchRequest {
            seed,
            start_counter: next_counter,
            max_attempts: attempts,
            prefix: prefix.clone(),
        };
        let input = encode_request(&req).map_err(|e| anyhow!("failed to encode request: {e:?}"))?;

        let create_body = JobCreateRequest {
            runtime_id: runtime_id.clone(),
            wasm_base64: base64::engine::general_purpose::STANDARD.encode(&wasm),
            input_base64: base64::engine::general_purpose::STANDARD.encode(&input),
            limits: Limits {
                max_memory_bytes: cli.max_memory_bytes,
                max_instructions: cli.max_instructions,
            },
            escrow_lamports: cli.escrow_per_job_lamports,
            assignment_worker_pubkey: cli.assignment_worker_pubkey.clone(),
        };

        let create: JobCreateResponse = client
            .post(format!("{scheduler_url}/v1/job/create"))
            .json(&create_body)
            .send()
            .await
            .context("scheduler /v1/job/create request failed")?
            .error_for_status()
            .context("scheduler /v1/job/create returned error")?
            .json()
            .await
            .context("invalid /v1/job/create response")?;

        jobs_submitted += 1;
        escrow_spent += cli.escrow_per_job_lamports;

        let winning_output_hash = poll_winner_hash(
            &client,
            &scheduler_url,
            &create.job_id,
            cli.poll_interval_ms,
            cli.job_timeout_secs,
        )
        .await?;

        if winning_output_hash.eq_ignore_ascii_case(&not_found_hash) {
            next_counter = next_counter.saturating_add(attempts);
            continue;
        }

        let local_output = execute_request(&req);
        let local_hash = hex::encode(edgerun_crypto::blake3_256(&local_output));
        if !local_hash.eq_ignore_ascii_case(&winning_output_hash) {
            return Err(anyhow!(
                "winning output hash mismatch for job {}: scheduler={} local={}",
                create.job_id,
                winning_output_hash,
                local_hash
            ));
        }

        let found = match decode_response(&local_output).map_err(|e| anyhow!("{e:?}"))? {
            SearchResponse::Found(found) => found,
            SearchResponse::NotFound => {
                next_counter = next_counter.saturating_add(attempts);
                continue;
            }
            SearchResponse::Error(code) => {
                return Err(anyhow!(
                    "payload returned error code for job {}: {}",
                    create.job_id,
                    code
                ));
            }
        };

        let derived = derive_keypair(seed, found.counter);
        if derived.address != found.address || derived.public_key != found.public_key {
            return Err(anyhow!(
                "derived key mismatch for counter {}",
                found.counter
            ));
        }

        print_outcome(&FinalOutcome::Found {
            jobs_submitted,
            escrow_spent_lamports: escrow_spent,
            job_id: create.job_id,
            counter: found.counter,
            address: found.address,
            pubkey_hex: hex::encode(found.public_key),
            keypair_hex: hex::encode(derived.keypair_bytes),
        })?;
        return Ok(());
    }

    print_outcome(&FinalOutcome::ExhaustedRange {
        jobs_submitted,
        escrow_spent_lamports: escrow_spent,
        next_counter,
    })?;
    Ok(())
}

fn validate_cli(cli: &Cli) -> Result<()> {
    if cli.prefix.is_empty() {
        anyhow::bail!("prefix must not be empty");
    }
    if cli.chunk_attempts == 0 {
        anyhow::bail!("chunk_attempts must be > 0");
    }
    if cli.end_counter <= cli.start_counter {
        anyhow::bail!("end_counter must be greater than start_counter");
    }
    if cli.mode == SearchMode::DistributedInsecure {
        if !cli.allow_worker_seed_exposure {
            anyhow::bail!(
                "distributed search requires --allow-worker-seed-exposure; this mode leaks derivation seed material to workers"
            );
        }
        if cli.scheduler_url.as_deref().unwrap_or_default().is_empty() {
            anyhow::bail!("--scheduler-url is required for distributed-insecure mode");
        }
        if cli.runtime_id.as_deref().unwrap_or_default().is_empty() {
            anyhow::bail!("--runtime-id is required for distributed-insecure mode");
        }
        if cli.wasm_path.is_none() {
            anyhow::bail!("--wasm-path is required for distributed-insecure mode");
        }
        if cli.escrow_per_job_lamports == 0 {
            anyhow::bail!("escrow_per_job_lamports must be > 0");
        }
        if cli.max_escrow_lamports < cli.escrow_per_job_lamports {
            anyhow::bail!("max_escrow_lamports must be >= escrow_per_job_lamports");
        }
    }
    Ok(())
}

fn parse_seed(seed_hex: &str) -> Result<[u8; 32]> {
    let raw = seed_hex.trim();
    let bytes = hex::decode(raw).context("seed_hex must be hex")?;
    if bytes.len() != 32 {
        anyhow::bail!("seed_hex must decode to 32 bytes, got {}", bytes.len());
    }
    let mut out = [0_u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

async fn poll_winner_hash(
    client: &reqwest::Client,
    scheduler_url: &str,
    job_id: &str,
    poll_interval_ms: u64,
    timeout_secs: u64,
) -> Result<String> {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs.max(1));
    loop {
        let status: JobStatusResponse = client
            .get(format!("{scheduler_url}/v1/job/{job_id}"))
            .send()
            .await
            .context("scheduler /v1/job/{job_id} request failed")?
            .error_for_status()
            .context("scheduler /v1/job/{job_id} returned error")?
            .json()
            .await
            .context("invalid /v1/job/{job_id} response")?;

        if let Some(quorum) = status.quorum {
            if quorum.quorum_reached {
                if let Some(hash) = quorum.winning_output_hash {
                    return Ok(hash);
                }
                anyhow::bail!("job {job_id} reached quorum without winning_output_hash");
            }
        }

        if Instant::now() >= deadline {
            anyhow::bail!("job {job_id} timed out before quorum");
        }
        sleep(Duration::from_millis(poll_interval_ms.max(50))).await;
    }
}

fn print_outcome(outcome: &FinalOutcome) -> Result<()> {
    let value = match outcome {
        FinalOutcome::Found {
            jobs_submitted,
            escrow_spent_lamports,
            job_id,
            counter,
            address,
            pubkey_hex,
            keypair_hex,
        } => json::object! {
            status: "found",
            jobs_submitted: *jobs_submitted,
            escrow_spent_lamports: *escrow_spent_lamports,
            job_id: job_id.as_str(),
            counter: *counter,
            address: address.as_str(),
            pubkey_hex: pubkey_hex.as_str(),
            keypair_hex: keypair_hex.as_str(),
        },
        FinalOutcome::ExhaustedRange {
            jobs_submitted,
            escrow_spent_lamports,
            next_counter,
        } => json::object! {
            status: "exhausted_range",
            jobs_submitted: *jobs_submitted,
            escrow_spent_lamports: *escrow_spent_lamports,
            next_counter: *next_counter,
        },
        FinalOutcome::ExhaustedEscrow {
            jobs_submitted,
            escrow_spent_lamports,
            next_counter,
        } => json::object! {
            status: "exhausted_escrow",
            jobs_submitted: *jobs_submitted,
            escrow_spent_lamports: *escrow_spent_lamports,
            next_counter: *next_counter,
        },
    };
    println!("{}", json::stringify_pretty(value, 2));
    Ok(())
}
