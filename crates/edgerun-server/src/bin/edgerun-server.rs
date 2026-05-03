//! Lightweight edgerun-server binary — hardcoded config for edgerun.tech.
//!
//! No YAML parsing. Config matches deploy/server/*.yaml files.
//!
//! Services:
//! - DNS authoritative for edgerun.tech (port 53 UDP+TCP)
//! - SMTP (port 25, 587) with Maildir storage, outbound relay, DKIM signing
//! - IMAP (port 143) sharing Maildir root with SMTP
//! - HTTP (port 80) for MTA-STS policy
//! - ACME client provisions Let's Encrypt certs via DNS-01 challenge in background

#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

#[cfg(not(target_os = "none"))]
extern crate std;

use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use edgerun_acme::{AcmeClient, AcmeConfig, AccountKey};
use edgerun_acme::DnsChallenge;
use edgerun_dns::server::{DnsServer, DnsServerConfig};
use edgerun_dns::zone::DnsZone;
use edgerun_email_auth::sign::DkimSigner;
use edgerun_http::server::HttpServer;
use edgerun_http::{Handler, Request, Response, StatusCode, into_handler_async};
use edgerun_rt::{CancellationToken, Runtime, sleep};
use edgerun_server::{ImapConfig, Server, SmtpConfig};
use edgerun_tls::CertificateAndKey;
use edgerun_tls::certificate::Certificate;

// ===========================================================================
// Hardcoded deployment config — matches deploy/server/*.yaml
// ===========================================================================

const SERVER_IP: [u8; 4] = [172, 245, 67, 49];
const HOSTNAME: &str = "mail.edgerun.tech";
const ORIGIN: &str = "edgerun.tech";

// Maildir and queue paths
const MAILDIR_ROOT: &str = "/var/lib/edgerun/mail/maildirs";
const QUEUE_DATA_ROOT: &str = "/var/lib/edgerun/mail/queue";

// DKIM config
const DKIM_DOMAIN: &str = "edgerun.tech";
const DKIM_SELECTOR: &str = "mail";
const DKIM_KEY_PATH: &str = "/etc/edgerun/server/dkim-mail.private.pem";

// TLS paths (provisioned by ACME or manually placed)
const TLS_CERT_PATH: &str = "/etc/edgerun/server/tls/fullchain.pem";
const TLS_KEY_PATH: &str = "/etc/edgerun/server/tls/privkey.pem";

// ACME account key path (generated on first run)
const ACME_ACCOUNT_KEY_PATH: &str = "/etc/edgerun/server/acme-account.pem";

// User credentials
const USER_USERNAME: &str = "ken";
const USER_PASSWORD: &str = "CHANGE_ME_ON_SERVER";

// DNS SOA config
const SOA_MNAME: &str = "ns1.edgerun.tech";
const SOA_RNAME: &str = "admin.edgerun.tech";
const SOA_SERIAL: u32 = 2026050301;
const SOA_REFRESH: u32 = 3600;
const SOA_RETRY: u32 = 900;
const SOA_EXPIRE: u32 = 604800;
const SOA_MINIMUM: u32 = 86400;

// ACME config
const ACME_CONTACT: &str = "ken@edgerun.tech";
const ACME_DOMAINS: &[&str] = &["mail.edgerun.tech", "mta-sts.edgerun.tech", "dash.edgerun.tech"];

// ===========================================================================
// CLI
// ===========================================================================

