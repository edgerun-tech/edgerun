use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use core::str::FromStr;
use edgerun_solana::signers::{Ed25519Signer, Signer};
use edgerun_solana::{DeploymentClient, solana_types::Pubkey};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{eprintln, println};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeploymentCommand {
    Create {
        deployment: String,
        owner: Option<String>,
        governance: Option<String>,
        provider: Option<String>,
        name: Option<String>,
        containers: u32,
        cpu_cores: u32,
        memory_bytes: u64,
        storage_bytes: u64,
        deposit: u64,
        auto_stop_on_price_increase: bool,
    },
    Get {
        deployment: String,
    },
    Start {
        deployment: String,
        owner: Option<String>,
    },
    Assign {
        deployment: String,
        provider: String,
        scheduler: Option<String>,
    },
    Pause {
        deployment: String,
        owner: Option<String>,
    },
    Resume {
        deployment: String,
        owner: Option<String>,
    },
    Dispute {
        deployment: String,
        owner: Option<String>,
    },
    Resolve {
        deployment: String,
        resolver: Option<String>,
        refund_recipient: String,
        provider_payout_recipient: String,
        slash_recipient: String,
        refund_to_buyer: u64,
        provider_payout: u64,
        slash_to_dao: u64,
    },
    Stop {
        deployment: String,
        owner: Option<String>,
        provider_payout_recipient: String,
    },
    TickBurn {
        deployment: String,
    },
    Report {
        deployment: String,
        provider: String,
        cpu_cores: u32,
        memory_bytes: u64,
        storage_bytes: u64,
        network_bytes: u64,
        containers: u32,
    },
    BurnRate {
        cpu_cores: u32,
        memory_bytes: u64,
        storage_bytes: u64,
        network_mbps: u32,
    },
    SchedulePricing {
        deployment: String,
        governance: Option<String>,
        core_hour: u64,
        ram_gib_hour: u64,
        storage_gib_hour: u64,
        network_mbit_hour: u64,
        effective_at: i64,
    },
}

