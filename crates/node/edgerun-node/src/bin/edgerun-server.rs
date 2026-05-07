//! Lightweight edgerun-server binary with compiled-in bootstrap policy.
//!
//! No YAML parsing. The deployed binary is single-use: controller identity,
//! initial domains, mailboxes, website routes, cert names, and storage paths are
//! compiled into `compiled_deployment.rs`.
//!
//! Services:
//! - DNS authoritative for compiled domains (port 53 UDP+TCP)
//! - SMTP (port 25) with Maildir storage, outbound relay, DKIM signing
//! - IMAP (port 143) sharing Maildir root with SMTP
//! - HTTP (port 80) for MTA-STS and basic compiled website routes
//! - ACME client provisions Let's Encrypt certs via DNS-01 challenge

#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

#[cfg(not(target_os = "none"))]
extern crate std;

mod compiled_deployment;

use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use compiled_deployment::{CompiledDeployment, DEPLOYMENT};
use edgerun_acme::{AccountKey, AcmeClient, AcmeConfig, Dns01Challenge};
use edgerun_node::rt::{sleep, CancellationToken, Runtime};
use edgerun_node::runtime::{MemoryRuntimeStorage, RuntimeKernel, RuntimeServicePlan};
use edgerun_node::services::dns_runtime::{
    DnsRuntime as DnsServer, DnsRuntimeConfig as DnsServerConfig,
};
use edgerun_node::services::{ImapConfig, NodeRuntime, SmtpConfig};
use edgerun_protocols::dns::{record::DnsRecordType, DnsZone};
use edgerun_protocols::email_auth::sign::DkimSigner;
use edgerun_protocols::sign_p256::P256ProtocolSigner;
use edgerun_tls::certificate::Certificate;
use edgerun_tls::{generate_csr, signing_key_to_pem, CertificateAndKey};

#[cfg(feature = "derived-db")]
mod host_derived_db {
    use super::CompiledDeployment;
    use std::path::Path;

    pub fn initialize(deployment: &CompiledDeployment) -> Result<(), edgerun_derived_db::DbError> {
        edgerun_derived_db::initialize_runtime_database(
            Path::new(deployment.derived_db_path),
            deployment.policy.node_label,
            deployment.origin,
            unix_now(),
        )
    }

    fn unix_now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}

// ===========================================================================
// DNS SOA config
// ===========================================================================

const SOA_SERIAL: u32 = 2026050401;
const SOA_REFRESH: u32 = 3600;
const SOA_RETRY: u32 = 900;
const SOA_EXPIRE: u32 = 604800;
const SOA_MINIMUM: u32 = 86400;

// The private DKIM key stays on disk. The TXT value is public and can be
// replaced by the bootstrap-builder once DKIM key generation is automated.
const FALLBACK_DKIM_TXT: &str = "v=DKIM1; k=rsa; p=MIIBCgKCAQEAy+Enfug7AYUY+u5InnNMGM39BJsmkqkZFpx5HAUe7ffAPBJhVDIJI4OfcewD+04at3W8aXBx/ZYXxPVO2twqj5nIuKpJVAAeKCaJUuMCyciUtZ89bG61zHFtimMekY4YdSRUwN05Ukq1QfSKFz5FF0E7q7/+KlKATLb3lTtzMJK4olNIDj7EnzW2b9W9PIyyzsnjp/YRSL/u6YlZRWkAT62I8AqnpbegdXMt89aWJMkXF7cfR2TJCJb73qU/ACFYSd6asGWfsCFsDh3dtp8lwpJFwmDY2kUlv6HdN0Hp9NuiX+Ck7z+rrsH9YhVlKlQTtna++bjg2XVgS1rE0o0ogQIDAQAB";

// ===========================================================================
// CLI
// ===========================================================================

#[derive(Debug)]
enum Mode {
    Run,
    HealthCheck,
    SendSystemReport,
    PrintCompiledPlan,
}

fn parse_args() -> (Mode, Option<String>) {
    let args: Vec<String> = std::env::args().collect();
    let mut mode = Mode::Run;
    let mut config_path: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--health-check" => mode = Mode::HealthCheck,
            "--send-system-report" => mode = Mode::SendSystemReport,
            "--print-compiled-plan" => mode = Mode::PrintCompiledPlan,
            "--config" => {
                // Backward compatible no-op. The deploy binary must not read YAML.
                if i + 1 < args.len() {
                    config_path = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    (mode, config_path)
}

// ===========================================================================
// Compiled deployment helpers
// ===========================================================================

fn ip_addr(deployment: &CompiledDeployment) -> std::net::Ipv4Addr {
    let ip = deployment.public_ipv4;
    std::net::Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3])
}

