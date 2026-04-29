use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use edgerun_solana::signers::{Ed25519Signer, Signer};
use edgerun_solana::{solana_types::Pubkey, DeploymentClient, DeploymentStatus, ProviderClient};
use std::{eprintln, println};

pub enum ProviderCommand {
    Register {
        provider: String,
        authority: Option<String>,
        cpu_cores: u32,
        memory_bytes: u64,
        storage_bytes: u64,
        network_mbits: u32,
    },
    Get {
        provider: String,
    },
    List,
    Earnings {
        provider: String,
    },
    Attest {
        provider: String,
        uptime_percent_bps: u32,
    },
    Pause {
        provider: String,
    },
    Resume {
        provider: String,
    },
}

pub fn parse_provider_command() -> ProviderCommand {
    let mut args = std::env::args();
    let _ = args.next();
    let _ = args.next();

    match args.next().as_deref() {
        Some("register") | Some("r") => {
            let mut provider = None;
            let mut authority = None;
            let mut cpu_cores = 4u32;
            let mut memory_bytes = 8589934592u64;
            let mut storage_bytes = 10737418240u64;
            let mut network_mbits = 100u32;

            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--authority" | "-a" => authority = args.next(),
                    "--cpu-cores" | "-c" => {
                        if let Ok(v) = args.next().unwrap_or_default().parse() {
                            cpu_cores = v;
                        }
                    }
                    "--memory" | "-m" => {
                        if let Ok(v) = args.next().unwrap_or_default().parse() {
                            memory_bytes = v;
                        }
                    }
                    "--storage" | "-s" => {
                        if let Ok(v) = args.next().unwrap_or_default().parse() {
                            storage_bytes = v;
                        }
                    }
                    "--network" | "-n" => {
                        if let Ok(v) = args.next().unwrap_or_default().parse() {
                            network_mbits = v;
                        }
                    }
                    _ if !arg.starts_with('-') => provider = Some(arg),
                    _ => {}
                }
            }

            ProviderCommand::Register {
                provider: provider.unwrap_or_else(|| "".to_string()),
                authority,
                cpu_cores,
                memory_bytes,
                storage_bytes,
                network_mbits,
            }
        }
        Some("get") | Some("g") => {
            let provider = args.next().unwrap_or_default();
            ProviderCommand::Get { provider }
        }
        Some("list") | Some("l") => ProviderCommand::List,
        Some("earnings") | Some("e") => {
            let provider = args.next().unwrap_or_default();
            ProviderCommand::Earnings { provider }
        }
        Some("attest") | Some("a") => {
            let provider = args.next().unwrap_or_default();
            let uptime = match args.next().unwrap_or_default().parse() {
                Ok(value) => value,
                Err(_) => {
                    eprintln!("invalid uptime percent basis-points value");
                    std::process::exit(1);
                }
            };
            ProviderCommand::Attest {
                provider,
                uptime_percent_bps: uptime,
            }
        }
        Some("pause") => {
            let provider = args.next().unwrap_or_default();
            ProviderCommand::Pause { provider }
        }
        Some("resume") => {
            let provider = args.next().unwrap_or_default();
            ProviderCommand::Resume { provider }
        }
        _ => {
            eprintln!(
                "Unknown provider command. Use: register, get, list, earnings, attest, pause, resume"
            );
            std::process::exit(1);
        }
    }
}

fn parse_pubkey(
    label: &str,
    value: &str,
) -> Result<Pubkey, Box<dyn std::error::Error + Send + Sync>> {
    if value.is_empty() {
        return Err(format!("missing {label} pubkey").into());
    }
    value
        .parse()
        .map_err(|err| format!("invalid {label} pubkey '{value}': {err}").into())
}

fn signer_pubkey(signer: &Ed25519Signer) -> Pubkey {
    Pubkey::new_from_array(signer.pubkey())
}