pub fn parse_deployment_command() -> DeploymentCommand {
    let mut args = std::env::args();
    let _ = args.next();
    let _ = args.next();

    match parse_deployment_args(args) {
        Ok(command) => command,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}

fn parse_deployment_args<I, S>(args: I) -> Result<DeploymentCommand, String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = args.into_iter().map(Into::into);

    match args.next().as_deref() {
        Some("create") | Some("c") => {
            let mut deployment = None;
            let mut owner = None;
            let mut governance = None;
            let mut provider = None;
            let mut name = None;
            let mut containers = 1u32;
            let mut cpu_cores = 2u32;
            let mut memory_bytes = 4294967296u64;
            let mut storage_bytes = 5368709120u64;
            let mut deposit = 1000000000u64;
            let mut auto_stop_on_price_increase = true;

            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--owner" | "-o" => owner = Some(next_value(&mut args, &arg)?),
                    "--governance" | "-g" => governance = Some(next_value(&mut args, &arg)?),
                    "--provider" | "-p" => {
                        return Err(
                            "buyer-selected providers are disabled; use deployment assign after create"
                                .to_string(),
                        );
                    }
                    "--name" => name = Some(next_value(&mut args, &arg)?),
                    "--containers" => containers = next_parse(&mut args, &arg)?,
                    "--cpu" | "-c" => cpu_cores = next_parse(&mut args, &arg)?,
                    "--memory" | "-m" => memory_bytes = next_parse(&mut args, &arg)?,
                    "--storage" | "-s" => storage_bytes = next_parse(&mut args, &arg)?,
                    "--deposit" | "-d" => deposit = next_parse(&mut args, &arg)?,
                    "--keep-running-on-price-increase" => auto_stop_on_price_increase = false,
                    "--auto-stop-on-price-increase" => auto_stop_on_price_increase = true,
                    _ if !arg.starts_with('-') && deployment.is_none() => deployment = Some(arg),
                    _ if !arg.starts_with('-') => {
                        return Err(format!("unexpected extra deployment argument '{arg}'"));
                    }
                    _ => return Err(format!("unknown deployment create option '{arg}'")),
                }
            }

            Ok(DeploymentCommand::Create {
                deployment: deployment.unwrap_or_default(),
                owner,
                governance,
                provider,
                name,
                containers,
                cpu_cores,
                memory_bytes,
                storage_bytes,
                deposit,
                auto_stop_on_price_increase,
            })
        }
        Some("get") | Some("g") => {
            let deployment = args.next().unwrap_or_default();
            reject_trailing(args, "deployment get")?;
            Ok(DeploymentCommand::Get { deployment })
        }
        Some("start") => {
            let deployment = args.next().unwrap_or_default();
            let owner = parse_owner_arg(args, "deployment start")?;
            Ok(DeploymentCommand::Start { deployment, owner })
        }
        Some("assign") | Some("assign-provider") => {
            let mut deployment = None;
            let mut provider = None;
            let mut scheduler = None;

            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--provider" | "-p" => provider = Some(next_value(&mut args, &arg)?),
                    "--scheduler" | "-s" => scheduler = Some(next_value(&mut args, &arg)?),
                    _ if !arg.starts_with('-') && deployment.is_none() => deployment = Some(arg),
                    _ if !arg.starts_with('-') => {
                        return Err(format!("unexpected extra deployment argument '{arg}'"));
                    }
                    _ => return Err(format!("unknown deployment assign option '{arg}'")),
                }
            }

            Ok(DeploymentCommand::Assign {
                deployment: deployment.unwrap_or_default(),
                provider: provider.unwrap_or_default(),
                scheduler,
            })
        }
        Some("pause") => {
            let deployment = args.next().unwrap_or_default();
            let owner = parse_owner_arg(args, "deployment pause")?;
            Ok(DeploymentCommand::Pause { deployment, owner })
        }
        Some("resume") => {
            let deployment = args.next().unwrap_or_default();
            let owner = parse_owner_arg(args, "deployment resume")?;
            Ok(DeploymentCommand::Resume { deployment, owner })
        }
        Some("dispute") => {
            let deployment = args.next().unwrap_or_default();
            let owner = parse_owner_arg(args, "deployment dispute")?;
            Ok(DeploymentCommand::Dispute { deployment, owner })
        }
        Some("resolve") => {
            let mut deployment = None;
            let mut resolver = None;
            let mut refund_recipient = None;
            let mut provider_payout_recipient = None;
            let mut slash_recipient = None;
            let mut refund_to_buyer = 0u64;
            let mut provider_payout = 0u64;
            let mut slash_to_dao = 0u64;

            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--resolver" | "-r" => resolver = Some(next_value(&mut args, &arg)?),
                    "--refund-recipient" => refund_recipient = Some(next_value(&mut args, &arg)?),
                    "--provider-payout-recipient" => {
                        provider_payout_recipient = Some(next_value(&mut args, &arg)?)
                    }
                    "--slash-recipient" => slash_recipient = Some(next_value(&mut args, &arg)?),
                    "--refund" => refund_to_buyer = next_parse(&mut args, &arg)?,
                    "--provider-payout" => provider_payout = next_parse(&mut args, &arg)?,
                    "--slash" => slash_to_dao = next_parse(&mut args, &arg)?,
                    _ if !arg.starts_with('-') && deployment.is_none() => deployment = Some(arg),
                    _ if !arg.starts_with('-') => {
                        return Err(format!("unexpected extra deployment argument '{arg}'"));
                    }
                    _ => return Err(format!("unknown deployment resolve option '{arg}'")),
                }
            }

            Ok(DeploymentCommand::Resolve {
                deployment: deployment.unwrap_or_default(),
                resolver,
                refund_recipient: refund_recipient.unwrap_or_default(),
                provider_payout_recipient: provider_payout_recipient.unwrap_or_default(),
                slash_recipient: slash_recipient.unwrap_or_default(),
                refund_to_buyer,
                provider_payout,
                slash_to_dao,
            })
        }
        Some("stop") => {
            let deployment = args.next().unwrap_or_default();
            let mut owner = None;
            let mut provider_payout_recipient = None;
            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--owner" | "-o" => owner = Some(next_value(&mut args, &arg)?),
                    "--provider-payout-recipient" => {
                        provider_payout_recipient = Some(next_value(&mut args, &arg)?)
                    }
                    _ if !arg.starts_with('-') => {
                        return Err(format!("unexpected argument for deployment stop: '{arg}'"));
                    }
                    _ => return Err(format!("unknown deployment stop option '{arg}'")),
                }
            }
            Ok(DeploymentCommand::Stop {
                deployment,
                owner,
                provider_payout_recipient: provider_payout_recipient.unwrap_or_default(),
            })
        }
        Some("tick-burn") | Some("tick") => {
            let deployment = args.next().unwrap_or_default();
            reject_trailing(args, "deployment tick-burn")?;
            Ok(DeploymentCommand::TickBurn { deployment })
        }
        Some("report") | Some("r") => {
            let mut deployment = None;
            let mut provider = None;
            let mut cpu_cores = 0u32;
            let mut memory_bytes = 0u64;
            let mut storage_bytes = 0u64;
            let mut network_bytes = 0u64;
            let mut containers = 1u32;

            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--provider" | "-p" => provider = Some(next_value(&mut args, &arg)?),
                    "--cpu" => cpu_cores = next_parse(&mut args, &arg)?,
                    "--memory" | "-m" => memory_bytes = next_parse(&mut args, &arg)?,
                    "--storage" | "-s" => storage_bytes = next_parse(&mut args, &arg)?,
                    "--network" | "-n" => network_bytes = next_parse(&mut args, &arg)?,
                    "--containers" => containers = next_parse(&mut args, &arg)?,
                    _ if !arg.starts_with('-') && deployment.is_none() => deployment = Some(arg),
                    _ if !arg.starts_with('-') => {
                        return Err(format!("unexpected extra deployment argument '{arg}'"));
                    }
                    _ => return Err(format!("unknown deployment report option '{arg}'")),
                }
            }

            Ok(DeploymentCommand::Report {
                deployment: deployment.unwrap_or_default(),
                provider: provider.unwrap_or_default(),
                cpu_cores,
                memory_bytes,
                storage_bytes,
                network_bytes,
                containers,
            })
        }
        Some("burn-rate") | Some("burn") => {
            let mut cpu_cores = 2u32;
            let mut memory_bytes = 4294967296u64;
            let mut storage_bytes = 5368709120u64;
            let mut network_mbps = 100u32;

            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--cpu" | "-c" => cpu_cores = next_parse(&mut args, &arg)?,
                    "--memory" | "-m" => memory_bytes = next_parse(&mut args, &arg)?,
                    "--storage" | "-s" => storage_bytes = next_parse(&mut args, &arg)?,
                    "--network" | "-n" => network_mbps = next_parse(&mut args, &arg)?,
                    _ => return Err(format!("unknown deployment burn-rate option '{arg}'")),
                }
            }

            Ok(DeploymentCommand::BurnRate {
                cpu_cores,
                memory_bytes,
                storage_bytes,
                network_mbps,
            })
        }
        Some("schedule-pricing") | Some("price") => {
            let mut deployment = None;
            let mut governance = None;
            let mut core_hour = edgerun_solana::types::pricing::CORE_HOUR;
            let mut ram_gib_hour = edgerun_solana::types::pricing::RAM_GIB_HOUR;
            let mut storage_gib_hour = edgerun_solana::types::pricing::STORAGE_GIB_HOUR;
            let mut network_mbit_hour = edgerun_solana::types::pricing::NETWORK_MBIT_HOUR;
            let mut effective_at = default_price_effective_at();

            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--governance" | "-g" => governance = Some(next_value(&mut args, &arg)?),
                    "--core-hour" => core_hour = next_parse(&mut args, &arg)?,
                    "--ram-gib-hour" => ram_gib_hour = next_parse(&mut args, &arg)?,
                    "--storage-gib-hour" => storage_gib_hour = next_parse(&mut args, &arg)?,
                    "--network-mbit-hour" => network_mbit_hour = next_parse(&mut args, &arg)?,
                    "--effective-at" => effective_at = next_parse(&mut args, &arg)?,
                    _ if !arg.starts_with('-') && deployment.is_none() => deployment = Some(arg),
                    _ if !arg.starts_with('-') => {
                        return Err(format!("unexpected extra deployment argument '{arg}'"));
                    }
                    _ => return Err(format!("unknown deployment schedule-pricing option '{arg}'")),
                }
            }

            Ok(DeploymentCommand::SchedulePricing {
                deployment: deployment.unwrap_or_default(),
                governance,
                core_hour,
                ram_gib_hour,
                storage_gib_hour,
                network_mbit_hour,
                effective_at,
            })
        }
        _ => Err("unknown deployment command. Use: create, get, assign, start, pause, resume, dispute, resolve, stop, tick-burn, report, burn-rate, schedule-pricing".to_string()),
    }
}