fn zone_name(origin: &str, fqdn: &str) -> String {
    let origin = origin.trim_end_matches('.');
    let fqdn = fqdn.trim_end_matches('.');
    if fqdn.eq_ignore_ascii_case(origin) {
        "@".to_string()
    } else if let Some(prefix) = fqdn.strip_suffix(&format!(".{origin}")) {
        prefix.to_string()
    } else {
        fqdn.to_string()
    }
}

fn first_mailbox_user(deployment: &CompiledDeployment) -> String {
    for domain in deployment.domains {
        if let Some(mailbox) = domain.mailboxes.first() {
            if let Some((local, _)) = mailbox.address.split_once('@') {
                return local.to_string();
            }
        }
    }
    "postmaster".to_string()
}

// ===========================================================================
// DNS zone builder
// ===========================================================================

fn build_dns_zone(deployment: &CompiledDeployment, dkim_txt: Option<&str>) -> DnsZone {
    let mut zone = DnsZone::new(deployment.origin);
    zone.set_default_ttl(3600);

    let soa_mname = format!("ns1.{}", deployment.origin);
    let soa_rname = format!("admin.{}", deployment.origin);
    zone.set_soa(
        &soa_mname,
        &soa_rname,
        SOA_SERIAL,
        SOA_REFRESH,
        SOA_RETRY,
        SOA_EXPIRE,
        SOA_MINIMUM,
    );

    let ip = ip_addr(deployment);
    let ns1 = format!("ns1.{}", deployment.origin);
    let ns2 = format!("ns2.{}", deployment.origin);
    zone.add_ns(&ns1);
    zone.add_ns(&ns2);
    zone.add_a(&zone_name(deployment.origin, &ns1), ip, 3600);
    zone.add_a(&zone_name(deployment.origin, &ns2), ip, 3600);

    for domain in deployment.domains {
        if domain.authoritative_dns {
            zone.add_caa(
                &zone_name(deployment.origin, domain.domain),
                false,
                "issue",
                "letsencrypt.org",
                3600,
            );
            zone.add_caa(
                &zone_name(deployment.origin, domain.domain),
                false,
                "iodef",
                &format!("mailto:admin@{}", deployment.origin),
                3600,
            );
        }

        if domain.mail_enabled() {
            let mail_host = deployment.mail_host_for_domain(domain.domain);
            zone.add_a(&zone_name(deployment.origin, &mail_host), ip, 3600);
            zone.add_mx(
                &zone_name(deployment.origin, domain.domain),
                0,
                &mail_host,
                3600,
            );
            zone.add_txt(
                &zone_name(deployment.origin, domain.domain),
                "v=spf1 mx -all",
                3600,
            );
            zone.add_txt(
                &zone_name(deployment.origin, &format!("_dmarc.{}", domain.domain)),
                &format!(
                    "v=DMARC1; p=quarantine; rua=mailto:dmarc-reports@{}",
                    deployment.origin
                ),
                3600,
            );
            zone.add_txt(
                &zone_name(deployment.origin, &format!("_mta-sts.{}", domain.domain)),
                &format!("v=STSv1; id={SOA_SERIAL}"),
                3600,
            );
            zone.add_txt(
                &zone_name(deployment.origin, &format!("_smtp._tls.{}", domain.domain)),
                &format!("v=TLSRPTv1; rua=mailto:tls-reports@{}", deployment.origin),
                3600,
            );
            if let Some(dkim_txt) = dkim_txt {
                zone.add_txt(
                    &zone_name(
                        deployment.origin,
                        &format!("{}._domainkey.{}", deployment.dkim_selector, domain.domain),
                    ),
                    dkim_txt,
                    86400,
                );
            }
        }

        if let Some(site) = domain.website {
            zone.add_a(&zone_name(deployment.origin, site.domain), ip, 3600);
        }
    }

    for (name, target) in deployment.external_cnames {
        zone.add_cname(&zone_name(deployment.origin, name), target, 3600);
    }

    zone
}

// ===========================================================================
// DKIM key loader
// ===========================================================================

