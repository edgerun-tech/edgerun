//! Host mail server binary.
//!
//! Loads Kubernetes-style Edgerun YAML via `edgerun-config`, serves
//! authoritative DNS zones with `edgerun-dns`, accepts SMTP, relays outbound
//! mail through the built-in queue, and exposes IMAP over the same Maildir.

use std::collections::BTreeMap;
use std::io;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;
use std::time::Duration;

use edgerun_config::{
    ConfigResource, DnsServerSpec, DnsZoneSpec, ImapServerSpec, MailUserSpec, SmtpServerSpec,
    ZoneRecord,
};
use edgerun_dns::{DnsRecord, DnsServer, DnsServerConfig, DnsZone};
use edgerun_email::imap::{ImapServer, ImapServerConfig, MaildirImapStore};
use edgerun_email::smtp::server::{MaildirStore, SmtpServer, SmtpServerConfig};
use edgerun_email::smtp::ServerLimits;
use edgerun_rt::CancellationToken;
use edgerun_tls::CertificateAndKey;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage(
            args.first()
                .map(String::as_str)
                .unwrap_or("edgerun-mail-server"),
        );
        return;
    }
    if args.iter().any(|arg| arg == "--init-material") {
        if let Err(error) = init_material(&args) {
            eprintln!("{error}");
            process::exit(1);
        }
        return;
    }
    if args.iter().any(|arg| arg == "--check-config") {
        match load_resources_from_args(&args) {
            Ok(resources) => {
                let (dns, zones, smtp, imap) = count_resources(&resources);
                println!(
                    "dns_servers={dns} dns_zones={zones} smtp_servers={smtp} imap_servers={imap}"
                );
                for resource in &resources {
                    if let ConfigResource::DnsZone(zone) = resource {
                        println!("dns_zone={} records={}", zone.origin, zone.records.len());
                    } else if let ConfigResource::SmtpServer(smtp) = resource {
                        println!(
                            "smtp_server={} local_domains={} users={}",
                            smtp.hostname,
                            smtp.local_domains.join(","),
                            smtp.users.as_ref().map(|users| users.len()).unwrap_or(0)
                        );
                    }
                }
            }
            Err(error) => {
                eprintln!("{error}");
                process::exit(1);
            }
        }
        return;
    }
    let config_path = match parse_config_path(&args) {
        Ok(path) => path,
        Err(message) => {
            eprintln!("{message}");
            process::exit(2);
        }
    };

    let resources = match load_resources(&config_path) {
        Ok(resources) => resources,
        Err(error) => {
            eprintln!("{error}");
            process::exit(1);
        }
    };

    let rt = edgerun_rt::Runtime::new_multi_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|error| {
            eprintln!("failed to build runtime: {error}");
            process::exit(1);
        });

    rt.block_on(async move {
        if let Err(error) = run(resources).await {
            eprintln!("edgerun-mail-server: {error}");
            process::exit(1);
        }
    });
}

fn print_usage(program: &str) {
    println!(
        "usage: {program} --config /etc/edgerun/mail.yaml\n\
         usage: {program} --init-material --domain edgerun.tech --selector mail --out-dir /etc/edgerun/mail"
    );
}

fn load_resources_from_args(args: &[String]) -> Result<Vec<ConfigResource>, String> {
    let config_path = parse_config_path(args)?;
    load_resources(&config_path)
}

fn load_resources(config_path: &Path) -> Result<Vec<ConfigResource>, String> {
    let yaml = std::fs::read_to_string(config_path)
        .map_err(|error| format!("failed to read {}: {error}", config_path.display()))?;
    edgerun_config::parse_config_file(&yaml)
        .map_err(|error| format!("failed to parse {}: {error}", config_path.display()))
}

fn count_resources(resources: &[ConfigResource]) -> (usize, usize, usize, usize) {
    let mut dns = 0;
    let mut zones = 0;
    let mut smtp = 0;
    let mut imap = 0;
    for resource in resources {
        match resource {
            ConfigResource::DnsServer(_) => dns += 1,
            ConfigResource::DnsZone(_) => zones += 1,
            ConfigResource::SmtpServer(_) => smtp += 1,
            ConfigResource::ImapServer(_) => imap += 1,
            _ => {}
        }
    }
    (dns, zones, smtp, imap)
}

