use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use core::str::FromStr;
use edgerun_solana::signers::{Ed25519Signer, Signer};
use edgerun_solana::{solana_types::Pubkey, DeploymentClient, DeploymentStatus, ProviderClient};
use std::{eprintln, println};

#[derive(Debug, Clone, PartialEq, Eq)]
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

    match parse_provider_args(args) {
        Ok(command) => command,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}

fn parse_provider_args<I, S>(args: I) -> Result<ProviderCommand, String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = args.into_iter().map(Into::into);

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
                    "--authority" | "-a" => authority = Some(next_value(&mut args, &arg)?),
                    "--cpu-cores" | "-c" => cpu_cores = next_parse(&mut args, &arg)?,
                    "--memory" | "-m" => memory_bytes = next_parse(&mut args, &arg)?,
                    "--storage" | "-s" => storage_bytes = next_parse(&mut args, &arg)?,
                    "--network" | "-n" => network_mbits = next_parse(&mut args, &arg)?,
                    _ if !arg.starts_with('-') && provider.is_none() => provider = Some(arg),
                    _ if !arg.starts_with('-') => {
                        return Err(format!("unexpected extra provider argument '{arg}'"));
                    }
                    _ => return Err(format!("unknown provider register option '{arg}'")),
                }
            }

            Ok(ProviderCommand::Register {
                provider: provider.unwrap_or_else(|| "".to_string()),
                authority,
                cpu_cores,
                memory_bytes,
                storage_bytes,
                network_mbits,
            })
        }
        Some("get") | Some("g") => {
            let provider = args.next().unwrap_or_default();
            reject_trailing(args, "provider get")?;
            Ok(ProviderCommand::Get { provider })
        }
        Some("list") | Some("l") => {
            reject_trailing(args, "provider list")?;
            Ok(ProviderCommand::List)
        }
        Some("earnings") | Some("e") => {
            let provider = args.next().unwrap_or_default();
            reject_trailing(args, "provider earnings")?;
            Ok(ProviderCommand::Earnings { provider })
        }
        Some("attest") | Some("a") => {
            let provider = args.next().unwrap_or_default();
            let uptime = next_parse(&mut args, "uptime percent basis-points")?;
            reject_trailing(args, "provider attest")?;
            Ok(ProviderCommand::Attest {
                provider,
                uptime_percent_bps: uptime,
            })
        }
        Some("pause") => {
            let provider = args.next().unwrap_or_default();
            reject_trailing(args, "provider pause")?;
            Ok(ProviderCommand::Pause { provider })
        }
        Some("resume") => {
            let provider = args.next().unwrap_or_default();
            reject_trailing(args, "provider resume")?;
            Ok(ProviderCommand::Resume { provider })
        }
        _ => Err(
            "unknown provider command. Use: register, get, list, earnings, attest, pause, resume"
                .to_string(),
        ),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn parses_register_provider_options() {
        let command = parse_provider_args(vec![
            "register",
            "provider-seed",
            "--authority",
            "authority-key",
            "--cpu-cores",
            "8",
            "--memory",
            "17179869184",
            "--storage",
            "34359738368",
            "--network",
            "1000",
        ])
        .unwrap();

        assert_eq!(
            command,
            ProviderCommand::Register {
                provider: "provider-seed".to_string(),
                authority: Some("authority-key".to_string()),
                cpu_cores: 8,
                memory_bytes: 17_179_869_184,
                storage_bytes: 34_359_738_368,
                network_mbits: 1000,
            }
        );
    }

    #[test]
    fn rejects_invalid_register_capacity() {
        let err = parse_provider_args(vec!["register", "--cpu-cores", "many"]).unwrap_err();
        assert!(err.contains("invalid value for --cpu-cores"));
    }

    #[test]
    fn rejects_missing_register_option_value() {
        let err = parse_provider_args(vec!["register", "--memory"]).unwrap_err();
        assert!(err.contains("missing value for --memory"));
    }

    #[test]
    fn parses_attest_uptime() {
        let command = parse_provider_args(vec!["attest", "provider-key", "9999"]).unwrap();
        assert_eq!(
            command,
            ProviderCommand::Attest {
                provider: "provider-key".to_string(),
                uptime_percent_bps: 9999,
            }
        );
    }

    #[test]
    fn rejects_trailing_get_argument() {
        let err = parse_provider_args(vec!["get", "provider-key", "extra"]).unwrap_err();
        assert!(err.contains("unexpected argument for provider get"));
    }
}