fn load_dkim_signer(deployment: &CompiledDeployment) -> Option<DkimSigner> {
    match fs::read_to_string(deployment.dkim_key_path) {
        Ok(pem) => match DkimSigner::from_private_key_pem(
            deployment.dkim_domain,
            deployment.dkim_selector,
            &pem,
        ) {
            Ok(signer) => {
                edgerun_node::node_info!("DKIM signer loaded from {}", deployment.dkim_key_path);
                Some(signer)
            }
            Err(e) => {
                edgerun_node::node_warn!("Failed to parse DKIM key: {}", e);
                None
            }
        },
        Err(_) => match DkimSigner::generate(deployment.dkim_domain, deployment.dkim_selector) {
            Ok(signer) => {
                if let Some(parent) = Path::new(deployment.dkim_key_path).parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        edgerun_node::node_warn!("Failed to create DKIM key directory: {}", e);
                        return Some(signer);
                    }
                }
                match signer.private_key_pem() {
                    Ok(pem) => {
                        if let Err(e) = fs::write(deployment.dkim_key_path, pem) {
                            edgerun_node::node_warn!("Failed to save generated DKIM key: {}", e);
                        } else {
                            #[cfg(unix)]
                            {
                                use std::os::unix::fs::PermissionsExt;
                                let _ = fs::set_permissions(
                                    deployment.dkim_key_path,
                                    fs::Permissions::from_mode(0o600),
                                );
                            }
                            edgerun_node::node_info!(
                                "Generated DKIM key at {}; publishing matching DNS record",
                                deployment.dkim_key_path
                            );
                        }
                    }
                    Err(e) => {
                        edgerun_node::node_warn!("Failed to encode generated DKIM key: {}", e)
                    }
                }
                Some(signer)
            }
            Err(e) => {
                edgerun_node::node_warn!("Failed to generate DKIM key: {}", e);
                None
            }
        },
    }
}

// ===========================================================================
// TLS certificate loader
// ===========================================================================

fn load_tls_cert(deployment: &CompiledDeployment) -> Option<CertificateAndKey> {
    if !Path::new(deployment.tls_cert_path).exists() || !Path::new(deployment.tls_key_path).exists()
    {
        edgerun_node::node_info!(
            "TLS certs not found at {} / {}, running without TLS",
            deployment.tls_cert_path,
            deployment.tls_key_path
        );
        return None;
    }

    let cert_pem = match fs::read_to_string(deployment.tls_cert_path) {
        Ok(p) => p,
        Err(e) => {
            edgerun_node::node_warn!("Failed to read TLS cert: {}", e);
            return None;
        }
    };
    let key_pem = match fs::read_to_string(deployment.tls_key_path) {
        Ok(p) => p,
        Err(e) => {
            edgerun_node::node_warn!("Failed to read TLS key: {}", e);
            return None;
        }
    };

    let combined = format!("{}\n{}", cert_pem, key_pem);
    match CertificateAndKey::from_pem(&combined) {
        Ok(cert) => {
            edgerun_node::node_info!("TLS certificate loaded from {}", deployment.tls_cert_path);
            Some(cert)
        }
        Err(e) => {
            edgerun_node::node_warn!("Failed to parse TLS cert: {}", e);
            None
        }
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

struct RuntimeBootstrapIdentity {
    node_id: [u8; 64],
}

fn ensure_runtime_bootstrap(
    deployment: &CompiledDeployment,
) -> Result<RuntimeBootstrapIdentity, Box<dyn std::error::Error + Send + Sync>> {
    let runtime_root = Path::new(deployment.runtime_root);
    let identity_dir = runtime_root.join("identity");
    let stream_dir = runtime_root.join("streams");
    fs::create_dir_all(&identity_dir)?;
    fs::create_dir_all(&stream_dir)?;

    let private_key_path = identity_dir.join("node-private-key.pem");
    let public_key_path = identity_dir.join("node-public-key.hex");
    let signing_key = if private_key_path.exists() {
        let pem = fs::read_to_string(&private_key_path)?;
        edgerun_crypto::p256_signing_key_from_pem(&pem)
            .ok_or("failed to parse runtime node private key")?
    } else {
        let (key, identity) = edgerun_protocols::keygen::generate_node_signing_key();
        let pem = edgerun_crypto::p256_signing_key_to_pem(&key);
        write_private_file(&private_key_path, pem.as_bytes())?;
        fs::write(&public_key_path, hex_bytes(&identity.node_id))?;
        key
    };

    let signer = P256ProtocolSigner::new(signing_key);
    let node_id = signer.raw_public_key();
    if !public_key_path.exists() {
        fs::write(&public_key_path, hex_bytes(&node_id))?;
    }

    let node_stream_dir = stream_dir.join(hex_bytes(&node_id));
    fs::create_dir_all(&node_stream_dir)?;
    let genesis_path = node_stream_dir.join("00000000000000000000.event.rkyv");
    if !genesis_path.exists() {
        let mut event = edgerun_stream::genesis_event(&node_id, unix_now_ms());
        edgerun_stream::sign_event(&mut event, &signer)
            .map_err(|err| format!("failed to sign runtime genesis event: {err}"))?;
        let event_bytes =
            edgerun_protocols::core_protocol::wire_stream::event_full_wire_bytes(&event);
        let tmp = node_stream_dir.join("00000000000000000000.event.rkyv.tmp");
        fs::write(&tmp, &event_bytes)?;
        fs::rename(&tmp, &genesis_path)?;
        fs::write(
            node_stream_dir.join("00000000000000000000.event.sha256"),
            hex_bytes(&edgerun_crypto::sha256(&event_bytes)),
        )?;
    }

    Ok(RuntimeBootstrapIdentity { node_id })
}

#[cfg(unix)]
fn write_private_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true).mode(0o600);
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

#[cfg(not(unix))]
fn write_private_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    fs::write(path, bytes)
}