fn parse_config_path(args: &[String]) -> Result<PathBuf, String> {
    let mut config = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--check-config" => {}
            "--config" | "-c" if i + 1 < args.len() => {
                config = Some(PathBuf::from(&args[i + 1]));
                i += 1;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        i += 1;
    }
    config.ok_or_else(|| {
        "missing --config /path/to/mail.yaml\nusage: edgerun-mail-server --config /etc/edgerun/mail.yaml"
            .to_string()
    })
}

async fn run(resources: Vec<ConfigResource>) -> io::Result<()> {
    let mut dns_servers = Vec::new();
    let mut zones = Vec::new();
    let mut smtp_specs = Vec::new();
    let mut imap_specs = Vec::new();

    for resource in resources {
        match resource {
            ConfigResource::DnsServer(spec) => dns_servers.push(spec),
            ConfigResource::DnsZone(spec) => zones.push(spec),
            ConfigResource::SmtpServer(spec) => smtp_specs.push(spec),
            ConfigResource::ImapServer(spec) => imap_specs.push(spec),
            _ => {}
        }
    }
    eprintln!(
        "edgerun-mail-server: config dns_servers={} dns_zones={} smtp_servers={} imap_servers={}",
        dns_servers.len(),
        zones.len(),
        smtp_specs.len(),
        imap_specs.len()
    );

    let shutdown = CancellationToken::new();
    let mut tasks = Vec::new();

    if !zones.is_empty() {
        let dns = Arc::new(build_dns_server(dns_servers.first(), &zones).await?);
        let dns_run = Arc::clone(&dns);
        tasks.push(edgerun_rt::spawn(async move {
            dns_run.run().await.map_err(to_io_error)
        }));
        let dns_shutdown = Arc::clone(&dns);
        let token = shutdown.clone();
        tasks.push(edgerun_rt::spawn(async move {
            token.cancelled().await;
            dns_shutdown.shutdown().await;
            Ok(())
        }));
    }

    for spec in smtp_specs {
        for server in build_smtp_servers(&spec)? {
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move { server.run(token).await }));
        }
    }

    for spec in imap_specs {
        for server in build_imap_servers(&spec)? {
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move { server.run(token).await }));
        }
    }

    if tasks.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "config did not define any DnsZone, SmtpServer, or ImapServer resources",
        ));
    }

    eprintln!(
        "edgerun-mail-server: running {} service task(s)",
        tasks.len()
    );
    for task in tasks {
        match task.await {
            Ok(result) => result?,
            Err(error) => return Err(io::Error::other(error)),
        }
    }
    Ok(())
}

async fn build_dns_server(
    server_spec: Option<&DnsServerSpec>,
    zone_specs: &[DnsZoneSpec],
) -> io::Result<DnsServer> {
    let config = DnsServerConfig {
        bind_addr: server_spec
            .and_then(|spec| spec.bind_address.clone())
            .unwrap_or_else(|| "0.0.0.0:53".to_string()),
        bind_addr_ipv6: server_spec.and_then(|spec| spec.bind_address_ipv6.clone()),
        default_ttl: server_spec
            .and_then(|spec| spec.default_ttl)
            .unwrap_or(3600),
        rate_limit_qps: server_spec
            .and_then(|spec| spec.rate_limit_qps)
            .unwrap_or(0),
    };
    let server = DnsServer::new(config).map_err(to_io_error)?;
    if let Some(forward_to) = server_spec.and_then(|spec| spec.forward_to.clone()) {
        server.set_forward_to(Some(forward_to)).await;
    }
    let mut by_origin: BTreeMap<String, Vec<&DnsZoneSpec>> = BTreeMap::new();
    for zone_spec in zone_specs {
        by_origin
            .entry(zone_spec.origin.to_ascii_lowercase())
            .or_default()
            .push(zone_spec);
    }
    for specs in by_origin.values() {
        server.add_zone(zone_from_config(specs)?).await;
    }
    Ok(server)
}

