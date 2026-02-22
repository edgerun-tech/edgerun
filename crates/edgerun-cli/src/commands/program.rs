// SPDX-License-Identifier: Apache-2.0
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};

use crate::process_helpers::{
    run_program_capture_sync_owned, run_program_sync, run_program_sync_with_env,
};
use crate::{ProgramCommand, SolanaCluster, program_tool_env};

const ACCOUNT_DISCRIMINATOR_BYTES: usize = 8;
const GLOBAL_CONFIG_INIT_SPACE: usize = 179;
const WORKER_STAKE_INIT_SPACE: usize = 49;
const JOB_INIT_SPACE: usize = 545;
const JOB_RESULT_INIT_SPACE: usize = 168;
const OUTPUT_AVAILABILITY_INIT_SPACE: usize = 232;

struct Phase2AccountSpec {
    name: &'static str,
    seed: &'static str,
    init_space: usize,
    notes: &'static str,
}

pub(crate) fn run_program_command(root: &Path, command: ProgramCommand) -> Result<()> {
    let program_root = root.join("program");
    if !program_root.is_dir() {
        return Err(anyhow!(
            "missing program workspace at {}",
            program_root.display()
        ));
    }
    let mut env = program_tool_env(&program_root);
    append_solana_path(&mut env);

    match command {
        ProgramCommand::AnalyzeAccounts { cluster } => {
            run_analyze_accounts(&program_root, cluster, &env)
        }
        ProgramCommand::Deploy {
            cluster,
            skip_build,
            final_immutable,
            program_id,
            keypair,
            fee_payer,
            max_len,
            no_update_frontend_config,
        } => run_deploy(
            root,
            &program_root,
            cluster,
            skip_build,
            final_immutable,
            program_id,
            keypair,
            fee_payer,
            max_len,
            !no_update_frontend_config,
            &env,
        ),
    }
}