fn unix_now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

fn unix_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

// ===========================================================================
// ACME provisioning loop — DNS-01 challenge
// ===========================================================================

async fn provision_certs(
    client: &AcmeClient,
    account_key: &AccountKey,
    dns_server: &DnsServer,
    base_zone: &mut DnsZone,
    domains: &[String],
) -> Result<(), edgerun_acme::AcmeError> {
    edgerun_node::node_info!("ACME: creating order for {:?}", domains);
    let order = client.create_order(domains).await?;

    let mut active_challenges: Vec<(String, Dns01Challenge)> = Vec::new();

    for authz_url in order.authorization_urls() {
        let authz = client.get_authorization(authz_url).await?;
        edgerun_node::node_info!("ACME: authorization status = {:?}", authz.status);

        let challenges = authz
            .challenges
            .as_ref()
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let dns_challenge = challenges
            .iter()
            .find(|c| matches!(c.challenge_type, edgerun_acme::types::ChallengeType::Dns01));
        let Some(challenge) = dns_challenge else {
            edgerun_node::node_warn!("ACME: no dns-01 challenge found");
            continue;
        };

        let token = challenge.token.as_deref().unwrap_or("");
        let domain = authz.identifier.value.clone();
        let dns_ch = Dns01Challenge::new(&domain, token, &account_key.thumbprint_b64());
        edgerun_node::node_info!("ACME: adding TXT record {}", dns_ch.record_name());

        base_zone.add_txt(dns_ch.record_name(), dns_ch.record_value(), 60);
        dns_server.add_zone(base_zone.clone()).await;
        active_challenges.push((domain.clone(), dns_ch));

        let validated = client.validate_challenge(&challenge.url).await?;
        edgerun_node::node_info!(
            "ACME: challenge validated, status = {:?}",
            validated.status()
        );

        let mut attempts = 0;
        while attempts < 30 {
            sleep(Duration::from_secs(2)).await;
            let status = client.get_challenge(&challenge.url).await?;
            edgerun_node::node_info!("ACME: challenge status = {:?}", status.status());
            if status.is_valid() {
                break;
            }
            if status.status() == edgerun_acme::ChallengeStatus::Invalid {
                edgerun_node::node_error!("ACME: DNS-01 challenge failed for {}", domain);
                return Err(edgerun_acme::AcmeError::ChallengeFailed(format!(
                    "DNS-01 challenge failed for {}",
                    domain
                )));
            }
            attempts += 1;
        }
    }

    for (_, ch) in &active_challenges {
        base_zone.remove_record(ch.record_name(), DnsRecordType::TXT);
    }
    if !active_challenges.is_empty() {
        dns_server.add_zone(base_zone.clone()).await;
        edgerun_node::node_info!("ACME: cleaned up DNS-01 TXT records");
    }

    let order_url = order.inner.id.clone();
    let mut order = client.get_order(&order_url).await?;
    edgerun_node::node_info!("ACME: order status = {:?}", order.status());

    if order.is_ready() || order.is_pending() {
        let Some(finalize_url) = order.finalize_url().cloned() else {
            return Err(edgerun_acme::AcmeError::OrderInvalid(
                "order missing finalize URL".into(),
            ));
        };

        let domain_refs: Vec<&str> = domains.iter().map(String::as_str).collect();
        let (csr_der, signing_key) = generate_csr(&domain_refs)
            .map_err(|e| edgerun_acme::AcmeError::Storage(e.to_string()))?;
        let key_pem = signing_key_to_pem(&signing_key)
            .map_err(|e| edgerun_acme::AcmeError::Storage(e.to_string()))?;

        edgerun_node::node_info!("ACME: finalizing order with CSR");
        order = client.finalize_order(&finalize_url, &csr_der).await?;

        let mut attempts = 0;
        while order.is_processing() || order.is_ready() {
            sleep(Duration::from_secs(2)).await;
            order = client.get_order(&order_url).await?;
            edgerun_node::node_info!("ACME: finalized order status = {:?}", order.status());
            attempts += 1;
            if attempts >= 30 {
                return Err(edgerun_acme::AcmeError::OrderInvalid(
                    "order did not finish processing".into(),
                ));
            }
        }

        if order.is_valid() {
            if let Some(parent) = Path::new(DEPLOYMENT.tls_key_path).parent() {
                fs::create_dir_all(parent).ok();
            }
            fs::write(DEPLOYMENT.tls_key_path, key_pem)
                .map_err(|e| edgerun_acme::AcmeError::Storage(e.to_string()))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ =
                    fs::set_permissions(DEPLOYMENT.tls_key_path, fs::Permissions::from_mode(0o600));
            }
            edgerun_node::node_info!("ACME: private key saved to {}", DEPLOYMENT.tls_key_path);
        }
    }

    if order.is_valid() {
        if let Some(cert_url) = order.certificate_url() {
            edgerun_node::node_info!("ACME: downloading certificate");
            let cert_pem = client.download_certificate(cert_url).await?;

            if let Some(parent) = Path::new(DEPLOYMENT.tls_cert_path).parent() {
                fs::create_dir_all(parent).ok();
            }
            fs::write(DEPLOYMENT.tls_cert_path, &cert_pem)
                .map_err(|e| edgerun_acme::AcmeError::Storage(e.to_string()))?;

            edgerun_node::node_info!("ACME: certificate saved to {}", DEPLOYMENT.tls_cert_path);
            return Ok(());
        }
    }

    Err(edgerun_acme::AcmeError::OrderInvalid(
        "order not ready".into(),
    ))
}