#[derive(Debug)]
enum Mode {
    Run,
    HealthCheck,
    SendSystemReport,
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
            "--config" => {
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
// DNS zone builder
// ===========================================================================

fn build_dns_zone() -> DnsZone {
    let mut zone = DnsZone::new(ORIGIN);
    zone.set_default_ttl(3600);

    zone.set_soa(SOA_MNAME, SOA_RNAME, SOA_SERIAL, SOA_REFRESH, SOA_RETRY, SOA_EXPIRE, SOA_MINIMUM);

    zone.add_ns("ns1.edgerun.tech");
    zone.add_ns("ns2.edgerun.tech");

    let ip = SERVER_IP;
    zone.add_a("@", edgerun_dns::std::net::Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]), 3600);
    zone.add_a("mail", edgerun_dns::std::net::Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]), 3600);
    zone.add_a("mta-sts", edgerun_dns::std::net::Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]), 3600);
    zone.add_a("blog", edgerun_dns::std::net::Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]), 3600);
    zone.add_a("git", edgerun_dns::std::net::Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]), 3600);
    zone.add_a("dash", edgerun_dns::std::net::Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]), 3600);
    zone.add_a("ns1", edgerun_dns::std::net::Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]), 3600);
    zone.add_a("ns2", edgerun_dns::std::net::Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]), 3600);

    zone.add_cname("blog", "sylchi.github.io", 3600);

    zone.add_mx("@", 0, "mail.edgerun.tech", 3600);
    zone.add_mx("nodes", 0, "mail.edgerun.tech", 3600);

    zone.add_txt("@", "v=spf1 mx -all", 3600);
    zone.add_txt("nodes", "v=spf1 -all", 3600);
    zone.add_txt("_dmarc", "v=DMARC1; p=quarantine; rua=mailto:dmarc-reports@edgerun.tech", 3600);
    zone.add_txt("_mta-sts", "v=STSv1; id=2026050301", 3600);
    zone.add_txt("_smtp._tls", "v=TLSRPTv1; rua=mailto:tls-reports@edgerun.tech", 3600);
    zone.add_txt("default._bimi", "v=BIMI1; l=https://mail.edgerun.tech/bimi/logo.svg", 3600);

    zone.add_txt("mail._domainkey",
        "v=DKIM1; k=rsa; p=MIIBCgKCAQEAy+Enfug7AYUY+u5InnNMGM39BJsmkqkZFpx5HAUe7ffAPBJhVDIJI4OfcewD+04at3W8aXBx/ZYXxPVO2twqj5nIuKpJVAAeKCaJUuMCyciUtZ89bG61zHFtimMekY4YdSRUwN05Ukq1QfSKFz5FF0E7q7/+KlKATLb3lTtzMJK4olNIDj7EnzW2b9W9PIyyzsnjp/YRSL/u6YlZRWkAT62I8AqnpbegdXMt89aWJMkXF7cfR2TJCJb73qU/ACFYSd6asGWfsCFsDh3dtp8lwpJFwmDY2kUlv6HdN0Hp9NuiX+Ck7z+rrsH9YhVlKlQTtna++bjg2XVgS1rE0o0ogQIDAQAB",
        86400);

    zone.add_caa("@", false, "issue", "letsencrypt.org", 3600);
    zone.add_caa("@", false, "iodef", "mailto:admin@edgerun.tech", 3600);

    zone
}

// ===========================================================================
// DKIM key loader
// ===========================================================================

fn load_dkim_signer() -> Option<DkimSigner> {
    match fs::read_to_string(DKIM_KEY_PATH) {
        Ok(pem) => {
            match DkimSigner::from_private_key_pem(DKIM_DOMAIN, DKIM_SELECTOR, &pem) {
                Ok(signer) => {
                    edgerun_log::info!("DKIM signer loaded from {}", DKIM_KEY_PATH);
                    Some(signer)
                }
                Err(e) => {
                    edgerun_log::warn!("Failed to parse DKIM key: {}", e);
                    None
                }
            }
        }
        Err(e) => {
            edgerun_log::warn!("DKIM key not found at {}, DKIM signing disabled", DKIM_KEY_PATH);
            None
        }
    }
}

// ===========================================================================
// TLS certificate loader
// ===========================================================================