fn zone_from_config(specs: &[&DnsZoneSpec]) -> io::Result<DnsZone> {
    let spec = specs
        .first()
        .ok_or_else(|| invalid_config("empty DNS zone group"))?;
    let mut zone = DnsZone::new(&spec.origin);
    zone.add_record(DnsRecord::soa(
        spec.origin.clone(),
        normalize_target(&spec.soa.mname, &spec.origin),
        spec.soa.rname.clone(),
        spec.soa.serial,
        spec.soa.refresh,
        spec.soa.retry,
        spec.soa.expire,
        spec.soa.minimum,
        spec.soa.minimum,
    ));
    for spec in specs {
        for record in &spec.records {
            add_zone_record(&mut zone, record, &spec.origin)?;
        }
        if let Some(wildcards) = &spec.wildcards {
            for record in wildcards {
                add_zone_record(&mut zone, record, &spec.origin)?;
            }
        }
    }
    Ok(zone)
}

fn add_zone_record(zone: &mut DnsZone, record: &ZoneRecord, origin: &str) -> io::Result<()> {
    let ttl = record.ttl.unwrap_or(3600);
    let rtype = record.record_type.to_ascii_uppercase();
    let value = record_value_string(record)?;

    match rtype.as_str() {
        "A" => zone.add_a(
            &record.name,
            value.parse::<Ipv4Addr>().map_err(invalid_config)?,
            ttl,
        ),
        "AAAA" => zone.add_aaaa(
            &record.name,
            value.parse::<Ipv6Addr>().map_err(invalid_config)?,
            ttl,
        ),
        "CNAME" => zone.add_cname(&record.name, &normalize_target(&value, origin), ttl),
        "NS" => zone.add_ns(&normalize_target(&value, origin)),
        "PTR" => zone.add_ptr(&record.name, &normalize_target(&value, origin), ttl),
        "MX" => {
            let (priority, exchange) = parse_mx(&value)?;
            zone.add_mx(
                &record.name,
                priority,
                &normalize_target(&exchange, origin),
                ttl,
            );
        }
        "TXT" | "SPF" => zone.add_txt(&record.name, &value, ttl),
        "CAA" => {
            let (critical, tag, caa_value) = parse_caa(&value)?;
            zone.add_caa(&record.name, critical, &tag, &caa_value, ttl);
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unsupported DNS record type in mail server config: {rtype}"),
            ));
        }
    }
    Ok(())
}

fn build_smtp_servers(spec: &SmtpServerSpec) -> io::Result<Vec<SmtpServer>> {
    let tls_cert = load_tls_from_spec(spec.tls_cert.as_deref(), spec.tls_key.as_deref())?;
    let mut config = SmtpServerConfig {
        bind_addr: spec
            .bind_address
            .clone()
            .unwrap_or_else(|| "0.0.0.0:25".to_string()),
        domain: spec.hostname.clone(),
        limits: ServerLimits {
            max_message_size: spec.max_message_size.unwrap_or(35_882_577),
            ..Default::default()
        },
        smtps: false,
        starttls: spec.starttls,
        local_domains: spec.local_domains.clone(),
        queue_data_root: if spec.relay_enabled {
            spec.queue_dir.as_ref().map(PathBuf::from)
        } else {
            None
        },
        relay_dns_server: spec
            .dns_server
            .clone()
            .unwrap_or_else(|| "127.0.0.1:53".to_string()),
        #[cfg(feature = "tls")]
        tls_cert: tls_cert.clone(),
        #[cfg(feature = "dkim")]
        dkim_signer: load_dkim_signer(spec)?,
        ..Default::default()
    };

    let handler = Arc::new(MaildirStore::new(required_path(
        spec.maildir_root.as_deref(),
        "SmtpServer.maildir_root",
    )?)?);
    register_smtp_users(&handler, spec)?;
    let mut servers = vec![SmtpServer::new(config.clone(), handler.clone())?];

    if spec.smtps {
        config.bind_addr = implicit_tls_addr(&config.bind_addr, 465);
        config.smtps = true;
        config.starttls = false;
        servers.push(SmtpServer::new(config, handler)?);
    }
    Ok(servers)
}