async fn acme_loop(
    client: AcmeClient,
    account_key: AccountKey,
    dns_server: DnsServer,
    mut base_zone: DnsZone,
    domains: Vec<String>,
    shutdown: CancellationToken,
) {
    edgerun_node::node_info!("ACME: starting DNS-01 provisioning loop for {:?}", domains);

    loop {
        if shutdown.is_cancelled() {
            break;
        }

        if let Ok(Some(cert_info)) = check_existing_certs(&DEPLOYMENT) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let days_left = cert_info.expires_at.saturating_sub(now) / 86400;
            if days_left > 30 {
                edgerun_node::node_info!(
                    "ACME: certs valid for {} more days, sleeping 12h",
                    days_left
                );
                for _ in 0..72 {
                    if shutdown.is_cancelled() {
                        break;
                    }
                    sleep(Duration::from_secs(600)).await;
                }
                continue;
            }
            edgerun_node::node_info!("ACME: certs expire in {} days, renewing", days_left);
        }

        match provision_certs(&client, &account_key, &dns_server, &mut base_zone, &domains).await {
            Ok(()) => edgerun_node::node_info!("ACME: certificates provisioned successfully"),
            Err(e) => edgerun_node::node_error!("ACME: provisioning failed: {}", e),
        }

        for _ in 0..72 {
            if shutdown.is_cancelled() {
                break;
            }
            sleep(Duration::from_secs(600)).await;
        }
    }

    edgerun_node::node_info!("ACME: loop shut down");
}

struct ExistingCertInfo {
    expires_at: u64,
}

fn check_existing_certs(
    deployment: &CompiledDeployment,
) -> Result<Option<ExistingCertInfo>, String> {
    if !Path::new(deployment.tls_cert_path).exists() {
        return Ok(None);
    }

    let pem = fs::read_to_string(deployment.tls_cert_path).map_err(|e| e.to_string())?;
    let cert = Certificate::from_pem(&pem)?;
    Ok(Some(ExistingCertInfo {
        expires_at: cert.not_after,
    }))
}

// ===========================================================================
// Health check
// ===========================================================================