fn load_tls_cert() -> Option<CertificateAndKey> {
    if !Path::new(TLS_CERT_PATH).exists() || !Path::new(TLS_KEY_PATH).exists() {
        edgerun_log::info!("TLS certs not found at {} / {}, running without TLS", TLS_CERT_PATH, TLS_KEY_PATH);
        return None;
    }

    let cert_pem = match fs::read_to_string(TLS_CERT_PATH) {
        Ok(p) => p,
        Err(e) => {
            edgerun_log::warn!("Failed to read TLS cert: {}", e);
            return None;
        }
    };
    let key_pem = match fs::read_to_string(TLS_KEY_PATH) {
        Ok(p) => p,
        Err(e) => {
            edgerun_log::warn!("Failed to read TLS key: {}", e);
            return None;
        }
    };

    let combined = format!("{}\n{}", cert_pem, key_pem);
    match CertificateAndKey::from_pem(&combined) {
        Ok(cert) => {
            edgerun_log::info!("TLS certificate loaded from {}", TLS_CERT_PATH);
            Some(cert)
        }
        Err(e) => {
            edgerun_log::warn!("Failed to parse TLS cert: {}", e);
            None
        }
    }
}

// ===========================================================================
// HTTP handler for MTA-STS
// ===========================================================================

struct MtaStsApp;

impl Handler for MtaStsApp {
    fn handle(&self, req: Request) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send + '_>> {
        let path = req.uri().path().to_string();

        if path == "/.well-known/mta-sts.txt" {
            let policy = "version: STSv1\nmode: enforce\nmx: mail.edgerun.tech\nmax_age: 604800\n";
            return Box::pin(async move {
                Response::new(StatusCode::new(200).unwrap())
                    .with_header("Content-Type", "text/plain; charset=utf-8")
                    .with_body(policy.as_bytes().to_vec())
            });
        }

        Box::pin(async move { Response::not_found() })
    }
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
    edgerun_log::info!("ACME: creating order for {:?}", domains);
    let order = client.create_order(domains).await?;

    let mut active_challenges: Vec<(String, DnsChallenge)> = Vec::new();

    // Process authorizations — inject DNS-01 TXT records
    for authz_url in order.authorization_urls() {
        let authz = client.get_authorization(authz_url).await?;
        edgerun_log::info!("ACME: authorization status = {:?}", authz.status);

        let challenges = authz.challenges.as_ref().map(|v| v.as_slice()).unwrap_or(&[]);
        let dns_challenge = challenges.iter().find(|c| {
            matches!(c.challenge_type, edgerun_acme::types::ChallengeType::Dns01)
        });
        let Some(challenge) = dns_challenge else {
            edgerun_log::warn!("ACME: no dns-01 challenge found");
            continue;
        };

        let token = challenge.token.as_deref().unwrap_or("");

        // Extract domain from authorization identifier
        let domain = authz.identifier.value.clone();
        edgerun_log::info!("ACME: DNS-01 challenge for domain {} token={}", domain, token);

        let dns_ch = DnsChallenge::new(&domain, token, account_key);
        edgerun_log::info!("ACME: adding TXT record {} = {}", dns_ch.record_name(), dns_ch.record_value());

        dns_ch.add_to_zone(base_zone);
        // Re-add zone to DNS server (replaces existing zone with updated one)
        dns_server.add_zone(base_zone.clone()).await;

        active_challenges.push((domain.clone(), dns_ch));

        // Validate challenge
        let validated = client.validate_challenge(&challenge.url).await?;
        edgerun_log::info!("ACME: challenge validated, status = {:?}", validated.status());

        // Poll for challenge to be valid
        let mut attempts = 0;
        while attempts < 30 {
            sleep(Duration::from_secs(2)).await;
            let status = client.get_challenge(&challenge.url).await?;
            edgerun_log::info!("ACME: challenge status = {:?}", status.status());
            if status.is_valid() {
                break;
            }
            if status.status() == edgerun_acme::ChallengeStatus::Invalid {
                edgerun_log::error!("ACME: DNS-01 challenge failed for {}", domain);
                return Err(edgerun_acme::AcmeError::ChallengeFailed(
                    format!("DNS-01 challenge failed for {}", domain),
                ));
            }
            attempts += 1;
        }
    }

    // Clean up DNS challenge TXT records
    for (_, ch) in &active_challenges {
        ch.remove_from_zone(base_zone);
    }
    if !active_challenges.is_empty() {
        dns_server.add_zone(base_zone.clone()).await;
        edgerun_log::info!("ACME: cleaned up DNS-01 TXT records");
    }

    // Check order status
    let order_url = order.inner.id.clone();
    let order = client.get_order(&order_url).await?;
    edgerun_log::info!("ACME: order status = {:?}", order.status());

    if order.is_valid() {
        if let Some(cert_url) = order.certificate_url() {
            edgerun_log::info!("ACME: downloading certificate");
            let cert_pem = client.download_certificate(cert_url).await?;

            fs::create_dir_all("/etc/edgerun/server/tls").ok();
            fs::write(TLS_CERT_PATH, &cert_pem)
                .map_err(|e| edgerun_acme::AcmeError::Storage(e.to_string()))?;

            edgerun_log::info!("ACME: certificate saved to {}", TLS_CERT_PATH);
            return Ok(());
        }
    }

    Err(edgerun_acme::AcmeError::OrderInvalid("order not ready".into()))
}

