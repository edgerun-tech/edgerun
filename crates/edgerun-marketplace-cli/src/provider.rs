use edgerun_solana::signers::Ed25519Signer;
use edgerun_solana::{solana_types::Pubkey, ProviderClient};

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
    Attest {
        provider: String,
        uptime_seconds: u32,
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
        Some("attest") | Some("a") => {
            let provider = args.next().unwrap_or_default();
            let uptime: u32 = args.next().unwrap_or_default().parse().unwrap_or(0);
            ProviderCommand::Attest {
                provider,
                uptime_seconds: uptime,
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
            eprintln!("Unknown provider command. Use: register, get, list, attest, pause, resume");
            std::process::exit(1);
        }
    }
}

pub async fn handle(
    cmd: ProviderCommand,
    rpc_url: String,
    signer: Option<Ed25519Signer>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = ProviderClient::new(&rpc_url)?;

    match &cmd {
        ProviderCommand::Register {
            provider,
            authority,
            cpu_cores,
            memory_bytes,
            storage_bytes,
            network_mbits,
        } => {
            let provider_pubkey: Pubkey = provider.parse().unwrap_or_default();
            let authority_pubkey = match authority {
                Some(a) => a.parse().unwrap_or_default(),
                None => Pubkey::default(),
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
                println!("Transaction sent: {}", tx_sig);
            } else {
                println!("\nNote: No keypair loaded. To send this transaction:");
                println!("  export SOLANA_KEYPAIR=/path/to/keypair");
            }
            Ok(())
        }
        ProviderCommand::Get { provider } => {
            let pubkey: Pubkey = provider.parse().unwrap_or_default();
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
        ProviderCommand::Attest {
            provider,
            uptime_seconds,
        } => {
            let pubkey: Pubkey = provider.parse().unwrap_or_default();
            if let Some(ref signer) = signer {
                let ix = client.attest_instruction(&pubkey, &pubkey, *uptime_seconds);
                let tx_sig = client.send_instruction_signed(ix, &pubkey, signer).await?;
                println!("Transaction sent: {}", tx_sig);
            } else {
                println!("Attest {} ({}s uptime)", provider, uptime_seconds);
            }
            Ok(())
        }
        ProviderCommand::Pause { provider } => {
            let pubkey: Pubkey = provider.parse().unwrap_or_default();
            if let Some(ref signer) = signer {
                let ix = client.pause_instruction(&pubkey, &pubkey);
                let tx_sig = client.send_instruction_signed(ix, &pubkey, signer).await?;
                println!("Transaction sent: {}", tx_sig);
            } else {
                println!("Pause instruction created for {}", provider);
            }
            Ok(())
        }
        ProviderCommand::Resume { provider } => {
            let pubkey: Pubkey = provider.parse().unwrap_or_default();
            if let Some(ref signer) = signer {
                let ix = client.resume_instruction(&pubkey, &pubkey);
                let tx_sig = client.send_instruction_signed(ix, &pubkey, signer).await?;
                println!("Transaction sent: {}", tx_sig);
            } else {
                println!("Resume instruction created for {}", provider);
            }
            Ok(())
        }
    }
}