fn run_health_check() -> ExitCode {
    use std::net::TcpStream;

    let mut ok = true;
    let checks = [
        ("SMTP port 25", "0.0.0.0:25"),
        ("IMAP port 143", "0.0.0.0:143"),
        ("DNS port 53", "0.0.0.0:53"),
        ("HTTP port 80", "0.0.0.0:80"),
    ];

    for (name, addr) in &checks {
        match TcpStream::connect_timeout(&addr.parse().unwrap(), Duration::from_secs(5)) {
            Ok(_) => println!("  [OK] {}", name),
            Err(e) => {
                println!("  [FAIL] {} ({})", name, e);
                ok = false;
            }
        }
    }

    if Path::new(DEPLOYMENT.tls_cert_path).exists() {
        match fs::read_to_string(DEPLOYMENT.tls_cert_path) {
            Ok(pem) => match Certificate::from_pem(&pem) {
                Ok(cert) => {
                    if cert.is_valid_at_unix_secs(unix_now_secs()) {
                        println!("  [OK] TLS certificate is valid");
                    } else {
                        println!("  [FAIL] TLS certificate is not currently valid");
                        ok = false;
                    }
                }
                Err(e) => {
                    println!("  [FAIL] TLS certificate parse error: {}", e);
                    ok = false;
                }
            },
            Err(e) => println!("  [WARN] Cannot read TLS cert: {}", e),
        }
    } else {
        println!("  [WARN] TLS certificate not yet provisioned (ACME pending)");
    }

    if ok {
        println!("Health check: all OK");
        ExitCode::SUCCESS
    } else {
        println!("Health check: some checks failed");
        ExitCode::FAILURE
    }
}

// ===========================================================================
// System report / compiled plan
// ===========================================================================