fn run_analyze_accounts(
    program_root: &Path,
    cluster: SolanaCluster,
    env: &[(OsString, OsString)],
) -> Result<()> {
    let specs = phase2_account_specs();
    let artifact_path = program_root.join("target/deploy/edgerun.so");
    let artifact_size = fs::metadata(&artifact_path)
        .with_context(|| format!("missing SBF artifact: {}", artifact_path.display()))?
        .len() as usize;

    println!("Phase 2 account analysis");
    println!("cluster: {}", cluster.as_str());
    println!();
    println!(
        "{:<24} {:>6} {:>7} {:>14}  Seeds / notes",
        "Account", "Init", "Total", "Rent-exempt"
    );
    println!("{}", "-".repeat(96));

    for spec in &specs {
        let total = spec.init_space + ACCOUNT_DISCRIMINATOR_BYTES;
        let rent_sol = fetch_rent_exempt_sol(cluster, total, program_root, env)?;
        println!(
            "{:<24} {:>6} {:>7} {:>14.9}  {} ({})",
            spec.name, spec.init_space, total, rent_sol, spec.seed, spec.notes
        );
    }

    let artifact_rent_sol = fetch_rent_exempt_sol(cluster, artifact_size, program_root, env)?;
    let loader_program_account_sol = fetch_rent_exempt_sol(cluster, 36, program_root, env)?;
    println!();
    println!(
        "Program artifact: {} bytes -> {:.9} SOL rent-exempt",
        artifact_size, artifact_rent_sol
    );
    println!(
        "Upgradeable loader program account (36 bytes): {:.9} SOL",
        loader_program_account_sol
    );
    println!(
        "Practical deploy budget (artifact + overhead): {:.3} - {:.3} SOL",
        artifact_rent_sol,
        artifact_rent_sol + 0.2
    );
    println!(
        "Rule-of-thumb answer to 'does deploy cost ~4 SOL?': {}",
        if artifact_rent_sol >= 3.5 {
            "yes (current build is close to that range)"
        } else {
            "no"
        }
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_deploy(
    root: &Path,
    program_root: &Path,
    cluster: SolanaCluster,
    skip_build: bool,
    final_immutable: bool,
    program_id: Option<String>,
    keypair: Option<PathBuf>,
    fee_payer: Option<PathBuf>,
    max_len: Option<u32>,
    update_frontend_config: bool,
    env: &[(OsString, OsString)],
) -> Result<()> {
    if !skip_build {
        run_program_sync_with_env(
            "Build SBF artifact",
            "cargo-build-sbf",
            &[
                "--manifest-path",
                "programs/edgerun_program/Cargo.toml",
                "--sbf-out-dir",
                "target/deploy",
            ],
            program_root,
            false,
            env,
        )?;
    }

    let so_path = program_root.join("target/deploy/edgerun.so");
    if !so_path.is_file() {
        return Err(anyhow!("missing deploy artifact at {}", so_path.display()));
    }

    let mut args = vec![
        "program".to_string(),
        "deploy".to_string(),
        "--url".to_string(),
        cluster.as_str().to_string(),
        so_path.display().to_string(),
        "--output".to_string(),
        "json".to_string(),
    ];
    if final_immutable {
        args.push("--final".to_string());
    }
    if let Some(id) = program_id.as_ref() {
        args.push("--program-id".to_string());
        args.push(id.clone());
    }
    if let Some(path) = keypair.as_ref() {
        args.push("--keypair".to_string());
        args.push(path.display().to_string());
    }
    if let Some(path) = fee_payer.as_ref() {
        args.push("--fee-payer".to_string());
        args.push(path.display().to_string());
    }
    if let Some(value) = max_len {
        args.push("--max-len".to_string());
        args.push(value.to_string());
    }

    let output =
        run_program_capture_sync_owned("Deploy program", "solana", &args, program_root, env)?;
    let deployed_program_id = parse_program_id_from_deploy_json(&output)?;
    println!(
        "deployed program id ({}): {deployed_program_id}",
        cluster.as_str()
    );

    if update_frontend_config {
        let config_path = root.join("frontend/config/solana-deployments.json");
        if config_path.is_file() {
            let mut doc = fs::read_to_string(&config_path)
                .with_context(|| format!("failed to read {}", config_path.display()))?;
            doc = doc.replace(
                &format!("\"{}\": \"\"", cluster.as_str()),
                &format!("\"{}\": \"{}\"", cluster.as_str(), deployed_program_id),
            );
            fs::write(&config_path, doc)
                .with_context(|| format!("failed to write {}", config_path.display()))?;
            run_program_sync(
                "Format deployment config",
                "bun",
                &[
                    "x",
                    "prettier",
                    "--write",
                    "frontend/config/solana-deployments.json",
                ],
                root,
                false,
            )?;
            println!(
                "updated frontend deployment mapping for {} in {}",
                cluster.as_str(),
                config_path.display()
            );
        } else {
            println!(
                "skipped frontend deployment mapping update (missing {})",
                config_path.display()
            );
        }
    }

    Ok(())
}

fn append_solana_path(env: &mut [(OsString, OsString)]) {
    let solana_bin = std::env::var("HOME")
        .map(PathBuf::from)
        .map(|p| p.join(".local/share/solana/install/active_release/bin"))
        .ok();
    let Some(solana_bin) = solana_bin else {
        return;
    };
    if !solana_bin.is_dir() {
        return;
    }
    let Some((_, current_path)) = env.iter_mut().find(|(k, _)| k == "PATH") else {
        return;
    };
    let mut merged = vec![solana_bin];
    merged.extend(std::env::split_paths(current_path));
    if let Ok(joined) = std::env::join_paths(merged) {
        *current_path = joined;
    }
}

fn fetch_rent_exempt_sol(
    cluster: SolanaCluster,
    bytes: usize,
    program_root: &Path,
    env: &[(OsString, OsString)],
) -> Result<f64> {
    let args = vec![
        "rent".to_string(),
        "--url".to_string(),
        cluster.as_str().to_string(),
        bytes.to_string(),
    ];
    let out = run_program_capture_sync_owned(
        &format!("Rent exemption for {bytes} bytes"),
        "solana",
        &args,
        program_root,
        env,
    )?;
    parse_rent_sol(&out)
}

fn parse_rent_sol(output: &str) -> Result<f64> {
    let marker = "Rent-exempt minimum:";
    let line = output
        .lines()
        .find(|line| line.contains(marker))
        .ok_or_else(|| anyhow!("unable to parse rent output"))?;
    let value = line
        .split(marker)
        .nth(1)
        .map(str::trim)
        .and_then(|s| s.split_whitespace().next())
        .ok_or_else(|| anyhow!("unable to parse rent output value"))?;
    value
        .parse::<f64>()
        .with_context(|| format!("invalid rent value: {value}"))
}

fn parse_program_id_from_deploy_json(output: &str) -> Result<String> {
    let start = output
        .find('{')
        .ok_or_else(|| anyhow!("deploy output did not include json payload"))?;
    let payload = &output[start..];
    let value: serde_json::Value =
        serde_json::from_str(payload).context("failed to parse deploy json payload")?;
    let id = value
        .get("programId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("deploy output missing programId"))?;
    Ok(id.to_string())
}

fn phase2_account_specs() -> Vec<Phase2AccountSpec> {
    vec![
        Phase2AccountSpec {
            name: "GlobalConfig",
            seed: "PDA: [b\"config\"]",
            init_space: GLOBAL_CONFIG_INIT_SPACE,
            notes: "singleton config + phase2 tier fields",
        },
        Phase2AccountSpec {
            name: "WorkerStake",
            seed: "PDA: [b\"worker_stake\", worker]",
            init_space: WORKER_STAKE_INIT_SPACE,
            notes: "stake ledger per worker",
        },
        Phase2AccountSpec {
            name: "Job",
            seed: "PDA: [b\"job\", job_id]",
            init_space: JOB_INIT_SPACE,
            notes: "phase2 dynamic committee (up to 9)",
        },
        Phase2AccountSpec {
            name: "JobResult",
            seed: "PDA: [b\"job_result\", job_id, worker]",
            init_space: JOB_RESULT_INIT_SPACE,
            notes: "one result account per worker submit",
        },
        Phase2AccountSpec {
            name: "OutputAvailability",
            seed: "PDA: [b\"output\", job_id]",
            init_space: OUTPUT_AVAILABILITY_INIT_SPACE,
            notes: "DA pointer + publisher metadata",
        },
    ]
}