fn default_price_effective_at() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64 + 86_400)
        .unwrap_or(86_400)
}

fn parse_owner_arg<I>(mut args: I, command: &str) -> Result<Option<String>, String>
where
    I: Iterator<Item = String>,
{
    let mut owner = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--owner" | "-o" => owner = Some(next_value(&mut args, &arg)?),
            _ if !arg.starts_with('-') => {
                return Err(format!("unexpected argument for {command}: '{arg}'"));
            }
            _ => return Err(format!("unknown {command} option '{arg}'")),
        }
    }
    Ok(owner)
}

fn next_value<I>(args: &mut I, flag: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    args.next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("missing value for {flag}"))
}

fn next_parse<I, T>(args: &mut I, flag: &str) -> Result<T, String>
where
    I: Iterator<Item = String>,
    T: FromStr,
{
    let value = next_value(args, flag)?;
    value
        .parse()
        .map_err(|_| format!("invalid value for {flag}: '{value}'"))
}

fn reject_trailing<I>(mut args: I, command: &str) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    match args.next() {
        Some(arg) => Err(format!("unexpected argument for {command}: '{arg}'")),
        None => Ok(()),
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
    client: &DeploymentClient,
    tx_sig: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Transaction sent: {}", tx_sig);
    client.confirm_transaction(tx_sig)?;
    println!("Transaction confirmed: {}", tx_sig);
    Ok(())
}