async fn acme_loop(
    client: AcmeClient,
    account_key: AccountKey,
    dns_server: DnsServer,
    mut base_zone: DnsZone,
    shutdown: CancellationToken,
) {
    edgerun_log::info!("ACME:starting DNS-01 provisioning loop");

    loop {
        if shutdown.is_cancelled() {
            break;
        }

        if let Ok(Some(cert_info)) = check_existing_certs() {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let days_left = (cert_info.expires_at - now) / 86400;
            if days_left > 30 {
                edgerun_log::info!("ACME: certs valid for {} more days, sleeping 12h", days_left);
                for _ in 0..72 {
                    if shutdown.is_cancelled() { break; }
                    sleep(Duration::from_secs(600)).await;
                }
                continue;
            }
            edgerun_log::info!("ACME: certs expire in {} days, renewing", days_left);
        }

        let domains: Vec<String> = ACME_DOMAINS.iter().map(|s| s.to_string()).collect();
        match provision_certs(&client, &account_key, &dns_server, &mut base_zone, &domains).await {
            Ok(()) => {
                edgerun_log::info!("ACME: certificates provisioned successfully");
            }
            Err(e) => {
                edgerun_log::error!("ACME: provisioning failed: {}", e);
            }
        }

        for _ in 0..72 {
            if shutdown.is_cancelled() { break; }
            sleep(Duration::from_secs(600)).await;
        }
    }

    edgerun_log::info!("ACME: loop shut down");
}

struct ExistingCertInfo {
    expires_at: u64,
}

