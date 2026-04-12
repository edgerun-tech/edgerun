//! edgerun-net — Unified DNS + DHCP server.
//!
//! One binary replaces dnsmasq, BIND, CoreDNS, ISC DHCP, Kea, and TFTP servers.
//!
//! # Quick Start
//!
//! ```bash
//! # Create config
//! cat > /etc/edgerun/net.yaml << 'EOF'
//! apiVersion: edgerun.tech/v1alpha1
//! kind: DnsZone
//! metadata:
//!   name: my-domain
//! spec:
//!   origin: mydomain.local
//!   soa:
//!     mname: ns1.mydomain.local
//!     rname: admin.mydomain.local
//!   records:
//!     - name: "@"
//!       type: NS
//!       value: ns1.mydomain.local
//! ---
//! apiVersion: edgerun.tech/v1alpha1
//! kind: DhcpPool
//! metadata:
//!   name: main-pool
//! spec:
//!   name: main-pool
//!   range_start: 192.168.1.100
//!   range_end: 192.168.1.200
//!   subnet_mask: 255.255.255.0
//! ---
//! apiVersion: edgerun.tech/v1alpha1
//! kind: DhcpServer
//! metadata:
//!   name: main
//! spec:
//!   interface: eth0
//!   pools: [main-pool]
//!   router: 192.168.1.1
//!   dns_servers: ["192.168.1.1"]
//!   domain_name: mydomain.local
//! EOF
//!
//! # Start the server
//! edgerun-net serve --config /etc/edgerun/net.yaml
//! ```
//!
//! # Hot-Reload
//!
//! ```bash
//! edgerun-net serve --config /etc/edgerun/net.yaml --hot-reload
//! ```

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};

/// edgerun-net — Unified DNS + DHCP server
#[derive(Parser, Debug)]
#[command(name = "edgerun-net")]
#[command(about = "One binary replaces dnsmasq + BIND + CoreDNS + ISC DHCP + Kea")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start all DNS and DHCP servers
    Serve {
        /// Path to K8s-style YAML config file
        #[arg(short, long, default_value = "/etc/edgerun/net.yaml")]
        config: PathBuf,

        /// Watch config files and reload on changes
        #[arg(long)]
        hot_reload: bool,

        /// Run in foreground (default)
        #[arg(long, default_value = "true")]
        foreground: bool,
    },

    /// Validate a config file without starting services
    Validate {
        /// Path to config file to validate
        #[arg(short, long)]
        config: PathBuf,
    },

    /// Convert dnsmasq.conf to edgerun YAML
    ImportDnsmasq {
        /// Path to dnsmasq.conf
        #[arg(short, long)]
        input: PathBuf,

        /// Output YAML file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Convert CoreDNS Corefile to edgerun YAML
    ImportCorefile {
        /// Path to Corefile
        #[arg(short, long)]
        input: PathBuf,

        /// Output YAML file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() {
    edgerun_log::init_from_env();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { config, hot_reload, foreground } => {
            let config_path = config.to_string_lossy().to_string();

            edgerun_log::info!("edgerun-net v{} starting", env!("CARGO_PKG_VERSION"));
            edgerun_log::info!("edgerun-net: config: {}", config_path);
            edgerun_log::info!("edgerun-net: hot-reload: {}", hot_reload);

            let server = edgerun_net::NetServer::new(&config_path, hot_reload);

            match server.run() {
                Ok(()) => {}
                Err(e) => {
                    edgerun_log::error!("edgerun-net: fatal error: {}", e);
                    process::exit(1);
                }
            }
        }

        Commands::Validate { config } => {
            let config_text = match std::fs::read_to_string(&config) {
                Ok(t) => t,
                Err(e) => {
                    edgerun_log::error!("edgerun-net: failed to read {}: {}", config.display(), e);
                    process::exit(1);
                }
            };

            match edgerun_config::parse_and_validate(&config_text) {
                Ok(state) => {
                    edgerun_log::info!("edgerun-net: config is valid!");
                    edgerun_log::info!("  DNS zones:    {}", state.dns_zones.len());
                    edgerun_log::info!("  DNS servers:  {}", state.dns_servers.len());
                    edgerun_log::info!("  DHCP servers: {}", state.dhcp_servers.len());
                    edgerun_log::info!("  DHCP pools:   {}", state.dhcp_pools.len());
                    edgerun_log::info!("  TFTP servers: {}", state.tftp_servers.len());
                }
                Err(e) => {
                    edgerun_log::error!("edgerun-net: config validation failed: {}", e);
                    process::exit(1);
                }
            }
        }

        Commands::ImportDnsmasq { input, output } => {
            let conf = match std::fs::read_to_string(&input) {
                Ok(t) => t,
                Err(e) => {
                    edgerun_log::error!("edgerun-net: failed to read {}: {}", input.display(), e);
                    process::exit(1);
                }
            };

            match edgerun_config::import_dnsmasq(&conf) {
                Ok(resources) => {
                    let yaml = edgerun_config::to_yaml_all(&resources)
                        .expect("failed to serialize YAML");
                    if let Some(out) = output {
                        std::fs::write(&out, yaml).expect("failed to write output");
                        edgerun_log::info!("edgerun-net: converted {} resources → {}", resources.len(), out.display());
                    } else {
                        println!("{}", yaml);
                    }
                }
                Err(e) => {
                    edgerun_log::error!("edgerun-net: import failed: {}", e);
                    process::exit(1);
                }
            }
        }

        Commands::ImportCorefile { input, output } => {
            let corefile = match std::fs::read_to_string(&input) {
                Ok(t) => t,
                Err(e) => {
                    edgerun_log::error!("edgerun-net: failed to read {}: {}", input.display(), e);
                    process::exit(1);
                }
            };

            match edgerun_config::import_corefile(&corefile) {
                Ok(resources) => {
                    let yaml = edgerun_config::to_yaml_all(&resources)
                        .expect("failed to serialize YAML");
                    if let Some(out) = output {
                        std::fs::write(&out, yaml).expect("failed to write output");
                        edgerun_log::info!("edgerun-net: converted {} resources → {}", resources.len(), out.display());
                    } else {
                        println!("{}", yaml);
                    }
                }
                Err(e) => {
                    edgerun_log::error!("edgerun-net: import failed: {}", e);
                    process::exit(1);
                }
            }
        }
    }
}