pub async fn handle(
    cmd: DeploymentCommand,
    rpc_url: String,
    signer: Option<Ed25519Signer>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = DeploymentClient::new(&rpc_url)?;

    match &cmd {
        DeploymentCommand::Create {
            deployment,
            owner,
            governance,
            provider,
            name,
            containers,
            cpu_cores,
            memory_bytes,
            storage_bytes,
            deposit,
            auto_stop_on_price_increase,
        } => {
            let owner_pubkey = match (owner, signer.as_ref()) {
                (Some(o), _) => parse_pubkey("owner", o)?,
                (None, Some(signer)) => signer_pubkey(signer),
                (None, None) => {
                    return Err("missing owner pubkey; pass --owner or set SOLANA_KEYPAIR".into());
                }
            };
            if provider.is_some() {
                return Err(
                    "buyer-selected providers are disabled; create unassigned deployments and use deployment assign from the scheduler".into(),
                );
            }
            let provider_pubkey = Pubkey::default();
            let governance_pubkey = match governance {
                Some(governance) => parse_pubkey("governance", governance)?,
                None => owner_pubkey,
            };
            let burn_rate = DeploymentClient::calculate_burn_rate(
                *cpu_cores,
                *memory_bytes,
                *storage_bytes,
                100,
            );

            println!("=== Create Deployment ===");
            println!("Deployment: {}", deployment);
            println!("Owner: {}", owner_pubkey);
            println!("Governance: {}", governance_pubkey);
            println!("Provider: {}", provider_pubkey);
            if let Some(n) = name {
                println!("Name: {}", n);
            }
            println!("Containers: {}", containers);
            println!(
                "Resources: {} cores, {} bytes RAM, {} bytes storage",
                cpu_cores, memory_bytes, storage_bytes
            );
            println!("Initial deposit: {} lamports", deposit);
            println!("Burn rate: {} lamports/sec", burn_rate);
            println!(
                "Auto-stop on price increase: {}",
                auto_stop_on_price_increase
            );

            if let Some(ref signer) = signer {
                let mut name_bytes = [0u8; 64];
                if let Some(ref n) = name {
                    let bytes = n.as_bytes();
                    name_bytes[..bytes.len().min(64)]
                        .copy_from_slice(&bytes[..bytes.len().min(64)]);
                }
                if deployment.is_empty() || deployment.parse::<Pubkey>().is_err() {
                    let seed = if deployment.is_empty() {
                        "deployment"
                    } else {
                        deployment.as_str()
                    };
                    if owner_pubkey != signer_pubkey(signer) {
                        return Err(
                            "seeded deployment creation requires signer to be the owner".into()
                        );
                    }
                    let (dep_pubkey, tx_sig) = client
                        .create_deployment_signed(
                            &owner_pubkey,
                            seed,
                            signer,
                            name_bytes,
                            *provider_pubkey.as_bytes(),
                            *containers,
                            *cpu_cores,
                            *memory_bytes,
                            *storage_bytes,
                            100,
                            *deposit,
                            burn_rate,
                            *auto_stop_on_price_increase,
                            *governance_pubkey.as_bytes(),
                        )
                        .await?;
                    println!("Deployment account: {}", dep_pubkey);
                    println!("Seed: {}", seed);
                    print_confirmed_tx(&client, &tx_sig)?;
                } else {
                    let dep_pubkey = parse_pubkey("deployment", deployment)?;
                    let ix = client.initialize_instruction(
                        &dep_pubkey,
                        &owner_pubkey,
                        name_bytes,
                        *provider_pubkey.as_bytes(),
                        *containers,
                        *cpu_cores,
                        *memory_bytes,
                        *storage_bytes,
                        100,
                        *deposit,
                        burn_rate,
                        *auto_stop_on_price_increase,
                        *governance_pubkey.as_bytes(),
                    );
                    let tx_sig = client
                        .send_instruction_signed(ix, &owner_pubkey, signer)
                        .await?;
                    print_confirmed_tx(&client, &tx_sig)?;
                }
            } else {
                println!("\nNote: No keypair loaded. To send this transaction:");
                println!("  export SOLANA_KEYPAIR=/path/to/keypair");
            }
            Ok(())
        }
        DeploymentCommand::Get { deployment } => {
            let pubkey = parse_pubkey("deployment", deployment)?;
            match client.get_deployment(&pubkey) {
                Ok(d) => {
                    println!("=== Deployment Info ===");
                    println!("Owner: {}", d.owner);
                    println!("Governance: {}", d.governance_authority);
                    println!("Provider: {}", d.provider);
                    println!("Status: {:?}", d.status);
                    println!(
                        "Resources: {} cores, {} bytes RAM, {} bytes storage",
                        d.total_cpu_cores, d.total_memory_bytes, d.total_storage_bytes
                    );
                    println!("Containers: {}", d.container_count);
                    println!("Deposit: {} lamports", d.deposit);
                    println!("Spent: {} lamports", d.spent);
                    println!(
                        "Remaining budget: {} lamports",
                        d.deposit.saturating_sub(d.spent)
                    );
                    println!("Provider earned: {} lamports", d.provider_earned);
                    println!("Buyer refunded: {} lamports", d.buyer_refunded);
                    println!("DAO slashed: {} lamports", d.dao_slashed);
                    if matches!(d.status, edgerun_solana::DeploymentStatus::Stopped) {
                        let settled = d
                            .provider_earned
                            .saturating_add(d.buyer_refunded)
                            .saturating_add(d.dao_slashed);
                        println!(
                            "Unsettled escrow: {} lamports",
                            d.deposit.saturating_sub(settled)
                        );
                    }
                    println!("Last report: {}", d.last_report_at);
                    println!(
                        "Current pricing: core {}, RAM GiB {}, storage GiB {}, network Mbit {} lamports/hour",
                        d.current_core_hour,
                        d.current_ram_gib_hour,
                        d.current_storage_gib_hour,
                        d.current_network_mbit_hour
                    );
                    if d.pending_price_effective_at > 0 {
                        println!(
                            "Pending pricing: core {}, RAM GiB {}, storage GiB {}, network Mbit {} lamports/hour at {}",
                            d.pending_core_hour,
                            d.pending_ram_gib_hour,
                            d.pending_storage_gib_hour,
                            d.pending_network_mbit_hour,
                            d.pending_price_effective_at
                        );
                    }
                    println!(
                        "Auto-stop on price increase: {}",
                        d.auto_stop_on_price_increase
                    );
                    println!(
                        "Usage: {} cores, {} bytes RAM, {} bytes storage, {} bytes net",
                        d.cpu_cores_used,
                        d.memory_bytes_used,
                        d.storage_bytes_used,
                        d.network_bytes_sent
                    );
                }
                Err(e) => {
                    println!("Deployment not found: {}", e);
                }
            }
            Ok(())
        }
        DeploymentCommand::Start { deployment, owner } => {
            let dep_pubkey = parse_pubkey("deployment", deployment)?;
            let owner_pubkey = match (owner, signer.as_ref()) {
                (Some(o), _) => parse_pubkey("owner", o)?,
                (None, Some(signer)) => signer_pubkey(signer),
                (None, None) => {
                    return Err("missing owner pubkey; pass --owner or set SOLANA_KEYPAIR".into());
                }
            };
            if let Some(ref signer) = signer {
                let ix = client.start_instruction(&dep_pubkey, &owner_pubkey);
                let tx_sig = client
                    .send_instruction_signed(ix, &owner_pubkey, signer)
                    .await?;
                print_confirmed_tx(&client, &tx_sig)?;
            } else {
                println!("Start instruction created for {}", deployment);
            }
            Ok(())
        }
        DeploymentCommand::Assign {
            deployment,
            provider,
            scheduler,
        } => {
            let dep_pubkey = parse_pubkey("deployment", deployment)?;
            let provider_pubkey = parse_pubkey("provider", provider)?;
            let scheduler_pubkey = match (scheduler, signer.as_ref()) {
                (Some(s), _) => parse_pubkey("scheduler", s)?,
                (None, Some(signer)) => signer_pubkey(signer),
                (None, None) => {
                    return Err(
                        "missing scheduler pubkey; pass --scheduler or set SOLANA_KEYPAIR".into(),
                    );
                }
            };

            println!("=== Assign Provider ===");
            println!("Deployment: {}", deployment);
            println!("Provider: {}", provider_pubkey);
            println!("Scheduler: {}", scheduler_pubkey);
            if let Some(ref signer) = signer {
                let tx_sig = client
                    .assign_provider_signed(
                        &dep_pubkey,
                        &scheduler_pubkey,
                        &provider_pubkey,
                        signer,
                    )
                    .await?;
                print_confirmed_tx(&client, &tx_sig)?;
            } else {
                println!("\nNote: No keypair loaded. To assign providers, load a keypair.");
            }
            Ok(())
        }
        DeploymentCommand::Pause { deployment, owner } => {
            let dep_pubkey = parse_pubkey("deployment", deployment)?;
            let owner_pubkey = match (owner, signer.as_ref()) {
                (Some(o), _) => parse_pubkey("owner", o)?,
                (None, Some(signer)) => signer_pubkey(signer),
                (None, None) => {
                    return Err("missing owner pubkey; pass --owner or set SOLANA_KEYPAIR".into());
                }
            };
            if let Some(ref signer) = signer {
                let ix = client.pause_instruction(&dep_pubkey, &owner_pubkey);
                let tx_sig = client
                    .send_instruction_signed(ix, &owner_pubkey, signer)
                    .await?;
                print_confirmed_tx(&client, &tx_sig)?;
            } else {
                println!("Pause instruction created for {}", deployment);
            }
            Ok(())
        }
        DeploymentCommand::Resume { deployment, owner } => {
            let dep_pubkey = parse_pubkey("deployment", deployment)?;
            let owner_pubkey = match (owner, signer.as_ref()) {
                (Some(o), _) => parse_pubkey("owner", o)?,
                (None, Some(signer)) => signer_pubkey(signer),
                (None, None) => {
                    return Err("missing owner pubkey; pass --owner or set SOLANA_KEYPAIR".into());
                }
            };
            if let Some(ref signer) = signer {
                let ix = client.resume_instruction(&dep_pubkey, &owner_pubkey);
                let tx_sig = client
                    .send_instruction_signed(ix, &owner_pubkey, signer)
                    .await?;
                print_confirmed_tx(&client, &tx_sig)?;
            } else {
                println!("Resume instruction created for {}", deployment);
            }
            Ok(())
        }
        DeploymentCommand::Dispute { deployment, owner } => {
            let dep_pubkey = parse_pubkey("deployment", deployment)?;
            let owner_pubkey = match (owner, signer.as_ref()) {
                (Some(o), _) => parse_pubkey("owner", o)?,
                (None, Some(signer)) => signer_pubkey(signer),
                (None, None) => {
                    return Err("missing owner pubkey; pass --owner or set SOLANA_KEYPAIR".into());
                }
            };
            if let Some(ref signer) = signer {
                let ix = client.dispute_instruction(&dep_pubkey, &owner_pubkey);
                let tx_sig = client
                    .send_instruction_signed(ix, &owner_pubkey, signer)
                    .await?;
                print_confirmed_tx(&client, &tx_sig)?;
            } else {
                println!("Dispute instruction created for {}", deployment);
            }
            Ok(())
        }
        DeploymentCommand::Resolve {
            deployment,
            resolver,
            refund_recipient,
            provider_payout_recipient,
            slash_recipient,
            refund_to_buyer,
            provider_payout,
            slash_to_dao,
        } => {
            let dep_pubkey = parse_pubkey("deployment", deployment)?;
            let resolver_pubkey = match (resolver, signer.as_ref()) {
                (Some(r), _) => parse_pubkey("resolver", r)?,
                (None, Some(signer)) => signer_pubkey(signer),
                (None, None) => {
                    return Err(
                        "missing resolver pubkey; pass --resolver or set SOLANA_KEYPAIR".into(),
                    );
                }
            };
            let refund_pubkey = parse_pubkey("refund recipient", refund_recipient)?;
            let provider_payout_pubkey =
                parse_pubkey("provider payout recipient", provider_payout_recipient)?;
            let slash_pubkey = parse_pubkey("slash recipient", slash_recipient)?;
            if let Some(ref signer) = signer {
                let ix = client.resolve_instruction(
                    &dep_pubkey,
                    &resolver_pubkey,
                    &refund_pubkey,
                    &provider_payout_pubkey,
                    &slash_pubkey,
                    *refund_to_buyer,
                    *provider_payout,
                    *slash_to_dao,
                );
                let tx_sig = client
                    .send_instruction_signed(ix, &resolver_pubkey, signer)
                    .await?;
                print_confirmed_tx(&client, &tx_sig)?;
            } else {
                println!(
                    "Resolve instruction created for {} (refund {}, provider payout {}, slash {})",
                    deployment, refund_to_buyer, provider_payout, slash_to_dao
                );
            }
            Ok(())
        }
        DeploymentCommand::Stop {
            deployment,
            owner,
            provider_payout_recipient,
        } => {
            let dep_pubkey = parse_pubkey("deployment", deployment)?;
            let owner_pubkey = match (owner, signer.as_ref()) {
                (Some(o), _) => parse_pubkey("owner", o)?,
                (None, Some(signer)) => signer_pubkey(signer),
                (None, None) => {
                    return Err("missing owner pubkey; pass --owner or set SOLANA_KEYPAIR".into());
                }
            };
            let provider_payout_pubkey =
                parse_pubkey("provider payout recipient", provider_payout_recipient)?;
            if let Some(ref signer) = signer {
                let ix =
                    client.stop_instruction(&dep_pubkey, &owner_pubkey, &provider_payout_pubkey);
                let tx_sig = client
                    .send_instruction_signed(ix, &owner_pubkey, signer)
                    .await?;
                print_confirmed_tx(&client, &tx_sig)?;
            } else {
                println!("Stop instruction created for {}", deployment);
            }
            Ok(())
        }
        DeploymentCommand::TickBurn { deployment } => {
            let dep_pubkey = parse_pubkey("deployment", deployment)?;
            let ix = client.tick_burn_instruction(&dep_pubkey);
            match signer.as_ref() {
                Some(signer) => {
                    let payer = signer_pubkey(signer);
                    let tx_sig = client.send_instruction_signed(ix, &payer, signer).await?;
                    print_confirmed_tx(&client, &tx_sig)?;
                }
                None => println!("Tick-burn instruction created for {}", deployment),
            }
            Ok(())
        }
        DeploymentCommand::Report {
            deployment,
            provider,
            cpu_cores,
            memory_bytes,
            storage_bytes,
            network_bytes,
            containers,
        } => {
            let dep_pubkey = parse_pubkey("deployment", deployment)?;
            let prov_pubkey = parse_pubkey("provider", provider)?;
            println!("=== Report Metrics ===");
            println!("Deployment: {}", deployment);
            println!("Provider: {}", provider);
            println!(
                "Metrics: {} cores, {} bytes RAM, {} bytes storage, {} bytes net",
                cpu_cores, memory_bytes, storage_bytes, network_bytes
            );

            if let Some(ref signer) = signer {
                let provider_authority = signer_pubkey(signer);
                let ix = client.report_metrics_instruction(
                    &dep_pubkey,
                    &prov_pubkey,
                    &provider_authority,
                    *cpu_cores,
                    *memory_bytes,
                    *storage_bytes,
                    *network_bytes,
                    *containers,
                );
                let tx_sig = client
                    .send_instruction_signed(ix, &provider_authority, signer)
                    .await?;
                print_confirmed_tx(&client, &tx_sig)?;
            } else {
                println!("\nNote: No keypair loaded. To report metrics, load a keypair.");
            }
            Ok(())
        }
        DeploymentCommand::BurnRate {
            cpu_cores,
            memory_bytes,
            storage_bytes,
            network_mbps,
        } => {
            let rate = DeploymentClient::calculate_burn_rate(
                *cpu_cores,
                *memory_bytes,
                *storage_bytes,
                *network_mbps,
            );
            println!("=== Burn Rate ===");
            println!(
                "Resources: {} cores, {} bytes RAM, {} bytes storage, {} Mbps",
                cpu_cores, memory_bytes, storage_bytes, network_mbps
            );
            println!("Per second: {} lamports", rate);
            println!("Per hour: {} lamports", rate * 3600);
            println!("Per day: {} lamports", rate * 86400);
            Ok(())
        }
        DeploymentCommand::SchedulePricing {
            deployment,
            governance,
            core_hour,
            ram_gib_hour,
            storage_gib_hour,
            network_mbit_hour,
            effective_at,
        } => {
            let dep_pubkey = parse_pubkey("deployment", deployment)?;
            let governance_pubkey = match (governance, signer.as_ref()) {
                (Some(g), _) => parse_pubkey("governance", g)?,
                (None, Some(signer)) => signer_pubkey(signer),
                (None, None) => {
                    return Err(
                        "missing governance pubkey; pass --governance or set SOLANA_KEYPAIR".into(),
                    );
                }
            };
            println!("=== Schedule Pricing ===");
            println!("Deployment: {}", deployment);
            println!("Governance: {}", governance_pubkey);
            println!(
                "Prices: core {}, RAM GiB {}, storage GiB {}, network Mbit {} lamports/hour",
                core_hour, ram_gib_hour, storage_gib_hour, network_mbit_hour
            );
            println!("Effective at: {}", effective_at);
            if let Some(ref signer) = signer {
                let ix = client.schedule_pricing_instruction(
                    &dep_pubkey,
                    &governance_pubkey,
                    *core_hour,
                    *ram_gib_hour,
                    *storage_gib_hour,
                    *network_mbit_hour,
                    *effective_at,
                );
                let tx_sig = client
                    .send_instruction_signed(ix, &governance_pubkey, signer)
                    .await?;
                print_confirmed_tx(&client, &tx_sig)?;
            } else {
                println!("\nNote: No keypair loaded. To schedule pricing, load a keypair.");
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn parses_create_deployment_options() {
        let command = parse_deployment_args(vec![
            "create",
            "deployment-seed",
            "--owner",
            "owner-key",
            "--governance",
            "governance-key",
            "--name",
            "web",
            "--containers",
            "3",
            "--cpu",
            "8",
            "--memory",
            "17179869184",
            "--storage",
            "34359738368",
            "--deposit",
            "2000000000",
            "--keep-running-on-price-increase",
        ])
        .unwrap();

        assert_eq!(
            command,
            DeploymentCommand::Create {
                deployment: "deployment-seed".to_string(),
                owner: Some("owner-key".to_string()),
                governance: Some("governance-key".to_string()),
                provider: None,
                name: Some("web".to_string()),
                containers: 3,
                cpu_cores: 8,
                memory_bytes: 17_179_869_184,
                storage_bytes: 34_359_738_368,
                deposit: 2_000_000_000,
                auto_stop_on_price_increase: false,
            }
        );
    }

    #[test]
    fn rejects_invalid_create_capacity() {
        let err = parse_deployment_args(vec!["create", "--cpu", "many"]).unwrap_err();
        assert!(err.contains("invalid value for --cpu"));
    }

    #[test]
    fn rejects_buyer_selected_provider_on_create() {
        let err = parse_deployment_args(vec![
            "create",
            "deployment-key",
            "--provider",
            "provider-key",
        ])
        .unwrap_err();
        assert!(err.contains("buyer-selected providers are disabled"));
    }

    #[test]
    fn parses_assign_provider() {
        let command = parse_deployment_args(vec![
            "assign",
            "deployment-key",
            "--provider",
            "provider-key",
            "--scheduler",
            "scheduler-key",
        ])
        .unwrap();

        assert_eq!(
            command,
            DeploymentCommand::Assign {
                deployment: "deployment-key".to_string(),
                provider: "provider-key".to_string(),
                scheduler: Some("scheduler-key".to_string()),
            }
        );
    }

    #[test]
    fn rejects_missing_owner_value() {
        let err = parse_deployment_args(vec!["start", "deployment-key", "--owner"]).unwrap_err();
        assert!(err.contains("missing value for --owner"));
    }

    #[test]
    fn rejects_trailing_get_argument() {
        let err = parse_deployment_args(vec!["get", "deployment-key", "extra"]).unwrap_err();
        assert!(err.contains("unexpected argument for deployment get"));
    }

    #[test]
    fn parses_report_containers() {
        let command = parse_deployment_args(vec![
            "report",
            "deployment-key",
            "--provider",
            "provider-key",
            "--cpu",
            "2",
            "--memory",
            "4096",
            "--storage",
            "8192",
            "--network",
            "16384",
            "--containers",
            "4",
        ])
        .unwrap();

        assert_eq!(
            command,
            DeploymentCommand::Report {
                deployment: "deployment-key".to_string(),
                provider: "provider-key".to_string(),
                cpu_cores: 2,
                memory_bytes: 4096,
                storage_bytes: 8192,
                network_bytes: 16_384,
                containers: 4,
            }
        );
    }

    #[test]
    fn parses_schedule_pricing() {
        let command = parse_deployment_args(vec![
            "schedule-pricing",
            "deployment-key",
            "--governance",
            "governance-key",
            "--core-hour",
            "10",
            "--ram-gib-hour",
            "20",
            "--storage-gib-hour",
            "30",
            "--network-mbit-hour",
            "40",
            "--effective-at",
            "12345",
        ])
        .unwrap();

        assert_eq!(
            command,
            DeploymentCommand::SchedulePricing {
                deployment: "deployment-key".to_string(),
                governance: Some("governance-key".to_string()),
                core_hour: 10,
                ram_gib_hour: 20,
                storage_gib_hour: 30,
                network_mbit_hour: 40,
                effective_at: 12345,
            }
        );
    }
}
