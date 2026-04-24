use edgerun_solana::{DeploymentClient, solana_types::Pubkey};
use edgerun_solana::signers::Ed25519Signer;

pub enum DeploymentCommand {
    Create {
        deployment: String,
        owner: Option<String>,
        name: Option<String>,
        containers: u32,
        cpu_cores: u32,
        memory_bytes: u64,
        storage_bytes: u64,
        deposit: u64,
    },
    Get { deployment: String },
    Start { deployment: String, owner: Option<String> },
    Stop { deployment: String, owner: Option<String> },
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
}

pub fn parse_deployment_command() -> DeploymentCommand {
    let mut args = std::env::args();
    let _ = args.next();
    let _ = args.next();
    
    match args.next().as_deref() {
        Some("create") | Some("c") => {
            let mut deployment = None;
            let mut owner = None;
            let mut name = None;
            let mut containers = 1u32;
            let mut cpu_cores = 2u32;
            let mut memory_bytes = 4294967296u64;
            let mut storage_bytes = 5368709120u64;
            let mut deposit = 1000000000u64;
            
            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--owner" | "-o" => owner = args.next(),
                    "--name" => name = args.next(),
                    "--containers" => {
                        if let Ok(v) = args.next().unwrap_or_default().parse() { containers = v; }
                    }
                    "--cpu" | "-c" => {
                        if let Ok(v) = args.next().unwrap_or_default().parse() { cpu_cores = v; }
                    }
                    "--memory" | "-m" => {
                        if let Ok(v) = args.next().unwrap_or_default().parse() { memory_bytes = v; }
                    }
                    "--storage" | "-s" => {
                        if let Ok(v) = args.next().unwrap_or_default().parse() { storage_bytes = v; }
                    }
                    "--deposit" | "-d" => {
                        if let Ok(v) = args.next().unwrap_or_default().parse() { deposit = v; }
                    }
                    _ if !arg.starts_with('-') => deployment = Some(arg),
                    _ => {}
                }
            }
            
            DeploymentCommand::Create {
                deployment: deployment.unwrap_or_default(),
                owner,
                name,
                containers,
                cpu_cores,
                memory_bytes,
                storage_bytes,
                deposit,
            }
        }
        Some("get") | Some("g") => {
            let deployment = args.next().unwrap_or_default();
            DeploymentCommand::Get { deployment }
        }
        Some("start") => {
            let deployment = args.next().unwrap_or_default();
            let owner = args.find(|a| a.starts_with("--owner")).map(|_| args.next().unwrap_or_default());
            DeploymentCommand::Start { deployment, owner }
        }
        Some("stop") => {
            let deployment = args.next().unwrap_or_default();
            let owner = args.find(|a| a.starts_with("--owner")).map(|_| args.next().unwrap_or_default());
            DeploymentCommand::Stop { deployment, owner }
        }
        Some("report") | Some("r") => {
            let mut deployment = None;
            let mut provider = None;
            let mut cpu_cores = 0u32;
            let mut memory_bytes = 0u64;
            let mut storage_bytes = 0u64;
            let mut network_bytes = 0u64;
            let containers = 1u32;
            
            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--provider" | "-p" => provider = args.next(),
                    "--cpu" => {
                        if let Ok(v) = args.next().unwrap_or_default().parse() { cpu_cores = v; }
                    }
                    "--memory" | "-m" => {
                        if let Ok(v) = args.next().unwrap_or_default().parse() { memory_bytes = v; }
                    }
                    "--storage" | "-s" => {
                        if let Ok(v) = args.next().unwrap_or_default().parse() { storage_bytes = v; }
                    }
                    "--network" | "-n" => {
                        if let Ok(v) = args.next().unwrap_or_default().parse() { network_bytes = v; }
                    }
                    _ if !arg.starts_with('-') && deployment.is_none() => deployment = Some(arg),
                    _ => {}
                }
            }
            
            DeploymentCommand::Report {
                deployment: deployment.unwrap_or_default(),
                provider: provider.unwrap_or_default(),
                cpu_cores,
                memory_bytes,
                storage_bytes,
                network_bytes,
                containers,
            }
        }
        Some("burn-rate") | Some("burn") => {
            let mut cpu_cores = 2u32;
            let mut memory_bytes = 4294967296u64;
            let mut storage_bytes = 5368709120u64;
            let mut network_mbps = 100u32;
            
            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--cpu" | "-c" => {
                        if let Ok(v) = args.next().unwrap_or_default().parse() { cpu_cores = v; }
                    }
                    "--memory" | "-m" => {
                        if let Ok(v) = args.next().unwrap_or_default().parse() { memory_bytes = v; }
                    }
                    "--storage" | "-s" => {
                        if let Ok(v) = args.next().unwrap_or_default().parse() { storage_bytes = v; }
                    }
                    "--network" | "-n" => {
                        if let Ok(v) = args.next().unwrap_or_default().parse() { network_mbps = v; }
                    }
                    _ => {}
                }
            }
            
            DeploymentCommand::BurnRate {
                cpu_cores,
                memory_bytes,
                storage_bytes,
                network_mbps,
            }
        }
        _ => {
            eprintln!("Unknown deployment command. Use: create, get, start, stop, report, burn-rate");
            std::process::exit(1);
        }
    }
}