fn run_send_system_report() -> ExitCode {
    let user = first_mailbox_user(&DEPLOYMENT);
    let report = format!(
        "From: server-report@{}\r\n\
         To: {}@{}\r\n\
         Subject: Edgerun Server Report\r\n\
         Date: {}\r\n\
         \r\n\
         Edgerun NodeRuntime System Report\r\n\
         ===========================\r\n\
         \r\n\
         Node label: {}\r\n\
         Hostname: {}\r\n\
         Origin: {}\r\n\
         Controller: {}\r\n\
         TLS cert: {}\r\n\
         DKIM key: {}\r\n\
         Maildir: {}\r\n\
         Queue: {}\r\n\
         ",
        DEPLOYMENT.origin,
        user,
        DEPLOYMENT.origin,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        DEPLOYMENT.policy.node_label,
        DEPLOYMENT.hostname,
        DEPLOYMENT.origin,
        hex_bytes(&DEPLOYMENT.policy.controller_id),
        if Path::new(DEPLOYMENT.tls_cert_path).exists() {
            "present"
        } else {
            "not provisioned"
        },
        if Path::new(DEPLOYMENT.dkim_key_path).exists() {
            "present"
        } else {
            "missing"
        },
        DEPLOYMENT.maildir_root,
        DEPLOYMENT.queue_data_root,
    );

    let maildir_new = format!("{}/{}/new", DEPLOYMENT.maildir_root, user);
    fs::create_dir_all(&maildir_new).ok();

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let filename = format!("{0}.S={0},R=0", timestamp);
    let filepath = format!("{}/{}", maildir_new, filename);

    match fs::write(&filepath, report.as_bytes()) {
        Ok(()) => {
            println!("Report delivered to {}", filepath);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Failed to write report: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn print_compiled_plan() -> ExitCode {
    let runtime_config = DEPLOYMENT.to_wire_config();
    let service_plan = RuntimeServicePlan::from_deployment(&runtime_config);
    println!("node_label={}", DEPLOYMENT.policy.node_label);
    println!("origin={}", DEPLOYMENT.origin);
    println!("hostname={}", DEPLOYMENT.hostname);
    println!("controller={}", hex_bytes(&DEPLOYMENT.policy.controller_id));
    println!("runtime_root={}", DEPLOYMENT.runtime_root);
    println!("derived_db_path={}", DEPLOYMENT.derived_db_path);
    println!(
        "public_ipv4={}.{}.{}.{}",
        DEPLOYMENT.public_ipv4[0],
        DEPLOYMENT.public_ipv4[1],
        DEPLOYMENT.public_ipv4[2],
        DEPLOYMENT.public_ipv4[3]
    );
    println!("local_mail_domains={:?}", DEPLOYMENT.local_mail_domains());
    println!("certificate_domains={:?}", DEPLOYMENT.certificate_domains());
    println!("runtime_listeners={}", service_plan.listeners.len());
    println!("runtime_requires_dns={}", service_plan.requires_dns());
    println!("runtime_requires_acme={}", service_plan.requires_acme());
    for domain in DEPLOYMENT.domains {
        println!(
            "domain={} authoritative_dns={} mail_enabled={}",
            domain.domain,
            domain.authoritative_dns,
            domain.mail_enabled()
        );
        for mailbox in domain.mailboxes {
            println!("  mailbox={} target={}", mailbox.address, mailbox.target);
        }
        for alias in domain.aliases {
            println!("  alias={} target={}", alias.address, alias.target);
        }
        if let Some(site) = domain.website {
            println!(
                "  website={} repo={} ref={} path={}",
                site.domain, site.repo, site.commit, site.path
            );
        }
    }
    ExitCode::SUCCESS
}

// ===========================================================================
// Main
// ===========================================================================

fn main() -> ExitCode {
    edgerun_node::logging::install_stderr_logger();

    let (mode, _config_path) = parse_args();

    match mode {
        Mode::HealthCheck => return run_health_check(),
        Mode::SendSystemReport => return run_send_system_report(),
        Mode::PrintCompiledPlan => return print_compiled_plan(),
        Mode::Run => {}
    }

    edgerun_node::node_info!(
        "edgerun-server starting on {} with controller {}",
        DEPLOYMENT.hostname,
        hex_bytes(&DEPLOYMENT.policy.controller_id)
    );

    let rt = Runtime::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build runtime");

    rt.block_on(async {
        if let Err(e) = run_server().await {
            edgerun_node::node_error!("server error: {}", e);
            return ExitCode::FAILURE;
        }
        ExitCode::SUCCESS
    })
}

async fn run_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let shutdown = CancellationToken::new();
    let bootstrap_identity = ensure_runtime_bootstrap(&DEPLOYMENT)?;
    edgerun_node::node_info!(
        "runtime bootstrap identity ready: node_id={}",
        hex_bytes(&bootstrap_identity.node_id)
    );
    let runtime_config = DEPLOYMENT.to_wire_config();
    let hardware_inventory = edgerun_node::hardware::HardwareInventory::discover();
    let provider_developer_id = edgerun_node::runtime::sha256(&bootstrap_identity.node_id);
    let service_plan =
        hardware_inventory.runtime_service_plan(&runtime_config, provider_developer_id);
    edgerun_node::node_info!(
        "runtime service plan: listeners={} dns={} acme={} mail_domains={} apps={} hardware_capabilities={}",
        service_plan.listeners.len(),
        service_plan.requires_dns(),
        service_plan.requires_acme(),
        service_plan.mail_domains().len(),
        service_plan.apps.len(),
        hardware_inventory.capability_descriptors.len()
    );
    let mut runtime_kernel =
        RuntimeKernel::new(MemoryRuntimeStorage::default(), service_plan.runtime_id);
    let installed_apps =
        runtime_kernel.install_service_plan_apps(&service_plan, unix_now_ms() as u64)?;
    edgerun_node::node_info!(
        "runtime app install events committed: apps={} events={}",
        installed_apps,
        runtime_kernel.events().len()
    );

    let dkim_signer = load_dkim_signer(&DEPLOYMENT);
    let tls_cert = load_tls_cert(&DEPLOYMENT);

    let dkim_txt = dkim_signer.as_ref().map(|signer| signer.public_key_txt());
    let base_zone = build_dns_zone(&DEPLOYMENT, dkim_txt.as_deref());
    edgerun_node::node_info!("DNS zone built for {}", DEPLOYMENT.origin);

    #[cfg(feature = "derived-db")]
    {
        host_derived_db::initialize(&DEPLOYMENT)?;
        edgerun_node::node_info!("derived database ready at {}", DEPLOYMENT.derived_db_path);
    }

    let dns_config = DnsServerConfig {
        bind_addr: "0.0.0.0:53".to_string(),
        default_ttl: 3600,
        rate_limit_qps: 100,
        bind_addr_ipv6: None,
    };
    let dns_server = DnsServer::new(dns_config)?;
    dns_server.add_zone(base_zone.clone()).await;

    let dns_server_for_run = dns_server.clone();
    let dns_shutdown = shutdown.clone();
    let dns_task = edgerun_node::rt::spawn(async move {
        let _ = dns_server_for_run.run(dns_shutdown).await;
    });

    let smtp_config = SmtpConfig {
        bind_addr: "0.0.0.0:25".to_string(),
        domain_name: DEPLOYMENT.hostname.to_string(),
        max_message_size: 35_882_577,
        smtps: false,
        starttls: tls_cert.is_some(),
        local_domains: DEPLOYMENT.local_mail_domains(),
        queue_data_root: Some(std::path::PathBuf::from(DEPLOYMENT.queue_data_root)),
        relay_dns_server: None,
        maildir_root: Some(std::path::PathBuf::from(DEPLOYMENT.maildir_root)),
        dkim_domain: dkim_signer
            .as_ref()
            .map(|_| DEPLOYMENT.dkim_domain.to_string()),
        dkim_selector: dkim_signer
            .as_ref()
            .map(|_| DEPLOYMENT.dkim_selector.to_string()),
        dkim_key_path: dkim_signer
            .as_ref()
            .map(|_| std::path::PathBuf::from(DEPLOYMENT.dkim_key_path)),
        tls_cert: tls_cert.clone(),
    };

    let imap_config = ImapConfig {
        bind_addr: "0.0.0.0:143".to_string(),
        domain_name: DEPLOYMENT.hostname.to_string(),
        imaps: false,
        maildir_root: Some(std::path::PathBuf::from(DEPLOYMENT.maildir_root)),
        tls_cert: tls_cert.clone(),
    };

    let http_app_id = edgerun_node::runtime::sha256(b"compiled-http-app");
    let mut server = NodeRuntime::new()
        .with_http_app("0.0.0.0:80", http_app_id)
        .with_smtp(smtp_config)
        .with_imap(imap_config);
    let mut bound = server.build().await?;

    let server_shutdown = shutdown.clone();
    let server_task = edgerun_node::rt::spawn(async move {
        let _ = bound.run(server_shutdown).await;
    });

    let acme_account = load_or_create_acme_account(&DEPLOYMENT);
    edgerun_node::node_info!("ACME account loaded: {}", acme_account.is_some());
    if let Some(account_key) = acme_account {
        let acme_config = AcmeConfig {
            directory_url: edgerun_acme::DirectoryUrl::LetsEncrypt,
            email: vec![DEPLOYMENT.acme_contact.to_string()],
            terms_of_service_agreed: true,
        };
        let acme_client = AcmeClient::new(acme_config, account_key.clone());
        let dns_server_for_acme = dns_server.clone();
        let base_zone_for_acme = base_zone.clone();
        let shutdown_for_acme = shutdown.clone();
        let domains = DEPLOYMENT.certificate_domains();
        edgerun_node::rt::spawn(async move {
            edgerun_node::node_info!("ACME: starting background init");
            let init_start = std::time::Instant::now();
            match acme_client.init().await {
                Ok(()) => {
                    edgerun_node::node_info!(
                        "ACME: directory fetched in {:?}",
                        init_start.elapsed()
                    );
                    acme_loop(
                        acme_client,
                        account_key,
                        dns_server_for_acme,
                        base_zone_for_acme,
                        domains,
                        shutdown_for_acme,
                    )
                    .await;
                }
                Err(e) => {
                    edgerun_node::node_warn!(
                        "ACME init failed after {:?}: {}",
                        init_start.elapsed(),
                        e
                    )
                }
            }
        });
    }

    let shutdown_clone = shutdown.clone();
    let signal_task = edgerun_node::rt::spawn(async move {
        wait_for_signal().await;
        shutdown_clone.cancel();
    });

    edgerun_node::node_info!("All services started. Press Ctrl-C to stop.");

    let _ = signal_task.await;

    shutdown.cancel();
    dns_server.shutdown().await;
    let _ = server_task.await;
    let _ = dns_task.await;

    edgerun_node::node_info!("edgerun-server stopped");
    Ok(())
}

fn load_or_create_acme_account(deployment: &CompiledDeployment) -> Option<AccountKey> {
    if Path::new(deployment.acme_account_key_path).exists() {
        match fs::read_to_string(deployment.acme_account_key_path) {
            Ok(pem) => match AccountKey::from_pem(&pem) {
                Ok(key) => {
                    edgerun_node::node_info!(
                        "ACME account key loaded from {}",
                        deployment.acme_account_key_path
                    );
                    return Some(key);
                }
                Err(e) => edgerun_node::node_warn!("Failed to parse ACME account key: {}", e),
            },
            Err(e) => edgerun_node::node_warn!("Failed to read ACME account key: {}", e),
        }
    }

    let key = AccountKey::generate();
    if let Some(parent) = Path::new(deployment.acme_account_key_path).parent() {
        fs::create_dir_all(parent).ok();
    }
    if let Err(e) = fs::write(deployment.acme_account_key_path, key.pem()) {
        edgerun_node::node_warn!("Failed to save ACME account key: {}", e);
    } else {
        edgerun_node::node_info!(
            "ACME account key generated and saved to {}",
            deployment.acme_account_key_path
        );
    }
    Some(key)
}

#[cfg(not(target_os = "none"))]
async fn wait_for_signal() {
    let ctrl_c = edgerun_node::rt::ctrl_c();
    ctrl_c.await;
    edgerun_node::node_info!("Received Ctrl-C, shutting down...");
}

#[cfg(target_os = "none")]
async fn wait_for_signal() {
    loop {
        edgerun_node::rt::sleep(Duration::from_secs(60)).await;
    }
}