fn build_imap_servers(spec: &ImapServerSpec) -> io::Result<Vec<ImapServer>> {
    let tls_cert = load_tls_from_spec(spec.tls_cert.as_deref(), spec.tls_key.as_deref())?;
    let store = Arc::new(MaildirImapStore::new(required_path(
        spec.maildir_root.as_deref(),
        "ImapServer.maildir_root",
    )?)?);
    register_imap_users(&store, spec);
    let config = ImapServerConfig {
        bind_addr: spec
            .bind_address
            .clone()
            .unwrap_or_else(|| "0.0.0.0:143".to_string()),
        domain_name: spec.hostname.clone(),
        imaps: false,
        #[cfg(feature = "tls")]
        tls_cert: tls_cert.clone(),
        ..Default::default()
    };

    let mut servers = vec![ImapServer::with_store(config, store.clone())?];
    if spec.imaps {
        let config = ImapServerConfig {
            bind_addr: implicit_tls_addr(
                spec.bind_address.as_deref().unwrap_or("0.0.0.0:143"),
                993,
            ),
            domain_name: spec.hostname.clone(),
            imaps: true,
            #[cfg(feature = "tls")]
            tls_cert,
            ..Default::default()
        };
        servers.push(ImapServer::with_store(config, store)?);
    }
    Ok(servers)
}

fn register_smtp_users(store: &MaildirStore, spec: &SmtpServerSpec) -> io::Result<()> {
    let users = configured_users(spec.users.as_deref(), &spec.local_domains);
    for user in users {
        let domains = user
            .domains
            .as_deref()
            .unwrap_or(spec.local_domains.as_slice());
        let domain_refs: Vec<&str> = domains.iter().map(String::as_str).collect();
        store.add_user(&user.username, &domain_refs)?;
    }
    Ok(())
}

fn register_imap_users(store: &MaildirImapStore, spec: &ImapServerSpec) {
    let users = configured_users(spec.users.as_deref(), &[]);
    for user in users {
        if let Some(password) = user.password.as_deref() {
            store.add_user(&user.username, password);
        }
    }
}

fn configured_users(users: Option<&[MailUserSpec]>, local_domains: &[String]) -> Vec<MailUserSpec> {
    if let Some(users) = users {
        return users.to_vec();
    }
    let domains = if local_domains.is_empty() {
        None
    } else {
        Some(local_domains.to_vec())
    };
    vec![MailUserSpec {
        username: "postmaster".to_string(),
        password: None,
        domains,
    }]
}

#[cfg(feature = "dkim")]
fn load_dkim_signer(spec: &SmtpServerSpec) -> io::Result<Option<edgerun_email_auth::DkimSigner>> {
    let (Some(domain), Some(selector), Some(path)) = (
        spec.dkim_domain.as_deref(),
        spec.dkim_selector.as_deref(),
        spec.dkim_key_path.as_deref(),
    ) else {
        return Ok(None);
    };
    let pem = std::fs::read_to_string(path)?;
    edgerun_email_auth::DkimSigner::from_private_key_pem(domain, selector, &pem).map(Some)
}

fn load_tls_from_spec(
    cert: Option<&str>,
    key: Option<&str>,
) -> io::Result<Option<CertificateAndKey>> {
    let (Some(cert), Some(key)) = (cert, key) else {
        return Ok(None);
    };
    let cert_pem = read_pem_or_file(cert)?;
    let key_pem = read_pem_or_file(key)?;
    CertificateAndKey::from_pem(&format!("{cert_pem}\n{key_pem}"))
        .map(Some)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, format!("{error}")))
}

fn read_pem_or_file(value: &str) -> io::Result<String> {
    if value.contains("-----BEGIN ") {
        Ok(value.to_string())
    } else {
        std::fs::read_to_string(value)
    }
}

fn required_path<'a>(value: Option<&'a str>, field: &str) -> io::Result<&'a Path> {
    value.map(Path::new).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{field} is required for persistent mail service"),
        )
    })
}