fn check_existing_certs() -> Result<Option<ExistingCertInfo>, String> {
    if !Path::new(TLS_CERT_PATH).exists() {
        return Ok(None);
    }

    let pem = fs::read_to_string(TLS_CERT_PATH).map_err(|e| e.to_string())?;
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
        match TcpStream::connect_timeout(
            &addr.parse().unwrap(),
            Duration::from_secs(5),
        ) {
            Ok(_) => println!("  [OK] {}", name),
            Err(e) => {
                println!("  [FAIL] {} ({})", name, e);
                ok = false;
            }
        }
    }

    if Path::new(TLS_CERT_PATH).exists() {
        match fs::read_to_string(TLS_CERT_PATH) {
            Ok(pem) => {
                match Certificate::from_pem(&pem) {
                    Ok(cert) => {
                        if cert.is_valid_now() {
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
                }
            }
            Err(e) => {
                println!("  [WARN] Cannot read TLS cert: {}", e);
            }
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
// System report
// ===========================================================================

fn run_send_system_report() -> ExitCode {
    let report = format!(
        "From: server-report@{}\r\n\
         To: {}@{}\r\n\
         Subject: Edgerun Server Report\r\n\
         Date: {}\r\n\
         \r\n\
         Edgerun Server System Report\r\n\
         ===========================\r\n\
         \r\n\
         Hostname: {}\r\n\
         TLS cert: {}\r\n\
         DKIM key: {}\r\n\
         Maildir: {}\r\n\
         Queue: {}\r\n\
         ",
        ORIGIN,
        USER_USERNAME,
        ORIGIN,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        HOSTNAME,
        if Path::new(TLS_CERT_PATH).exists() { "present" } else { "not provisioned" },
        if Path::new(DKIM_KEY_PATH).exists() { "present" } else { "missing" },
        MAILDIR_ROOT,
        QUEUE_DATA_ROOT,
    );

    let maildir_new = format!("{}/{}/new", MAILDIR_ROOT, USER_USERNAME);
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

// ===========================================================================
// Main
// ===========================================================================

fn main() -> ExitCode {
    #[cfg(not(target_os = "none"))]
    {
        use std::io::Write;
        edgerun_log::set_format_logger(|level, module, args| {
            let mut stderr = std::io::stderr().lock();
            let _ = writeln!(stderr, "[{}] {}: {}", level.as_str(), module, args);
            let _ = stderr.flush();
        });
    }
    #[cfg(target_os = "none")]
    {
        edgerun_rt::log::init_serial_logger();
    }

    let (mode, _config_path) = parse_args();

    match mode {
        Mode::HealthCheck => {
            return run_health_check();
        }
        Mode::SendSystemReport => {
            return run_send_system_report();
        }
        Mode::Run => {}
    }

    edgerun_log::info!("edgerun-server starting on {}", HOSTNAME);

    let rt = Runtime::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build runtime");

    rt.block_on(async {
        if let Err(e) = run_server().await {
            edgerun_log::error!("server error: {}", e);
            return ExitCode::FAILURE;
        }
        ExitCode::SUCCESS
    })
}

async fn run_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let shutdown = CancellationToken::new();

    let dkim_signer = load_dkim_signer();
    let tls_cert = load_tls_cert();

    // Build DNS zone
    let base_zone = build_dns_zone();
    edgerun_log::info!("DNS zone built for {}", ORIGIN);

    // Start DNS server
    let dns_config = DnsServerConfig {
        bind_addr: "0.0.0.0:53".to_string(),
        default_ttl: 3600,
        rate_limit_qps: 100,
        bind_addr_ipv6: None,
    };
    let dns_server = DnsServer::new(dns_config)?;
    dns_server.add_zone(base_zone.clone()).await;

    let dns_server_for_run = dns_server.clone();
    let dns_task = edgerun_rt::spawn(async move {
        let _ = dns_server_for_run.run().await;
    });

    let smtp_config = SmtpConfig {
        bind_addr: "0.0.0.0:25".to_string(),
        domain_name: HOSTNAME.to_string(),
        max_message_size: 35_882_577,
        smtps: false,
        starttls: tls_cert.is_some(),
        local_domains: vec![ORIGIN.to_string()],
        queue_data_root: Some(std::path::PathBuf::from(QUEUE_DATA_ROOT)),
        relay_dns_server: "1.1.1.1:53".to_string(),
        maildir_root: Some(std::path::PathBuf::from(MAILDIR_ROOT)),
        dkim_domain: dkim_signer.as_ref().map(|_| DKIM_DOMAIN.to_string()),
        dkim_selector: dkim_signer.as_ref().map(|_| DKIM_SELECTOR.to_string()),
        dkim_key_path: dkim_signer.as_ref().map(|_| std::path::PathBuf::from(DKIM_KEY_PATH)),
        tls_cert: tls_cert.clone(),
    };

    let imap_config = ImapConfig {
        bind_addr: "0.0.0.0:143".to_string(),
        domain_name: HOSTNAME.to_string(),
        imaps: false,
        maildir_root: Some(std::path::PathBuf::from(MAILDIR_ROOT)),
        tls_cert: tls_cert.clone(),
    };

    // HTTP server for MTA-STS only
    let http_handler = into_handler_async(|req| {
        let app = MtaStsApp;
        async move { app.handle(req).await }
    });

    let http_server = HttpServer::new(http_handler)
        .bind("0.0.0.0:80")
        .await
        .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error + Send + Sync>)?;

    let http_task = edgerun_rt::spawn(async move {
        let _ = http_server.serve().await;
    });

    let mut server = Server::new()
        .with_smtp(smtp_config)
        .with_imap(imap_config);

    let mut bound = server.build().await?;

    let server_shutdown = shutdown.clone();
    let server_task = edgerun_rt::spawn(async move {
        let _ = bound.run(server_shutdown).await;
    });

    // ACME client with DNS-01 challenge — run as background task
    let acme_account = load_or_create_acme_account();
    edgerun_log::info!("ACME account loaded: {}", acme_account.is_some());
    if let Some(account_key) = acme_account {
        let acme_config = AcmeConfig {
            directory_url: edgerun_acme::DirectoryUrl::LetsEncrypt,
            email: vec![ACME_CONTACT.to_string()],
            terms_of_service_agreed: true,
        };
        let acme_client = AcmeClient::new(acme_config, account_key.clone());
        edgerun_rt::spawn(async move {
            edgerun_log::info!("ACME: starting background init");
            let init_start = std::time::Instant::now();
            match acme_client.init().await {
                Ok(()) => {
                    edgerun_log::info!("ACME: directory fetched in {:?}", init_start.elapsed());
                }
                Err(e) => {
                    edgerun_log::warn!("ACME init failed after {:?}: {}", init_start.elapsed(), e);
                }
            }
        });
    }

    let shutdown_clone = shutdown.clone();
    let signal_task = edgerun_rt::spawn(async move {
        wait_for_signal().await;
        shutdown_clone.cancel();
    });

    edgerun_log::info!("All services started. Press Ctrl-C to stop.");

    let _ = signal_task.await;

    shutdown.cancel();
    dns_server.shutdown().await;
    let _ = server_task.await;
    let _ = http_task.await;
    let _ = dns_task.await;

    edgerun_log::info!("edgerun-server stopped");
    Ok(())
}

fn load_or_create_acme_account() -> Option<AccountKey> {
    if Path::new(ACME_ACCOUNT_KEY_PATH).exists() {
        match fs::read_to_string(ACME_ACCOUNT_KEY_PATH) {
            Ok(pem) => match AccountKey::from_pem(&pem) {
                Ok(key) => {
                    edgerun_log::info!("ACME account key loaded from {}", ACME_ACCOUNT_KEY_PATH);
                    return Some(key);
                }
                Err(e) => {
                    edgerun_log::warn!("Failed to parse ACME account key: {}", e);
                }
            },
            Err(e) => {
                edgerun_log::warn!("Failed to read ACME account key: {}", e);
            }
        }
    }

    let key = AccountKey::generate();
    fs::create_dir_all("/etc/edgerun/server").ok();
    if let Err(e) = fs::write(ACME_ACCOUNT_KEY_PATH, key.pem()) {
        edgerun_log::warn!("Failed to save ACME account key: {}", e);
    } else {
        edgerun_log::info!("ACME account key generated and saved to {}", ACME_ACCOUNT_KEY_PATH);
    }
    Some(key)
}

#[cfg(not(target_os = "none"))]
async fn wait_for_signal() {
    let ctrl_c = edgerun_rt::ctrl_c();
    ctrl_c.await;
    edgerun_log::info!("Received Ctrl-C, shutting down...");
}

#[cfg(target_os = "none")]
async fn wait_for_signal() {
    loop {
        edgerun_rt::sleep(Duration::from_secs(60)).await;
    }
}