fn print_confirmed_tx(
    client: &ProviderClient,
    tx_sig: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Transaction sent: {}", tx_sig);
    client.confirm_transaction(tx_sig)?;
    println!("Transaction confirmed: {}", tx_sig);
    Ok(())
}

pub async fn handle(
    cmd: ProviderCommand,
    rpc_url: String,
    signer: Option<Ed25519Signer>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match &cmd {
        ProviderCommand::Register {
            provider,
            authority,
            cpu_cores,
            memory_bytes,
            storage_bytes,
            network_mbits,
        } => {
            let client = ProviderClient::new(&rpc_url)?;
            let authority_pubkey = match (authority, signer.as_ref()) {
                (Some(a), _) => parse_pubkey("authority", a)?,
                (None, Some(signer)) => signer_pubkey(signer),
                (None, None) => {
                    return Err(
                        "missing authority pubkey; pass --authority or set SOLANA_KEYPAIR".into(),
                    );
                }
            };

            let min_collateral = edgerun_solana::types::collateral::calculate_minimum(
                *cpu_cores,
                *memory_bytes,
                *storage_bytes,
                *network_mbits,
            );
            println!("=== Register Provider ===");
            println!("Provider: {}", provider);
            println!("Minimum collateral: {} lamports", min_collateral);
            println!(
                "Resources: {} cores, {} bytes RAM, {} bytes storage, {} Mbps",
                cpu_cores, memory_bytes, storage_bytes, network_mbits
            );

            if let Some(ref signer) = signer {
                if provider.is_empty() || provider.parse::<Pubkey>().is_err() {
                    let seed = if provider.is_empty() {
                        "provider"
                    } else {
                        provider.as_str()
                    };
                    if authority_pubkey != signer_pubkey(signer) {
                        return Err(
                            "seeded provider creation requires signer to be the authority".into(),
                        );
                    }
                    let (provider_pubkey, tx_sig) = client
                        .register_with_seed_signed(
                            &authority_pubkey,
                            seed,
                            signer,
                            *cpu_cores,
                            *memory_bytes,
                            *storage_bytes,
                            *network_mbits,
                        )
                        .await?;
                    println!("Provider account: {}", provider_pubkey);
                    println!("Seed: {}", seed);
                    print_confirmed_tx(&client, &tx_sig)?;
                } else {
                    let provider_pubkey = parse_pubkey("provider", provider)?;
                    let ix = client.register_instruction(
                        &provider_pubkey,
                        &authority_pubkey,
                        *cpu_cores,
                        *memory_bytes,
                        *storage_bytes,
                        *network_mbits,
                    );
                    let tx_sig = client
                        .send_instruction_signed(ix, &authority_pubkey, signer)
                        .await?;
                    print_confirmed_tx(&client, &tx_sig)?;
                }
            } else {
                println!("\nNote: No keypair loaded. To send this transaction:");
                println!("  export SOLANA_KEYPAIR=/path/to/keypair");
            }
            Ok(())
        }
        ProviderCommand::Get { provider } => {
            let client = ProviderClient::new(&rpc_url)?;
            let pubkey = parse_pubkey("provider", provider)?;
            match client.get_provider(&pubkey) {
                Ok(p) => {
                    println!("=== Provider Info ===");
                    println!("Collateral: {} lamports", p.collateral_staked);
                    println!(
                        "Resources: {} cores, {} bytes RAM, {} bytes storage, {} Mbps",
                        p.cpu_cores, p.memory_bytes, p.storage_bytes, p.network_mbits
                    );
                    println!("Status: {:?}", p.status);
                    println!("Earnings: {} lamports", p.total_earnings);
                }
                Err(e) => {
                    println!("Provider not found: {}", e);
                }
            }
            Ok(())
        }
        ProviderCommand::List => {
            let client = ProviderClient::new(&rpc_url)?;
            match client.find_active_providers() {
                Ok(providers) => {
                    println!("=== Active Providers ({}) ===", providers.len());
                    for pk in providers {
                        println!("  {}", pk);
                    }
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
            Ok(())
        }
        ProviderCommand::Earnings { provider } => {
            let provider_pubkey = parse_pubkey("provider", provider)?;
            let deployment_client = DeploymentClient::new(&rpc_url)?;
            let deployments = deployment_client.list_deployments()?;
            let mut matched = 0usize;
            let mut stopped = 0usize;
            let mut provider_earned = 0u64;
            let mut active_spent = 0u64;
            let mut buyer_refunded = 0u64;
            let mut dao_slashed = 0u64;

            println!("=== Provider Deployment Earnings ===");
            println!("Provider: {}", provider_pubkey);
            for account in deployments {
                if account.deployment.provider != provider_pubkey {
                    continue;
                }
                matched += 1;
                provider_earned =
                    provider_earned.saturating_add(account.deployment.provider_earned);
                buyer_refunded = buyer_refunded.saturating_add(account.deployment.buyer_refunded);
                dao_slashed = dao_slashed.saturating_add(account.deployment.dao_slashed);
                if matches!(account.deployment.status, DeploymentStatus::Stopped) {
                    stopped += 1;
                } else {
                    active_spent = active_spent.saturating_add(account.deployment.spent);
                }
                println!(
                    "  {} {:?} earned={} spent={} refunded={} slashed={}",
                    account.pubkey,
                    account.deployment.status,
                    account.deployment.provider_earned,
                    account.deployment.spent,
                    account.deployment.buyer_refunded,
                    account.deployment.dao_slashed
                );
            }
            println!("Deployments: {}", matched);
            println!("Stopped deployments: {}", stopped);
            println!("Settled provider earnings: {} lamports", provider_earned);
            println!("Unsettled active spent: {} lamports", active_spent);
            println!("Buyer refunded: {} lamports", buyer_refunded);
            println!("DAO slashed: {} lamports", dao_slashed);
            Ok(())
        }
        ProviderCommand::Attest {
            provider,
            uptime_percent_bps,
        } => {
            let client = ProviderClient::new(&rpc_url)?;
            let pubkey = parse_pubkey("provider", provider)?;
            if *uptime_percent_bps > 10_000 {
                return Err("uptime percent basis-points must be <= 10000".into());
            }
            if let Some(ref signer) = signer {
                let authority_pubkey = signer_pubkey(signer);
                let ix = client.attest_instruction(&pubkey, &authority_pubkey, *uptime_percent_bps);
                let tx_sig = client
                    .send_instruction_signed(ix, &authority_pubkey, signer)
                    .await?;
                print_confirmed_tx(&client, &tx_sig)?;
            } else {
                println!("Attest {} ({} bps uptime)", provider, uptime_percent_bps);
            }
            Ok(())
        }
        ProviderCommand::Pause { provider } => {
            let client = ProviderClient::new(&rpc_url)?;
            let pubkey = parse_pubkey("provider", provider)?;
            if let Some(ref signer) = signer {
                let authority_pubkey = signer_pubkey(signer);
                let ix = client.pause_instruction(&pubkey, &authority_pubkey);
                let tx_sig = client
                    .send_instruction_signed(ix, &authority_pubkey, signer)
                    .await?;
                print_confirmed_tx(&client, &tx_sig)?;
            } else {
                println!("Pause instruction created for {}", provider);
            }
            Ok(())
        }
        ProviderCommand::Resume { provider } => {
            let client = ProviderClient::new(&rpc_url)?;
            let pubkey = parse_pubkey("provider", provider)?;
            if let Some(ref signer) = signer {
                let authority_pubkey = signer_pubkey(signer);
                let ix = client.resume_instruction(&pubkey, &authority_pubkey);
                let tx_sig = client
                    .send_instruction_signed(ix, &authority_pubkey, signer)
                    .await?;
                print_confirmed_tx(&client, &tx_sig)?;
            } else {
                println!("Resume instruction created for {}", provider);
            }
            Ok(())
        }
    }
}