fn record_value_string(record: &ZoneRecord) -> io::Result<String> {
    if let Some(value) = record.value.as_str() {
        return Ok(value.to_string());
    }
    if let Some(value) = record.value.as_i64() {
        return Ok(value.to_string());
    }
    if let Some(value) = record.value.as_u64() {
        return Ok(value.to_string());
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "DNS record {} {} must use a scalar value",
            record.name, record.record_type
        ),
    ))
}

fn parse_mx(value: &str) -> io::Result<(u16, String)> {
    let mut parts = value.split_whitespace();
    let priority = parts
        .next()
        .ok_or_else(|| invalid_config("MX value must be '<priority> <exchange>'"))?
        .parse::<u16>()
        .map_err(invalid_config)?;
    let exchange = parts
        .next()
        .ok_or_else(|| invalid_config("MX value must be '<priority> <exchange>'"))?;
    Ok((priority, exchange.to_string()))
}

fn parse_caa(value: &str) -> io::Result<(bool, String, String)> {
    let mut parts = value.splitn(3, char::is_whitespace);
    let flags = parts
        .next()
        .ok_or_else(|| invalid_config("CAA value must be '<flags> <tag> <value>'"))?
        .parse::<u8>()
        .map_err(invalid_config)?;
    let tag = parts
        .next()
        .ok_or_else(|| invalid_config("CAA value must be '<flags> <tag> <value>'"))?;
    let caa_value = parts
        .next()
        .ok_or_else(|| invalid_config("CAA value must be '<flags> <tag> <value>'"))?;
    Ok((flags & 0x80 != 0, tag.to_string(), caa_value.to_string()))
}

fn normalize_target(value: &str, origin: &str) -> String {
    let trimmed = value.trim_end_matches('.');
    if trimmed == "@" {
        origin.to_string()
    } else if trimmed.ends_with(origin) {
        trimmed.to_string()
    } else {
        format!("{trimmed}.{origin}")
    }
}

fn implicit_tls_addr(addr: &str, port: u16) -> String {
    if let Some((host, _)) = addr.rsplit_once(':') {
        format!("{host}:{port}")
    } else {
        format!("0.0.0.0:{port}")
    }
}

fn invalid_config(error: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error.to_string())
}

fn to_io_error(error: impl std::fmt::Display) -> io::Error {
    io::Error::other(error.to_string())
}

fn init_material(args: &[String]) -> io::Result<()> {
    let domain = arg_value(args, "--domain").unwrap_or("edgerun.tech");
    let selector = arg_value(args, "--selector").unwrap_or("mail");
    let out_dir = PathBuf::from(arg_value(args, "--out-dir").unwrap_or("/etc/edgerun/mail"));
    let tls_dir = out_dir.join("tls");
    std::fs::create_dir_all(&tls_dir)?;

    let dkim = edgerun_email_auth::DkimSigner::generate(domain, selector).map_err(to_io_error)?;
    let dkim_key_path = out_dir.join(format!("dkim-{selector}.private.pem"));
    std::fs::write(&dkim_key_path, dkim.private_key_pem().map_err(to_io_error)?)?;

    let (cert_pem, key_pem) =
        edgerun_tls::generate_self_signed_pem(&[&format!("mail.{domain}"), domain])
            .map_err(to_io_error)?;
    std::fs::write(tls_dir.join("fullchain.pem"), cert_pem)?;
    std::fs::write(tls_dir.join("privkey.pem"), key_pem)?;
    println!("dkim_key_path={}", dkim_key_path.display());
    println!(
        "dkim_txt_name={selector}._domainkey.{domain}\ndkim_txt_value={}",
        dkim.public_key_txt()
    );
    println!("tls_cert={}", tls_dir.join("fullchain.pem").display());
    println!("tls_key={}", tls_dir.join("privkey.pem").display());
    Ok(())
}

fn arg_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
}

#[allow(dead_code)]
async fn _keep_acme_dns_challenge_api_reachable(
    zone: &mut DnsZone,
    challenge: &edgerun_acme::DnsChallenge,
) {
    challenge.add_to_zone(zone);
}