pub async fn handle(cmd: DeploymentCommand, rpc_url: String, signer: Option<Ed25519Signer>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = DeploymentClient::new(&rpc_url)?;

    match &cmd {
        DeploymentCommand::Create { deployment, owner, name, containers, cpu_cores, memory_bytes, storage_bytes, deposit } => {
            let owner_pubkey = match owner {
                Some(o) => o.parse().unwrap_or_default(),
                None => Pubkey::default(),
            };
            let burn_rate = DeploymentClient::calculate_burn_rate(*cpu_cores, *memory_bytes, *storage_bytes, 100);

            println!("=== Create Deployment ===");
            println!("Deployment: {}", deployment);
            println!("Owner: {}", owner_pubkey);
            if let Some(n) = name {
                println!("Name: {}", n);
            }
            println!("Containers: {}", containers);
            println!("Resources: {} cores, {} bytes RAM, {} bytes storage", cpu_cores, memory_bytes, storage_bytes);
            println!("Initial deposit: {} lamports", deposit);
            println!("Burn rate: {} lamports/sec", burn_rate);

            if let Some(ref signer) = signer {
                let dep_pubkey: Pubkey = deployment.parse().unwrap_or_default();
                let mut name_bytes = [0u8; 64];
                if let Some(ref n) = name {
                    let bytes = n.as_bytes();
                    name_bytes[..bytes.len().min(64)].copy_from_slice(&bytes[..bytes.len().min(64)]);
                }
                let ix = client.initialize_instruction(
                    &dep_pubkey, &owner_pubkey, name_bytes, [0u8; 32], *containers,
                    *cpu_cores, *memory_bytes, *storage_bytes, 100, *deposit, burn_rate
                );
                let tx_sig = client.send_instruction_signed(ix, &owner_pubkey, signer).await?;
                println!("Transaction sent: {}", tx_sig);
            } else {
                println!("\nNote: No keypair loaded. To send this transaction:");
                println!("  export SOLANA_KEYPAIR=/path/to/keypair");
            }
            Ok(())
        }
        DeploymentCommand::Get { deployment } => {
            let pubkey: Pubkey = deployment.parse().unwrap_or_default();
            match client.get_deployment(&pubkey) {
                Ok(d) => {
                    println!("=== Deployment Info ===");
                    println!("Owner: {}", d.owner);
                    println!("Provider: {}", d.provider);
                    println!("Status: {:?}", d.status);
                    println!("Resources: {} cores, {} bytes RAM, {} bytes storage",
                        d.total_cpu_cores, d.total_memory_bytes, d.total_storage_bytes);
                    println!("Containers: {}", d.container_count);
                    println!("Deposit: {} lamports", d.deposit);
                    println!("Spent: {} lamports", d.spent);
                    println!("Remaining: {} lamports", d.deposit.saturating_sub(d.spent));
                }
                Err(e) => {
                    println!("Deployment not found: {}", e);
                }
            }
            Ok(())
        }
        DeploymentCommand::Start { deployment, owner } => {
            let dep_pubkey: Pubkey = deployment.parse().unwrap_or_default();
            let owner_pubkey = match owner {
                Some(o) => o.parse().unwrap_or_default(),
                None => Pubkey::default(),
            };
            if let Some(ref signer) = signer {
                let ix = client.start_instruction(&dep_pubkey, &owner_pubkey);
                let tx_sig = client.send_instruction_signed(ix, &owner_pubkey, signer).await?;
                println!("Transaction sent: {}", tx_sig);
            } else {
                println!("Start instruction created for {}", deployment);
            }
            Ok(())
        }
        DeploymentCommand::Stop { deployment, owner } => {
            let dep_pubkey: Pubkey = deployment.parse().unwrap_or_default();
            let owner_pubkey = match owner {
                Some(o) => o.parse().unwrap_or_default(),
                None => Pubkey::default(),
            };
            if let Some(ref signer) = signer {
                let ix = client.stop_instruction(&dep_pubkey, &owner_pubkey);
                let tx_sig = client.send_instruction_signed(ix, &owner_pubkey, signer).await?;
                println!("Transaction sent: {}", tx_sig);
            } else {
                println!("Stop instruction created for {}", deployment);
            }
            Ok(())
        }
        DeploymentCommand::Report { deployment, provider, cpu_cores, memory_bytes, storage_bytes, network_bytes, containers } => {
            let dep_pubkey: Pubkey = deployment.parse().unwrap_or_default();
            let prov_pubkey: Pubkey = provider.parse().unwrap_or_default();
            println!("=== Report Metrics ===");
            println!("Deployment: {}", deployment);
            println!("Provider: {}", provider);
            println!("Metrics: {} cores, {} bytes RAM, {} bytes storage, {} bytes net",
                cpu_cores, memory_bytes, storage_bytes, network_bytes);

            if let Some(ref signer) = signer {
                let ix = client.report_metrics_instruction(
                    &dep_pubkey, &prov_pubkey, *cpu_cores, *memory_bytes, *storage_bytes, *network_bytes, *containers
                );
                let tx_sig = client.send_instruction_signed(ix, &prov_pubkey, signer).await?;
                println!("Transaction sent: {}", tx_sig);
            } else {
                println!("\nNote: No keypair loaded. To report metrics, load a keypair.");
            }
            Ok(())
        }
        DeploymentCommand::BurnRate { cpu_cores, memory_bytes, storage_bytes, network_mbps } => {
            let rate = DeploymentClient::calculate_burn_rate(*cpu_cores, *memory_bytes, *storage_bytes, *network_mbps);
            println!("=== Burn Rate ===");
            println!("Resources: {} cores, {} bytes RAM, {} bytes storage, {} Mbps",
                cpu_cores, memory_bytes, storage_bytes, network_mbps);
            println!("Per second: {} lamports", rate);
            println!("Per hour: {} lamports", rate * 3600);
            println!("Per day: {} lamports", rate * 86400);
            Ok(())
        }
    }
}