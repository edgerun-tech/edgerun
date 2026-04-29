//! Host Edgerun server binary.
//!
//! Loads Kubernetes-style Edgerun YAML via `edgerun-config`, serves
//! authoritative DNS zones with `edgerun-dns`, accepts SMTP, relays outbound
//! mail through the built-in queue, and exposes IMAP over the same Maildir.

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::future::Future;
use std::io;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, TcpStream, UdpSocket};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use edgerun_acme::{AccountKey, AcmeClient, AcmeConfig};
use edgerun_acme::{ChallengeStatus, ChallengeType, DirectoryUrl, OrderStatus};
use edgerun_blog::{BlogConfig, BlogHandler};
use edgerun_config::edgerun_json::JsonValue;
use edgerun_config::{
    ConfigResource, DnsServerSpec, DnsZoneSpec, DnssecConfig, ImapServerSpec, MailUserSpec,
    SmtpServerSpec, ZoneRecord,
};
use edgerun_dns::{
    DnsMessage, DnsRecord, DnsRecordData, DnsRecordType, DnsResponseCode, DnsServer,
    DnsServerConfig, DnsZone,
};
use edgerun_email::imap::{ImapServer, ImapServerConfig, MaildirImapStore};
use edgerun_email::smtp::relay::queue::{MailIndex, MailQueueStats};
use edgerun_email::smtp::server::{MailHandler, MaildirStore, SmtpServer, SmtpServerConfig};
use edgerun_email::smtp::types::MailEnvelope;
use edgerun_email::smtp::ServerLimits;
use edgerun_encoding::base64::{standard_decode, standard_encode_wrapped};
use edgerun_http::{Handler, HttpServer, Request, Response, StatusCode};
use edgerun_machine_report::{gather_machine_report, render_machine_report, OutputFormat};
use edgerun_rt::CancellationToken;
use edgerun_tls::CertificateAndKey;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage(args.first().map(String::as_str).unwrap_or("edgerun-server"));
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
    if args.iter().any(|arg| arg == "--send-system-report") {
        match send_system_report_from_args(&args) {
            Ok(()) => return,
            Err(error) => {
                eprintln!("system-report: failed: {error}");
                process::exit(1);
            }
        }
    }
    if args.iter().any(|arg| arg == "--health-check") {
        match run_health_check_from_args(&args) {
            Ok(()) => return,
            Err(error) => {
                eprintln!("health: failed: {error}");
                process::exit(1);
            }
        }
    }
    let options = match parse_server_options(&args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            process::exit(2);
        }
    };

    let resources = match load_resources(&options.config_path) {
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
        if let Err(error) = run(resources, options.blog).await {
            eprintln!("edgerun-server: {error}");
            process::exit(1);
        }
    });
}

fn print_usage(program: &str) {
    println!(
        "usage: {program} --config /etc/edgerun/server/server.yaml\n\
         usage: {program} --health-check --config /etc/edgerun/server/server.yaml\n\
         usage: {program} --send-system-report --config /etc/edgerun/server/server.yaml [--report-to admin@example.com]\n\
         usage: {program} --config /etc/edgerun/server/server.yaml --blog-host blog.edgerun.tech --blog-root /srv/blog [--blog-static-root /srv/blog/.generated]\n\
         usage: {program} --init-material --domain edgerun.tech --selector mail --out-dir /etc/edgerun/server"
    );
}

fn load_resources_from_args(args: &[String]) -> Result<Vec<ConfigResource>, String> {
    let config_path = parse_server_options(args)?.config_path;
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

#[derive(Clone)]
struct ServerOptions {
    config_path: PathBuf,
    blog: Option<BlogMount>,
}

#[derive(Clone)]
struct BlogMount {
    host: String,
    root: PathBuf,
    static_root: Option<PathBuf>,
    title: String,
    description: String,
    base_url: String,
}

fn parse_server_options(args: &[String]) -> Result<ServerOptions, String> {
    let mut config = None;
    let mut blog_host = None;
    let mut blog_root = None;
    let mut blog_static_root = None;
    let mut blog_title = "Edgerun Blog".to_string();
    let mut blog_description = "Notes from the Edgerun project.".to_string();
    let mut blog_base_url = String::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--check-config" => {}
            "--health-check" => {}
            "--send-system-report" => {}
            "--report-to" | "--report-from" if i + 1 < args.len() => {
                i += 1;
            }
            "--config" | "-c" if i + 1 < args.len() => {
                config = Some(PathBuf::from(&args[i + 1]));
                i += 1;
            }
            "--blog-host" if i + 1 < args.len() => {
                blog_host = Some(args[i + 1].clone());
                i += 1;
            }
            "--blog-root" if i + 1 < args.len() => {
                blog_root = Some(PathBuf::from(&args[i + 1]));
                i += 1;
            }
            "--blog-static-root" if i + 1 < args.len() => {
                blog_static_root = Some(PathBuf::from(&args[i + 1]));
                i += 1;
            }
            "--blog-title" if i + 1 < args.len() => {
                blog_title = args[i + 1].clone();
                i += 1;
            }
            "--blog-description" if i + 1 < args.len() => {
                blog_description = args[i + 1].clone();
                i += 1;
            }
            "--blog-base-url" if i + 1 < args.len() => {
                blog_base_url = args[i + 1].trim_end_matches('/').to_string();
                i += 1;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        i += 1;
    }
    let config_path = config.ok_or_else(|| {
        "missing --config /path/to/server.yaml\nusage: edgerun-server --config /etc/edgerun/server/server.yaml"
            .to_string()
    })?;
    let blog = match (blog_host, blog_root) {
        (Some(host), Some(root)) => Some(BlogMount {
            host,
            root,
            static_root: blog_static_root,
            title: blog_title,
            description: blog_description,
            base_url: blog_base_url,
        }),
        (None, None) => None,
        _ => {
            return Err(
                "--blog-host and --blog-root must be provided together when enabling the blog"
                    .to_string(),
            );
        }
    };
    Ok(ServerOptions { config_path, blog })
}

fn run_health_check_from_args(args: &[String]) -> io::Result<()> {
    let resources = load_resources_from_args(args).map_err(invalid_config)?;
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

    let mut checks = 0usize;
    for cert_path in health_tls_cert_paths(&smtp_specs, &imap_specs) {
        check_certificate_fresh(&cert_path, Duration::from_secs(14 * 24 * 60 * 60))?;
        checks += 1;
    }

    if !zones.is_empty() {
        let dns_addr = local_probe_addr(
            dns_servers
                .first()
                .and_then(|spec| spec.bind_address.as_deref()),
            "0.0.0.0:53",
        );
        for zone in &zones {
            check_dns_zone(&dns_addr, zone)?;
            checks += 1;
        }
    }

    for spec in &smtp_specs {
        let smtp_addr = local_probe_addr(spec.bind_address.as_deref(), "0.0.0.0:25");
        check_line_banner("smtp", &smtp_addr, "220")?;
        checks += 1;
        if spec.relay_enabled {
            let stats = queue_stats_from_spec(spec)?;
            println!(
                "health: ok mail_queue active={} queued={} retrying={} sending={} recipients={} bytes={} max_retries={}",
                stats.active,
                stats.queued,
                stats.retrying,
                stats.sending,
                stats.recipients,
                stats.bytes,
                stats.max_retry_count
            );
            checks += 1;
        }

        if spec.smtps {
            let smtps_addr =
                local_probe_addr(Some(&implicit_tls_addr(&smtp_addr, 465)), "127.0.0.1:465");
            check_tcp_connect("smtps", &smtps_addr)?;
            checks += 1;
        }
        if spec.starttls
            && smtp_health_auth_enabled(spec, &imap_specs)
            && spec.tls_cert.is_some()
            && spec.tls_key.is_some()
        {
            let submission_addr = local_probe_addr(
                Some(&implicit_tls_addr(
                    spec.bind_address.as_deref().unwrap_or("0.0.0.0:25"),
                    587,
                )),
                "127.0.0.1:587",
            );
            check_line_banner("submission", &submission_addr, "220")?;
            checks += 1;
        }
    }

    for spec in &imap_specs {
        let imap_addr = local_probe_addr(spec.bind_address.as_deref(), "0.0.0.0:143");
        check_line_banner("imap", &imap_addr, "* OK")?;
        checks += 1;

        if spec.imaps {
            let imaps_addr = local_probe_addr(
                Some(&implicit_tls_addr(
                    spec.bind_address.as_deref().unwrap_or("0.0.0.0:143"),
                    993,
                )),
                "127.0.0.1:993",
            );
            check_tcp_connect("imaps", &imaps_addr)?;
            checks += 1;
        }
    }

    if let Some(webmail) = build_webmail_config(&smtp_specs, &imap_specs)? {
        check_http_status(
            "http",
            "127.0.0.1:80",
            &webmail.hostname,
            &["HTTP/1.1 200", "HTTP/1.1 308", "HTTP/1.1 404"],
        )?;
        checks += 1;
        if webmail.tls_cert.is_some() && webmail.tls_key.is_some() {
            check_tcp_connect("https", "127.0.0.1:443")?;
            checks += 1;
        }
    }

    if checks == 0 {
        return Err(invalid_config(
            "config did not define any health-checkable DNS, SMTP, IMAP, or web listener",
        ));
    }
    println!("health: ok checks={checks}");
    Ok(())
}

fn send_system_report_from_args(args: &[String]) -> io::Result<()> {
    let resources = load_resources_from_args(args).map_err(invalid_config)?;
    let mut smtp_specs = Vec::new();
    let mut imap_specs = Vec::new();
    for resource in resources {
        match resource {
            ConfigResource::SmtpServer(spec) => smtp_specs.push(spec),
            ConfigResource::ImapServer(spec) => imap_specs.push(spec),
            _ => {}
        }
    }
    let smtp = smtp_specs
        .first()
        .ok_or_else(|| invalid_config("system report requires a SmtpServer resource"))?;
    let maildir_root = required_path(smtp.maildir_root.as_deref(), "SmtpServer.maildir_root")?;
    let store = MaildirStore::new(&maildir_root)?;
    register_smtp_users(&store, smtp, &imap_specs)?;

    let to = arg_value(args, "--report-to")
        .map(str::to_string)
        .or_else(|| default_report_recipient(smtp))
        .ok_or_else(|| invalid_config("missing --report-to and no default local user"))?;
    let from = arg_value(args, "--report-from")
        .map(str::to_string)
        .unwrap_or_else(|| default_report_sender(smtp));
    store.validate_recipient(&to)?;

    let report = gather_machine_report();
    let machine = render_machine_report(&report, OutputFormat::Text).map_err(invalid_config)?;
    let queue_stats = queue_report_block(smtp)?;
    let body = format!(
        "edgerun server report\r\n\
         hostname: {}\r\n\
         smtp_hostname: {}\r\n\
         local_domains: {}\r\n\
         maildir_root: {}\r\n\
         queue_dir: {}\r\n\
         queue_active: {}\r\n\
         queue_queued: {}\r\n\
         queue_retrying: {}\r\n\
         queue_sending: {}\r\n\
         queue_recipients: {}\r\n\
         queue_bytes: {}\r\n\
         queue_max_retries: {}\r\n\
         tls_cert: {}\r\n\
         generated_by: edgerun-server --send-system-report\r\n\
         \r\n\
         {}\r\n",
        local_hostname(),
        smtp.hostname,
        smtp.local_domains.join(","),
        maildir_root.display(),
        smtp.queue_dir.as_deref().unwrap_or("(disabled)"),
        queue_stats.active,
        queue_stats.queued,
        queue_stats.retrying,
        queue_stats.sending,
        queue_stats.recipients,
        queue_stats.bytes,
        queue_stats.max_retry_count,
        smtp.tls_cert.as_deref().unwrap_or("(none)"),
        normalize_crlf(&machine)
    );
    let message = format!(
        "From: Edgerun System <{}>\r\n\
         To: <{}>\r\n\
         Subject: Edgerun system report for {}\r\n\
         Date: {}\r\n\
         Message-ID: <system-report-{}@{}>\r\n\
         MIME-Version: 1.0\r\n\
         Content-Type: text/plain; charset=utf-8\r\n\
         Content-Transfer-Encoding: 8bit\r\n\
         \r\n\
         {}",
        sanitize_header(&from),
        sanitize_header(&to),
        sanitize_header(&smtp.hostname),
        http_date_now(),
        unique_webmail_id(),
        sanitize_header(&smtp.hostname),
        body
    );
    let mut envelope = MailEnvelope::new(from);
    envelope.add_recipient(to.clone(), Vec::new(), Default::default(), None);
    envelope.data = message.into_bytes();
    store.accept_mail(&envelope)?;
    println!("system-report: delivered locally to {to}");
    Ok(())
}

fn queue_report_block(spec: &SmtpServerSpec) -> io::Result<MailQueueStats> {
    if !spec.relay_enabled || spec.queue_dir.is_none() {
        return Ok(MailQueueStats::default());
    }
    queue_stats_from_spec(spec)
}

fn default_report_recipient(spec: &SmtpServerSpec) -> Option<String> {
    let users = configured_users(spec.users.as_deref(), &spec.local_domains);
    let user = spec
        .catch_all_user
        .as_deref()
        .or_else(|| users.first().map(|user| user.username.as_str()))?;
    let domain = spec.local_domains.first()?;
    Some(format!("{user}@{domain}"))
}

fn default_report_sender(spec: &SmtpServerSpec) -> String {
    let domain = spec
        .local_domains
        .first()
        .map(String::as_str)
        .unwrap_or("localhost");
    format!("system@{domain}")
}

fn local_hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .map(|value| value.trim().to_string())
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "localhost".to_string())
}

fn health_tls_cert_paths(
    smtp_specs: &[SmtpServerSpec],
    imap_specs: &[ImapServerSpec],
) -> Vec<String> {
    let mut paths = Vec::new();
    for path in smtp_specs
        .iter()
        .filter_map(|spec| spec.tls_cert.as_deref())
        .chain(
            imap_specs
                .iter()
                .filter_map(|spec| spec.tls_cert.as_deref()),
        )
    {
        if !paths.iter().any(|existing| existing == path) {
            paths.push(path.to_string());
        }
    }
    paths
}

fn check_certificate_fresh(path: &str, minimum_remaining: Duration) -> io::Result<()> {
    let pem = std::fs::read_to_string(path)?;
    let cert = edgerun_tls::certificate::Certificate::from_pem(&pem).map_err(invalid_config)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if now < cert.not_before {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("certificate {path} is not valid yet"),
        ));
    }
    let threshold = now.saturating_add(minimum_remaining.as_secs());
    if cert.not_after <= threshold {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "certificate {path} expires too soon: not_after={}",
                cert.not_after
            ),
        ));
    }
    println!(
        "health: ok certificate path={path} not_after={}",
        cert.not_after
    );
    Ok(())
}

fn check_dns_zone(addr: &str, zone: &DnsZoneSpec) -> io::Result<()> {
    let soa = dns_query(addr, &zone.origin, DnsRecordType::SOA)?;
    require_dns_response(
        &soa,
        DnsResponseCode::NoError,
        &zone.origin,
        DnsRecordType::SOA,
    )?;
    require_record(&soa.answers, DnsRecordType::SOA, "answer")?;
    if zone.dnssec.is_some() {
        require_rrsig(&soa.answers, DnsRecordType::SOA, "answer")?;

        let dnskey = dns_query(addr, &zone.origin, DnsRecordType::DNSKEY)?;
        require_dns_response(
            &dnskey,
            DnsResponseCode::NoError,
            &zone.origin,
            DnsRecordType::DNSKEY,
        )?;
        require_record(&dnskey.answers, DnsRecordType::DNSKEY, "answer")?;
        require_rrsig(&dnskey.answers, DnsRecordType::DNSKEY, "answer")?;
        require_fresh_rrsigs(&dnskey.answers, Duration::from_secs(24 * 60 * 60))?;

        let negative_name = format!("health-nx-{}.{}", process::id(), zone.origin);
        let negative = dns_query(addr, &negative_name, DnsRecordType::A)?;
        require_dns_response(
            &negative,
            DnsResponseCode::NXDomain,
            &negative_name,
            DnsRecordType::A,
        )?;
        require_record(&negative.authority, DnsRecordType::NSEC, "authority")?;
        require_rrsig(&negative.authority, DnsRecordType::NSEC, "authority")?;
        require_fresh_rrsigs(&negative.authority, Duration::from_secs(24 * 60 * 60))?;
    }
    if zone
        .records
        .iter()
        .any(|record| record.record_type.eq_ignore_ascii_case("CAA"))
    {
        let caa = dns_query(addr, &zone.origin, DnsRecordType::CAA)?;
        require_dns_response(
            &caa,
            DnsResponseCode::NoError,
            &zone.origin,
            DnsRecordType::CAA,
        )?;
        require_record(&caa.answers, DnsRecordType::CAA, "answer")?;
        if zone.dnssec.is_some() {
            require_rrsig(&caa.answers, DnsRecordType::CAA, "answer")?;
            require_fresh_rrsigs(&caa.answers, Duration::from_secs(24 * 60 * 60))?;
        }
    }
    println!("health: ok dns zone={}", zone.origin);
    Ok(())
}

fn queue_stats_from_spec(spec: &SmtpServerSpec) -> io::Result<MailQueueStats> {
    let queue_dir = spec
        .queue_dir
        .as_deref()
        .ok_or_else(|| invalid_config("relay_enabled requires SmtpServer.queue_dir"))?;
    let rt = edgerun_rt::Builder::new_multi_thread()
        .build()
        .map_err(to_io_error)?;
    rt.block_on(async move {
        let index = MailIndex::open(Path::new(queue_dir)).await?;
        Ok(index.stats().await)
    })
}

fn dns_query(addr: &str, name: &str, qtype: DnsRecordType) -> io::Result<DnsMessage> {
    let query = DnsMessage::query(dns_query_id(), name.to_string(), qtype);
    let wire = query.to_wire();
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::from_secs(5)))?;
    socket.set_write_timeout(Some(Duration::from_secs(5)))?;
    socket.send_to(&wire, addr)?;
    let mut buf = [0u8; 4096];
    let (len, _) = socket.recv_from(&mut buf)?;
    let response = parse_dns_message(&buf[..len])?;
    if response.header.truncated {
        return dns_query_tcp(addr, name, qtype);
    }
    Ok(response)
}

fn dns_query_tcp(addr: &str, name: &str, qtype: DnsRecordType) -> io::Result<DnsMessage> {
    let query = DnsMessage::query(dns_query_id(), name.to_string(), qtype);
    let wire = query.to_wire();
    let mut stream = connect_tcp(addr)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    stream.write_all(&(wire.len() as u16).to_be_bytes())?;
    stream.write_all(&wire)?;
    let mut len_buf = [0u8; 2];
    stream.read_exact(&mut len_buf)?;
    let len = u16::from_be_bytes(len_buf) as usize;
    let mut response = vec![0u8; len];
    stream.read_exact(&mut response)?;
    parse_dns_message(&response)
}

fn dns_query_id() -> u16 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos as u16) ^ (process::id() as u16)
}

fn parse_dns_message(data: &[u8]) -> io::Result<DnsMessage> {
    DnsMessage::from_wire(data).map_err(to_io_error)
}

fn require_dns_response(
    message: &DnsMessage,
    expected: DnsResponseCode,
    name: &str,
    qtype: DnsRecordType,
) -> io::Result<()> {
    if message.header.response_code != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "dns {name} {qtype}: expected {}, got {}",
                expected.as_str(),
                message.header.response_code.as_str()
            ),
        ));
    }
    Ok(())
}

fn require_record(records: &[DnsRecord], rtype: DnsRecordType, section: &str) -> io::Result<()> {
    if records.iter().any(|record| record.rtype == rtype) {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("dns response missing {rtype} in {section} section"),
    ))
}

fn require_rrsig(records: &[DnsRecord], covered: DnsRecordType, section: &str) -> io::Result<()> {
    let covered = covered.as_u16();
    if records.iter().any(|record| {
        matches!(
            &record.data,
            DnsRecordData::RRSIG { type_covered, .. } if *type_covered == covered
        )
    }) {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("dns response missing RRSIG in {section} section"),
    ))
}

fn require_fresh_rrsigs(records: &[DnsRecord], minimum_remaining: Duration) -> io::Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let threshold = now + minimum_remaining.as_secs();
    let stale = records.iter().find_map(|record| {
        if let DnsRecordData::RRSIG { expiration, .. } = &record.data {
            if u64::from(*expiration) <= threshold {
                return Some(*expiration);
            }
        }
        None
    });
    if let Some(expiration) = stale {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("dns response contains RRSIG expiring too soon: {expiration}"),
        ));
    }
    Ok(())
}

fn check_line_banner(label: &str, addr: &str, expected_prefix: &str) -> io::Result<()> {
    let mut stream = connect_tcp(addr)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    if !line.starts_with(expected_prefix) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{label} {addr}: unexpected banner {line:?}"),
        ));
    }
    println!("health: ok {label} addr={addr}");
    Ok(())
}

fn check_http_status(label: &str, addr: &str, host: &str, allowed: &[&str]) -> io::Result<()> {
    let mut stream = connect_tcp(addr)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    write!(
        stream,
        "GET / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
    )?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    if !allowed.iter().any(|prefix| line.starts_with(prefix)) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{label} {addr}: unexpected status {line:?}"),
        ));
    }
    println!("health: ok {label} addr={addr}");
    Ok(())
}

fn check_tcp_connect(label: &str, addr: &str) -> io::Result<()> {
    let stream = connect_tcp(addr)?;
    stream.shutdown(std::net::Shutdown::Both).ok();
    println!("health: ok {label} addr={addr}");
    Ok(())
}

fn connect_tcp(addr: &str) -> io::Result<TcpStream> {
    if let Ok(socket) = addr.parse::<SocketAddr>() {
        TcpStream::connect_timeout(&socket, Duration::from_secs(5))
    } else {
        TcpStream::connect(addr)
    }
}

fn local_probe_addr(configured: Option<&str>, default_addr: &str) -> String {
    let addr = configured.unwrap_or(default_addr);
    let Some((host, port)) = addr.rsplit_once(':') else {
        return default_addr.to_string();
    };
    let local_host = match host {
        "" | "0.0.0.0" | "::" | "[::]" => "127.0.0.1",
        other => other,
    };
    format!("{local_host}:{port}")
}

fn smtp_health_auth_enabled(spec: &SmtpServerSpec, imap_specs: &[ImapServerSpec]) -> bool {
    configured_users(spec.users.as_deref(), &spec.local_domains)
        .iter()
        .any(|user| smtp_user_password(user, imap_specs).is_some())
}

async fn run(resources: Vec<ConfigResource>, blog: Option<BlogMount>) -> io::Result<()> {
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
        "edgerun-server: config dns_servers={} dns_zones={} smtp_servers={} imap_servers={}",
        dns_servers.len(),
        zones.len(),
        smtp_specs.len(),
        imap_specs.len()
    );

    let shutdown = CancellationToken::new();
    let mut tasks = Vec::new();
    let mut dns_server = None;

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
        if zones_have_dnssec(&zones) {
            let dns_resign = Arc::clone(&dns);
            let zone_specs = zones.clone();
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move {
                run_dnssec_resigner(dns_resign, zone_specs, token).await
            }));
        }
        dns_server = Some(dns);
    }

    for spec in &smtp_specs {
        if spec.acme_enabled {
            let Some(dns) = dns_server.as_ref() else {
                return Err(invalid_config("ACME DNS-01 requires at least one DnsZone"));
            };
            ensure_acme_certificate(spec, dns, &zones).await?;
        }
    }

    if let Some(webmail) = build_webmail_config(&smtp_specs, &imap_specs)? {
        let tls = load_tls_from_spec(webmail.tls_cert.as_deref(), webmail.tls_key.as_deref())?;
        let web_handler = WebmailHandler::new(webmail.clone());
        let site_handler = SiteRouter::new(web_handler, blog);
        if tls.is_some() {
            let http = HttpServer::new(HttpsRedirectHandler::new(webmail.hostname.clone()))
                .bind("0.0.0.0:80")
                .await
                .map_err(to_io_error)?;
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move {
                http.serve_with_shutdown(token).await.map_err(to_io_error)
            }));
        } else {
            let http = HttpServer::new(site_handler.clone())
                .bind("0.0.0.0:80")
                .await
                .map_err(to_io_error)?;
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move {
                http.serve_with_shutdown(token).await.map_err(to_io_error)
            }));
        }

        if let Some(tls) = tls {
            let https = HttpServer::new(site_handler)
                .with_tls(tls)
                .bind("0.0.0.0:443")
                .await
                .map_err(to_io_error)?;
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move {
                https.serve_with_shutdown(token).await.map_err(to_io_error)
            }));
        }
    }

    for spec in &smtp_specs {
        for server in build_smtp_servers(spec, &imap_specs)? {
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

    eprintln!("edgerun-server: running {} service task(s)", tasks.len());
    for task in tasks {
        match task.await {
            Ok(result) => result?,
            Err(error) => return Err(io::Error::other(error)),
        }
    }
    Ok(())
}

#[derive(Clone)]
struct WebmailConfig {
    hostname: String,
    username: String,
    password: String,
    address: String,
    maildir_root: PathBuf,
    smtp_addr: String,
    tls_cert: Option<String>,
    tls_key: Option<String>,
}

fn build_webmail_config(
    smtp_specs: &[SmtpServerSpec],
    imap_specs: &[ImapServerSpec],
) -> io::Result<Option<WebmailConfig>> {
    let Some(imap) = imap_specs.first() else {
        return Ok(None);
    };
    let Some(user) = imap.users.as_deref().and_then(|users| users.first()) else {
        return Ok(None);
    };
    let Some(password) = user.password.clone() else {
        return Ok(None);
    };
    let smtp = smtp_specs.first();
    let hostname = smtp
        .map(|spec| spec.hostname.clone())
        .unwrap_or_else(|| imap.hostname.clone());
    let domain = smtp
        .and_then(|spec| spec.local_domains.first().cloned())
        .unwrap_or_else(|| hostname.trim_start_matches("mail.").to_string());
    let smtp_addr = smtp
        .and_then(|spec| spec.bind_address.clone())
        .unwrap_or_else(|| "127.0.0.1:25".to_string())
        .replace("0.0.0.0:", "127.0.0.1:");
    let maildir_root = imap
        .maildir_root
        .clone()
        .or_else(|| smtp.and_then(|spec| spec.maildir_root.clone()))
        .unwrap_or_else(|| "/var/lib/edgerun/mail/maildirs".to_string());
    Ok(Some(WebmailConfig {
        hostname,
        username: user.username.clone(),
        password,
        address: format!("{}@{}", user.username, domain),
        maildir_root: PathBuf::from(maildir_root),
        smtp_addr,
        tls_cert: imap.tls_cert.clone(),
        tls_key: imap.tls_key.clone(),
    }))
}

#[derive(Clone)]
struct WebmailHandler {
    config: WebmailConfig,
}

impl WebmailHandler {
    fn new(config: WebmailConfig) -> Self {
        Self { config }
    }

    fn handle_sync(&self, request: Request) -> Response {
        let path = request.uri().request_target();
        if path == "/" || path == "/index.html" {
            return match request.method().as_str() {
                "GET" | "HEAD" => html_response(WEBMAIL_HTML),
                _ => method_not_allowed("GET, HEAD"),
            };
        }
        if path == "/bimi/logo.svg" {
            return match request.method().as_str() {
                "GET" | "HEAD" => svg_response(BIMI_LOGO_SVG),
                _ => method_not_allowed("GET, HEAD"),
            };
        }
        if path == "/.well-known/mta-sts.txt" {
            return match request.method().as_str() {
                "GET" | "HEAD" => mta_sts_policy_response(),
                _ => method_not_allowed("GET, HEAD"),
            };
        }
        if path.starts_with("/api/") && !authorized(&request, &self.config) {
            return unauthorized();
        }
        match (request.method().as_str(), path.as_str()) {
            ("GET", "/api/messages") => match list_webmail_messages(&self.config) {
                Ok(body) => json_response(&body),
                Err(error) => server_error(&error.to_string()),
            },
            ("GET", path) if path.starts_with("/api/message/") && path.contains("/attachment/") => {
                let rest = &path["/api/message/".len()..];
                let Some((id, index)) = rest.split_once("/attachment/") else {
                    return Response::not_found();
                };
                let id = percent_decode(id);
                let index = index.parse::<usize>().unwrap_or(usize::MAX);
                match read_webmail_attachment(&self.config, &id, index) {
                    Ok((attachment, data)) => attachment_response(&attachment, data),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => Response::not_found(),
                    Err(error) => server_error(&error.to_string()),
                }
            }
            ("GET", path) if path.starts_with("/api/message/") => {
                let id = percent_decode(&path["/api/message/".len()..]);
                match read_webmail_message(&self.config, &id) {
                    Ok(body) => json_response(&body),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => Response::not_found(),
                    Err(error) => server_error(&error.to_string()),
                }
            }
            ("POST", path) if path.starts_with("/api/message/") => {
                let rest = &path["/api/message/".len()..];
                let Some((id, action)) = rest.rsplit_once('/') else {
                    return Response::not_found();
                };
                let id = percent_decode(id);
                match apply_message_action(&self.config, &id, action) {
                    Ok(()) => json_response(r#"{"ok":true}"#),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => Response::not_found(),
                    Err(error) => server_error(&error.to_string()),
                }
            }
            ("POST", "/api/send") => match send_webmail_message(&self.config, &request) {
                Ok(()) => json_response(r#"{"ok":true}"#),
                Err(error) => server_error(&error.to_string()),
            },
            _ => Response::not_found(),
        }
    }
}

#[derive(Clone)]
struct SiteRouter {
    webmail: WebmailHandler,
    blog_host: Option<String>,
    blog: Option<BlogHandler>,
}

impl SiteRouter {
    fn new(webmail: WebmailHandler, blog: Option<BlogMount>) -> Self {
        let (blog_host, blog) = match blog {
            Some(blog) => {
                let base_url = if blog.base_url.is_empty() {
                    format!("https://{}", blog.host)
                } else {
                    blog.base_url
                };
                let config = BlogConfig {
                    root: blog.root,
                    static_root: blog.static_root,
                    bind_addr: String::new(),
                    title: blog.title,
                    description: blog.description,
                    base_url,
                };
                (
                    Some(normalize_host(&blog.host)),
                    Some(BlogHandler::new(config)),
                )
            }
            None => (None, None),
        };
        Self {
            webmail,
            blog_host,
            blog,
        }
    }

    fn handle_sync(&self, request: Request) -> Response {
        let host = request_host(&request);
        if self.blog_host.as_deref() == host.as_deref() {
            if let Some(blog) = &self.blog {
                return blog.handle_sync(request);
            }
        }
        self.webmail.handle_sync(request)
    }
}

impl Handler for SiteRouter {
    fn handle(&self, request: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        Box::pin(async move { self.handle_sync(request) })
    }
}

fn request_host(request: &Request) -> Option<String> {
    request
        .headers()
        .get("Host")
        .map(|value| normalize_host(value.as_str()))
}

fn normalize_host(host: &str) -> String {
    host.split(':').next().unwrap_or(host).to_ascii_lowercase()
}

struct HttpsRedirectHandler {
    hostname: String,
}

impl HttpsRedirectHandler {
    fn new(hostname: String) -> Self {
        Self { hostname }
    }
}

impl Handler for HttpsRedirectHandler {
    fn handle(&self, request: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        Box::pin(async move {
            if request.method().as_str() != "GET" && request.method().as_str() != "HEAD" {
                return method_not_allowed("GET, HEAD");
            }
            let target = request.uri().request_target();
            let host = request
                .headers()
                .get("Host")
                .map(|value| value.as_str())
                .unwrap_or(&self.hostname);
            let location = format!("https://{}{}", host, target);
            Response::text(StatusCode::new(308).unwrap(), "")
                .with_header("Location", &location)
                .with_header("Cache-Control", "no-store")
                .with_header("X-Content-Type-Options", "nosniff")
        })
    }
}

impl Handler for WebmailHandler {
    fn handle(&self, request: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        Box::pin(async move { self.handle_sync(request) })
    }
}

fn authorized(request: &Request, config: &WebmailConfig) -> bool {
    let Some(header) = request.headers().get("authorization") else {
        return false;
    };
    let value = header.as_str();
    let Some(encoded) = value.strip_prefix("Basic ") else {
        return false;
    };
    let Ok(decoded) = standard_decode(encoded) else {
        return false;
    };
    let Ok(credentials) = String::from_utf8(decoded) else {
        return false;
    };
    credentials == format!("{}:{}", config.username, config.password)
        || credentials == format!("{}:{}", config.address, config.password)
}

fn unauthorized() -> Response {
    Response::text(StatusCode::new(401).unwrap(), "authentication required")
        .with_header("WWW-Authenticate", r#"Basic realm="Edgerun Mail""#)
        .with_header("Cache-Control", "no-store")
        .with_header("X-Content-Type-Options", "nosniff")
}

fn method_not_allowed(allow: &str) -> Response {
    Response::text(StatusCode::new(405).unwrap(), "method not allowed")
        .with_header("Allow", allow)
        .with_header("Cache-Control", "no-store")
        .with_header("X-Content-Type-Options", "nosniff")
}

fn html_response(body: &str) -> Response {
    Response::html(StatusCode::OK, body)
        .with_header("Cache-Control", "no-store")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_header("X-Frame-Options", "DENY")
        .with_header("Referrer-Policy", "no-referrer")
        .with_header("Permissions-Policy", "camera=(), microphone=(), geolocation=()")
        .with_header(
            "Strict-Transport-Security",
            "max-age=31536000; includeSubDomains",
        )
        .with_header(
            "Content-Security-Policy",
            "default-src 'self'; base-uri 'none'; object-src 'none'; frame-ancestors 'none'; img-src 'self' data:; style-src 'unsafe-inline'; script-src 'unsafe-inline'; connect-src 'self'; form-action 'self'",
        )
}

fn json_response(body: &str) -> Response {
    Response::json(StatusCode::OK, body)
        .with_header("Cache-Control", "no-store")
        .with_header("X-Content-Type-Options", "nosniff")
}

fn svg_response(body: &str) -> Response {
    Response::new(StatusCode::OK)
        .with_header("Content-Type", "image/svg+xml")
        .with_header("Cache-Control", "public, max-age=3600")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_body(body.as_bytes().to_vec())
}

fn mta_sts_policy_response() -> Response {
    Response::new(StatusCode::OK)
        .with_header("Content-Type", "text/plain; charset=utf-8")
        .with_header("Cache-Control", "public, max-age=3600")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_body(MTA_STS_POLICY.as_bytes().to_vec())
}

fn attachment_response(attachment: &MailAttachment, data: Vec<u8>) -> Response {
    Response::new(StatusCode::OK)
        .with_header("Content-Type", &attachment.content_type)
        .with_header(
            "Content-Disposition",
            &format!(
                r#"attachment; filename="{}""#,
                sanitize_quoted_header(&attachment.name)
            ),
        )
        .with_header("Cache-Control", "no-store")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_body(data)
}

fn server_error(message: &str) -> Response {
    let body = format!(r#"{{"ok":false,"error":"{}"}}"#, json_escape(message));
    Response::json(StatusCode::INTERNAL_SERVER_ERROR, &body)
        .with_header("Cache-Control", "no-store")
        .with_header("X-Content-Type-Options", "nosniff")
}

fn list_webmail_messages(config: &WebmailConfig) -> io::Result<String> {
    let mut messages = Vec::new();
    collect_maildir_entries(config, "new", &mut messages)?;
    collect_maildir_entries(config, "cur", &mut messages)?;
    messages.sort_by(|a, b| b.modified.cmp(&a.modified));
    messages.truncate(100);

    let mut out = String::from(r#"{"messages":["#);
    for (idx, message) in messages.iter().enumerate() {
        let raw = std::fs::read_to_string(&message.path).unwrap_or_default();
        if idx > 0 {
            out.push(',');
        }
        let warning = auth_warning_reason(&raw);
        let attachment_count = message_attachments(&raw).len();
        out.push_str(&format!(
            r#"{{"id":"{}","from":"{}","to":"{}","subject":"{}","date":"{}","state":"{}","unread":{},"warning":{},"warningReason":"{}","attachmentCount":{},"preview":"{}"}}"#,
            json_escape(&message.id),
            json_escape(header_value(&raw, "From").unwrap_or_default().as_str()),
            json_escape(header_value(&raw, "To").unwrap_or_default().as_str()),
            json_escape(header_value(&raw, "Subject").unwrap_or_else(|| "(no subject)".to_string()).as_str()),
            json_escape(header_value(&raw, "Date").unwrap_or_default().as_str()),
            message.state,
            if message.state == "new" { "true" } else { "false" },
            if warning.is_some() { "true" } else { "false" },
            json_escape(warning.unwrap_or_default().as_str()),
            attachment_count,
            json_escape(&message_preview(&raw)),
        ));
    }
    out.push_str("]}");
    Ok(out)
}

fn read_webmail_message(config: &WebmailConfig, id: &str) -> io::Result<String> {
    let path = find_maildir_message(config, id)?;
    let raw = std::fs::read_to_string(path)?;
    let warning = auth_warning_reason(&raw);
    let attachments = message_attachments(&raw);
    let mut attachments_json = String::new();
    for (idx, attachment) in attachments.iter().enumerate() {
        if idx > 0 {
            attachments_json.push(',');
        }
        attachments_json.push_str(&format!(
            r#"{{"index":{},"name":"{}","contentType":"{}","size":{}}}"#,
            attachment.index,
            json_escape(&attachment.name),
            json_escape(&attachment.content_type),
            attachment.size,
        ));
    }
    Ok(format!(
        r#"{{"id":"{}","from":"{}","to":"{}","subject":"{}","date":"{}","warning":{},"warningReason":"{}","attachments":[{}],"body":"{}","raw":"{}"}}"#,
        json_escape(id),
        json_escape(header_value(&raw, "From").unwrap_or_default().as_str()),
        json_escape(header_value(&raw, "To").unwrap_or_default().as_str()),
        json_escape(
            header_value(&raw, "Subject")
                .unwrap_or_else(|| "(no subject)".to_string())
                .as_str()
        ),
        json_escape(header_value(&raw, "Date").unwrap_or_default().as_str()),
        if warning.is_some() { "true" } else { "false" },
        json_escape(warning.unwrap_or_default().as_str()),
        attachments_json,
        json_escape(&message_body(&raw)),
        json_escape(&raw),
    ))
}

fn read_webmail_attachment(
    config: &WebmailConfig,
    id: &str,
    index: usize,
) -> io::Result<(MailAttachment, Vec<u8>)> {
    let path = find_maildir_message(config, id)?;
    let raw = std::fs::read_to_string(path)?;
    for attachment in message_attachments(&raw) {
        if attachment.index == index {
            let data = decode_attachment_body(&attachment)?;
            return Ok((attachment, data));
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "attachment not found",
    ))
}

fn apply_message_action(config: &WebmailConfig, id: &str, action: &str) -> io::Result<()> {
    match action {
        "read" => move_maildir_message(config, id, "cur"),
        "unread" => move_maildir_message(config, id, "new"),
        "delete" => {
            let path = find_maildir_message(config, id)?;
            std::fs::remove_file(path)
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported message action: {action}"),
        )),
    }
}

fn send_webmail_message(config: &WebmailConfig, request: &Request) -> io::Result<()> {
    let body = request.body().unwrap_or_default();
    let value: JsonValue = edgerun_config::edgerun_json::from_json_slice(body)
        .map_err(|error| invalid_config(format!("invalid JSON: {error}")))?;
    let to = json_field(&value, "to")?;
    let subject = json_field(&value, "subject")?;
    let text = json_field(&value, "body")?;
    let attachments = json_attachments(&value)?;
    if !to.contains('@') || to.contains('\r') || to.contains('\n') {
        return Err(invalid_config("invalid recipient"));
    }
    let message_id = unique_webmail_id();
    let message = if attachments.is_empty() {
        format!(
            "From: {}\r\nTo: {}\r\nSubject: {}\r\nDate: {}\r\nMessage-ID: <{}@{}>\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\n{}\r\n",
            config.address,
            sanitize_header(&to),
            sanitize_header(&subject),
            http_date_now(),
            message_id,
            config.hostname,
            normalize_crlf(&text),
        )
    } else {
        build_multipart_message(config, &to, &subject, &text, &message_id, &attachments)
    };
    let smtp_addr = config.smtp_addr.clone();
    let hostname = config.hostname.clone();
    let from = config.address.clone();
    std::thread::spawn(move || {
        if let Err(error) = submit_smtp(&smtp_addr, &hostname, &from, &to, &message) {
            eprintln!("edgerun-server: webmail SMTP submit failed: {error}");
        }
    });
    Ok(())
}

fn json_field(value: &JsonValue, key: &str) -> io::Result<String> {
    value
        .get(key)
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| invalid_config(format!("missing JSON field: {key}")))
}

#[derive(Clone)]
struct OutgoingAttachment {
    name: String,
    content_type: String,
    data: Vec<u8>,
}

fn json_attachments(value: &JsonValue) -> io::Result<Vec<OutgoingAttachment>> {
    let Some(items) = value.get("attachments").and_then(JsonValue::as_array) else {
        return Ok(Vec::new());
    };
    if items.len() > 8 {
        return Err(invalid_config("too many attachments"));
    }
    let mut attachments = Vec::new();
    let mut total = 0usize;
    for item in items {
        let name = item
            .get("name")
            .and_then(JsonValue::as_str)
            .map(sanitize_attachment_name)
            .unwrap_or_else(|| "attachment.bin".to_string());
        let content_type = item
            .get("contentType")
            .and_then(JsonValue::as_str)
            .map(sanitize_content_type)
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let data64 = item
            .get("data")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| invalid_config("attachment missing data"))?;
        let data = standard_decode(data64).map_err(invalid_config)?;
        total = total.saturating_add(data.len());
        if total > 15 * 1024 * 1024 {
            return Err(invalid_config("attachments exceed 15 MiB"));
        }
        attachments.push(OutgoingAttachment {
            name,
            content_type,
            data,
        });
    }
    Ok(attachments)
}

fn build_multipart_message(
    config: &WebmailConfig,
    to: &str,
    subject: &str,
    text: &str,
    message_id: &str,
    attachments: &[OutgoingAttachment],
) -> String {
    let boundary = format!("edgerun-{}-mixed", message_id);
    let mut message = format!(
        "From: {}\r\nTo: {}\r\nSubject: {}\r\nDate: {}\r\nMessage-ID: <{}@{}>\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=\"{}\"\r\n\r\n",
        config.address,
        sanitize_header(to),
        sanitize_header(subject),
        http_date_now(),
        message_id,
        config.hostname,
        boundary,
    );
    message.push_str(&format!(
        "--{}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\n{}\r\n",
        boundary,
        normalize_crlf(text),
    ));
    for attachment in attachments {
        let encoded = String::from_utf8(standard_encode_wrapped(&attachment.data))
            .unwrap_or_else(|_| String::new());
        message.push_str(&format!(
            "--{}\r\nContent-Type: {}; name=\"{}\"\r\nContent-Disposition: attachment; filename=\"{}\"\r\nContent-Transfer-Encoding: base64\r\n\r\n{}\r\n",
            boundary,
            attachment.content_type,
            sanitize_quoted_header(&attachment.name),
            sanitize_quoted_header(&attachment.name),
            encoded,
        ));
    }
    message.push_str(&format!("--{}--\r\n", boundary));
    message
}

#[derive(Clone)]
struct WebmailMessage {
    id: String,
    path: PathBuf,
    state: &'static str,
    modified: std::time::SystemTime,
}

fn collect_maildir_entries(
    config: &WebmailConfig,
    state: &'static str,
    messages: &mut Vec<WebmailMessage>,
) -> io::Result<()> {
    let dir = config.maildir_root.join(&config.username).join(state);
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        messages.push(WebmailMessage {
            id: name.to_string(),
            path,
            state,
            modified,
        });
    }
    Ok(())
}

fn find_maildir_message(config: &WebmailConfig, id: &str) -> io::Result<PathBuf> {
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(io::Error::new(io::ErrorKind::NotFound, "message not found"));
    }
    for state in ["new", "cur"] {
        let path = config
            .maildir_root
            .join(&config.username)
            .join(state)
            .join(id);
        if path.is_file() {
            return Ok(path);
        }
    }
    Err(io::Error::new(io::ErrorKind::NotFound, "message not found"))
}

fn move_maildir_message(config: &WebmailConfig, id: &str, target_state: &str) -> io::Result<()> {
    let path = find_maildir_message(config, id)?;
    let target_dir = config
        .maildir_root
        .join(&config.username)
        .join(target_state);
    std::fs::create_dir_all(&target_dir)?;
    let target = target_dir.join(id);
    if path == target {
        return Ok(());
    }
    std::fs::rename(path, target)
}

fn submit_smtp(addr: &str, hostname: &str, from: &str, to: &str, message: &str) -> io::Result<()> {
    let mut stream = std::net::TcpStream::connect(addr)?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    expect_smtp(&mut reader, 220)?;
    smtp_command(
        &mut stream,
        &mut reader,
        &format!("EHLO {hostname}\r\n"),
        250,
    )?;
    smtp_command(
        &mut stream,
        &mut reader,
        &format!("MAIL FROM:<{from}>\r\n"),
        250,
    )?;
    smtp_command(
        &mut stream,
        &mut reader,
        &format!("RCPT TO:<{to}>\r\n"),
        250,
    )?;
    smtp_command(&mut stream, &mut reader, "DATA\r\n", 354)?;
    stream.write_all(dot_stuffed(message).as_bytes())?;
    stream.write_all(b"\r\n.\r\n")?;
    stream.flush()?;
    expect_smtp(&mut reader, 250)?;
    let _ = smtp_command(&mut stream, &mut reader, "QUIT\r\n", 221);
    Ok(())
}

fn smtp_command(
    stream: &mut std::net::TcpStream,
    reader: &mut BufReader<std::net::TcpStream>,
    command: &str,
    expected: u16,
) -> io::Result<()> {
    stream.write_all(command.as_bytes())?;
    stream.flush()?;
    expect_smtp(reader, expected)
}

fn expect_smtp(reader: &mut BufReader<std::net::TcpStream>, expected: u16) -> io::Result<()> {
    let mut line = String::new();
    loop {
        line.clear();
        let n = match reader.read_line(&mut line) {
            Ok(n) => n,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(25));
                continue;
            }
            Err(error) => return Err(error),
        };
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "SMTP closed"));
        }
        if line.len() < 4 {
            continue;
        }
        let code = line[..3].parse::<u16>().unwrap_or(0);
        let more = line.as_bytes().get(3) == Some(&b'-');
        if !more {
            if code == expected {
                return Ok(());
            }
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("SMTP expected {expected}, got {}", line.trim_end()),
            ));
        }
    }
}

fn header_value(raw: &str, name: &str) -> Option<String> {
    let mut current_name = String::new();
    let mut current_value = String::new();
    for line in raw.lines() {
        if line.is_empty() || line == "\r" {
            break;
        }
        if line.starts_with(' ') || line.starts_with('\t') {
            if current_name.eq_ignore_ascii_case(name) {
                current_value.push(' ');
                current_value.push_str(line.trim());
            }
            continue;
        }
        if current_name.eq_ignore_ascii_case(name) {
            return Some(current_value.trim().to_string());
        }
        if let Some((left, right)) = line.split_once(':') {
            current_name.clear();
            current_name.push_str(left.trim());
            current_value.clear();
            current_value.push_str(right.trim());
        }
    }
    if current_name.eq_ignore_ascii_case(name) {
        Some(current_value.trim().to_string())
    } else {
        None
    }
}

fn message_body(raw: &str) -> String {
    if let Some(boundary) =
        header_value(raw, "Content-Type").and_then(|value| header_param(&value, "boundary"))
    {
        for part in multipart_parts(raw, &boundary) {
            let headers = part_headers(&part);
            let content_type = header_value(&headers, "Content-Type")
                .unwrap_or_else(|| "text/plain".to_string())
                .to_ascii_lowercase();
            let disposition = header_value(&headers, "Content-Disposition")
                .unwrap_or_default()
                .to_ascii_lowercase();
            if content_type.starts_with("text/plain") && !disposition.contains("attachment") {
                return part_body(&part).trim().to_string();
            }
        }
    }
    raw.split_once("\r\n\r\n")
        .or_else(|| raw.split_once("\n\n"))
        .map(|(_, body)| body.trim().to_string())
        .unwrap_or_default()
}

fn message_preview(raw: &str) -> String {
    let body = message_body(raw).replace('\r', " ").replace('\n', " ");
    let mut preview = String::new();
    for ch in body.chars().take(180) {
        preview.push(ch);
    }
    preview
}

#[derive(Clone)]
struct MailAttachment {
    index: usize,
    name: String,
    content_type: String,
    transfer_encoding: String,
    body: String,
    size: usize,
}

fn message_attachments(raw: &str) -> Vec<MailAttachment> {
    let Some(boundary) =
        header_value(raw, "Content-Type").and_then(|value| header_param(&value, "boundary"))
    else {
        return Vec::new();
    };
    let mut attachments = Vec::new();
    for part in multipart_parts(raw, &boundary) {
        let headers = part_headers(&part);
        let disposition = header_value(&headers, "Content-Disposition").unwrap_or_default();
        let content_type = header_value(&headers, "Content-Type")
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let name =
            header_param(&disposition, "filename").or_else(|| header_param(&content_type, "name"));
        let is_attachment =
            disposition.to_ascii_lowercase().contains("attachment") || name.is_some();
        if !is_attachment {
            continue;
        }
        let transfer_encoding = header_value(&headers, "Content-Transfer-Encoding")
            .unwrap_or_default()
            .to_ascii_lowercase();
        let body = part_body(&part).trim().to_string();
        let size = if transfer_encoding == "base64" {
            standard_decode(
                &body
                    .chars()
                    .filter(|ch| !ch.is_whitespace())
                    .collect::<String>(),
            )
            .map(|data| data.len())
            .unwrap_or(0)
        } else {
            body.len()
        };
        attachments.push(MailAttachment {
            index: attachments.len(),
            name: name.unwrap_or_else(|| "attachment.bin".to_string()),
            content_type: content_type
                .split(';')
                .next()
                .unwrap_or("application/octet-stream")
                .trim()
                .to_string(),
            transfer_encoding,
            body,
            size,
        });
    }
    attachments
}

fn decode_attachment_body(attachment: &MailAttachment) -> io::Result<Vec<u8>> {
    if attachment.transfer_encoding == "base64" {
        let compact = attachment
            .body
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .collect::<String>();
        standard_decode(&compact).map_err(invalid_config)
    } else {
        Ok(attachment.body.as_bytes().to_vec())
    }
}

fn multipart_parts(raw: &str, boundary: &str) -> Vec<String> {
    let Some((_, body)) = raw
        .split_once("\r\n\r\n")
        .or_else(|| raw.split_once("\n\n"))
    else {
        return Vec::new();
    };
    let marker = format!("--{boundary}");
    body.split(&marker)
        .filter_map(|part| {
            let part = part.trim_start_matches(['\r', '\n']).trim_end();
            if part.is_empty() || part.starts_with("--") {
                None
            } else {
                Some(part.to_string())
            }
        })
        .collect()
}

fn part_headers(part: &str) -> String {
    part.split_once("\r\n\r\n")
        .or_else(|| part.split_once("\n\n"))
        .map(|(headers, _)| headers.to_string())
        .unwrap_or_default()
}

fn part_body(part: &str) -> &str {
    part.split_once("\r\n\r\n")
        .or_else(|| part.split_once("\n\n"))
        .map(|(_, body)| body)
        .unwrap_or_default()
}

fn header_param(value: &str, param: &str) -> Option<String> {
    for segment in value.split(';').skip(1) {
        if let Some((key, val)) = segment.split_once('=') {
            if key.trim().eq_ignore_ascii_case(param) {
                return Some(val.trim().trim_matches('"').trim_matches('\'').to_string());
            }
        }
    }
    None
}

fn auth_warning_reason(raw: &str) -> Option<String> {
    let auth = header_value(raw, "Authentication-Results")
        .or_else(|| header_value(raw, "X-Authentication-Results"))
        .unwrap_or_default()
        .to_ascii_lowercase();
    for marker in [
        "dmarc=fail",
        "spf=fail",
        "dkim=fail",
        "dmarc=permerror",
        "spf=permerror",
    ] {
        if auth.contains(marker) {
            return Some(format!("sender authentication reported {marker}"));
        }
    }

    let from_domain = header_value(raw, "From").and_then(|value| email_domain(&value));
    let return_path_domain =
        header_value(raw, "Return-Path").and_then(|value| email_domain(&value));
    if let (Some(from), Some(return_path)) = (from_domain, return_path_domain) {
        if !domains_align(&from, &return_path) {
            return Some(format!(
                "From domain {from} does not align with Return-Path domain {return_path}"
            ));
        }
    }
    None
}

fn email_domain(value: &str) -> Option<String> {
    let end = value.rfind('@')?;
    let domain = value[end + 1..]
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '.' && ch != '-')
        .to_ascii_lowercase();
    if domain.contains('.') {
        Some(domain)
    } else {
        None
    }
}

fn domains_align(left: &str, right: &str) -> bool {
    left == right || left.ends_with(&format!(".{right}")) || right.ends_with(&format!(".{left}"))
}

fn json_escape(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => {}
            ch => out.push(ch),
        }
    }
    out
}

fn percent_decode(value: &str) -> String {
    let mut out = Vec::new();
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(a), Some(b)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2])) {
                out.push((a << 4) | b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn sanitize_header(value: &str) -> String {
    value.replace(['\r', '\n'], " ").trim().to_string()
}

fn sanitize_quoted_header(value: &str) -> String {
    sanitize_header(value).replace(['"', '\\'], "_")
}

fn sanitize_attachment_name(value: &str) -> String {
    let name = value
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("attachment.bin")
        .chars()
        .filter(|ch| ch.is_ascii_graphic() || *ch == ' ')
        .collect::<String>();
    let name = name.trim();
    if name.is_empty() {
        "attachment.bin".to_string()
    } else {
        name.chars().take(120).collect()
    }
}

fn sanitize_content_type(value: &str) -> String {
    let value = sanitize_header(value);
    if value.contains('/')
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '-' | '+' | '.'))
    {
        value
    } else {
        "application/octet-stream".to_string()
    }
}

fn normalize_crlf(value: &str) -> String {
    value
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\n', "\r\n")
}

fn dot_stuffed(value: &str) -> String {
    let mut out = String::new();
    for line in value.replace("\r\n", "\n").split('\n') {
        if line.starts_with('.') {
            out.push('.');
        }
        out.push_str(line);
        out.push_str("\r\n");
    }
    out.trim_end_matches("\r\n").to_string()
}

fn unique_webmail_id() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_micros())
        .unwrap_or(0);
    format!("webmail-{now}")
}

fn http_date_now() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    edgerun_encoding::rfc2822::format_rfc2822_utc(seconds)
}

const WEBMAIL_HTML: &str = r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Edgerun Mail</title>
<link rel="icon" href='data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><text y="76" font-size="76">📧</text></svg>'>
<style>
:root{color-scheme:light dark;--bg:#f7f3eb;--panel:#fffdf8;--panel2:#f1eadc;--ink:#1c2430;--text:#1c2430;--muted:#627084;--line:#d8cfc0;--accent:#146c63;--accent-ink:#f4fffb;--accent2:#8b3f2f;--danger:#b42342;--code:#eee6d8}
:root[data-theme=dark]{--bg:#101418;--panel:#171d22;--panel2:#232b31;--ink:#f2ede4;--text:#f2ede4;--muted:#a5b2bf;--line:#2b353d;--accent:#6fc7b8;--accent-ink:#06201d;--accent2:#dfa06b;--danger:#fb7185;--code:#232b31}
*{box-sizing:border-box}body{margin:0;font:14px/1.45 ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;background:var(--bg);color:var(--ink)}
.app{height:100vh;display:grid;grid-template-columns:minmax(280px,380px) 1fr}
.list{border-right:1px solid var(--line);background:var(--bg);display:flex;flex-direction:column;min-width:0}
.top{min-height:56px;display:grid;grid-template-columns:1fr auto;gap:10px;padding:10px 14px;border-bottom:1px solid var(--line)}
.brand{font-weight:700;font-size:16px}.who{color:var(--muted);font-size:12px;align-self:center;text-align:right}.mail-count{color:var(--muted);font-size:12px}
.search{grid-column:1/3;position:relative}.search svg{position:absolute;left:10px;top:50%;transform:translateY(-50%);width:15px;height:15px;color:var(--muted);stroke:currentColor;fill:none;stroke-width:2}.search input{padding-left:34px;height:34px}
button{border:1px solid var(--line);background:var(--panel2);border-radius:6px;padding:8px 10px;cursor:pointer;color:var(--ink);transition:background .12s ease,border-color .12s ease,opacity .12s ease}
button:hover:not(:disabled){border-color:var(--accent)}button.primary{background:var(--accent);border-color:var(--accent);color:var(--accent-ink);font-weight:650}button.danger:hover:not(:disabled){border-color:var(--danger);color:var(--danger)}button:disabled{opacity:.38;cursor:not-allowed}
.icon{width:32px;height:32px;padding:0;display:inline-grid;place-items:center}.icon svg{width:17px;height:17px;stroke:currentColor;fill:none;stroke-width:2;stroke-linecap:round;stroke-linejoin:round}.copy{font-size:13px}
.messages{overflow:auto;min-height:0}.item{padding:12px 14px;border-bottom:1px solid var(--line);cursor:pointer;transition:background .12s ease,opacity .12s ease;display:grid;grid-template-columns:22px 1fr;gap:2px 8px}
.item:hover,.item.active,.item.checked{background:color-mix(in srgb,var(--accent) 12%,var(--panel))}.item.pending{opacity:.55}.item.unread .from,.item.unread .subject{font-weight:750}.item:not(.unread) .from,.item:not(.unread) .subject{font-weight:450}.pick{grid-row:1/4;align-self:start;margin:2px 0 0;width:auto}.from{white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.subject{margin-top:2px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}.preview{margin-top:4px;color:var(--muted);font-size:12px;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden}.paperclip{color:var(--muted);margin-left:5px}.paperclip svg{width:13px;height:13px;stroke:currentColor;fill:none;stroke-width:2;vertical-align:-2px}
.warn{color:var(--accent2);font-weight:700;margin-right:5px}.item .warn{font-size:13px}.warn svg,.warning svg{stroke:currentColor;fill:none;stroke-width:2;stroke-linecap:round;stroke-linejoin:round;vertical-align:-2px}
.pane{min-width:0;display:grid;grid-template-rows:auto 1fr;background:var(--panel)}
.actions{height:56px;display:flex;align-items:center;gap:8px;padding:0 16px;border-bottom:1px solid var(--line)}
.content{overflow:auto;padding:22px;max-width:980px;width:100%}.empty{color:var(--muted);margin-top:20vh;text-align:center}
h1{font-size:22px;margin:0 0 8px}.meta{color:var(--muted);margin-bottom:18px;display:grid;gap:3px}
.meta-line{display:flex;align-items:center;gap:7px;flex-wrap:wrap}.warning{border:1px solid var(--accent2);background:color-mix(in srgb,var(--accent2) 12%,var(--panel));color:var(--accent2);border-radius:6px;padding:9px 10px;margin:0 0 14px}
pre{white-space:pre-wrap;word-break:break-word;font:14px/1.5 ui-monospace,SFMono-Regular,Menlo,monospace;background:var(--code);border:1px solid var(--line);border-radius:6px;padding:14px}
.compose{display:none;padding:16px;border-bottom:1px solid var(--line);background:var(--panel)}.compose.open{display:grid;gap:10px}
.compose-grid{display:grid;grid-template-columns:96px 1fr;gap:10px;align-items:center}.compose-grid textarea,.compose-grid .attach-row{grid-column:1/3}
label{color:var(--muted);font-size:12px;text-transform:uppercase;letter-spacing:.04em}
input,textarea,select{width:100%;border:1px solid var(--line);border-radius:6px;padding:10px;font:inherit;background:var(--bg);color:var(--ink)}
input:focus,textarea:focus{outline:2px solid rgba(45,212,191,.28);border-color:var(--accent)}textarea{min-height:190px;resize:vertical}.row{display:flex;gap:8px;align-items:center}.status{color:var(--muted);font-size:13px;min-width:82px}.status.warn{color:var(--accent2)}.grow{flex:1}.attachments{display:flex;flex-wrap:wrap;gap:6px}.chip,.attachment{border:1px solid var(--line);border-radius:6px;background:var(--code);color:var(--muted);padding:5px 8px;font-size:12px}.attachment{display:inline-flex;align-items:center;gap:7px;color:var(--ink);text-decoration:none;margin:0 6px 6px 0}.chip button{border:0;background:transparent;color:var(--muted);padding:0;margin-left:6px;width:auto;height:auto}.attach-input{display:none}
.spin svg{animation:spin .8s linear infinite}@keyframes spin{to{transform:rotate(360deg)}}
@media(max-width:760px){.app{grid-template-columns:1fr;grid-template-rows:45vh 55vh}.list{border-right:0;border-bottom:1px solid var(--line)}.content{padding:16px}}
</style>
</head>
<body>
<svg aria-hidden="true" style="position:absolute;width:0;height:0;overflow:hidden">
<symbol id="i-refresh" viewBox="0 0 24 24"><path d="M21 12a9 9 0 0 1-15.4 6.4L3 16"/><path d="M3 21v-5h5"/><path d="M3 12A9 9 0 0 1 18.4 5.6L21 8"/><path d="M21 3v5h-5"/></symbol>
<symbol id="i-edit" viewBox="0 0 24 24"><path d="M12 20h9"/><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z"/></symbol>
<symbol id="i-send" viewBox="0 0 24 24"><path d="m22 2-7 20-4-9-9-4Z"/><path d="M22 2 11 13"/></symbol>
<symbol id="i-reply" viewBox="0 0 24 24"><path d="m9 17-6-5 6-5"/><path d="M3 12h11a7 7 0 0 1 7 7v1"/></symbol>
<symbol id="i-x" viewBox="0 0 24 24"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></symbol>
<symbol id="i-copy" viewBox="0 0 24 24"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></symbol>
<symbol id="i-trash" viewBox="0 0 24 24"><path d="M3 6h18"/><path d="M8 6V4h8v2"/><path d="m19 6-1 14H6L5 6"/><path d="M10 11v6"/><path d="M14 11v6"/></symbol>
<symbol id="i-mail-open" viewBox="0 0 24 24"><path d="M4 6 12 2l8 4v14H4Z"/><path d="m4 8 8 6 8-6"/></symbol>
<symbol id="i-mail" viewBox="0 0 24 24"><rect x="3" y="5" width="18" height="14" rx="2"/><path d="m3 7 9 6 9-6"/></symbol>
<symbol id="i-log-out" viewBox="0 0 24 24"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><path d="M16 17l5-5-5-5"/><path d="M21 12H9"/></symbol>
<symbol id="i-alert" viewBox="0 0 24 24"><path d="m21.7 18-8-14a2 2 0 0 0-3.4 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.7-3Z"/><path d="M12 9v4"/><path d="M12 17h.01"/></symbol>
<symbol id="i-search" viewBox="0 0 24 24"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></symbol>
<symbol id="i-paperclip" viewBox="0 0 24 24"><path d="m21.4 11.6-8.8 8.8a6 6 0 0 1-8.5-8.5l9.2-9.2a4 4 0 0 1 5.7 5.7l-9.2 9.2a2 2 0 0 1-2.8-2.8l8.5-8.5"/></symbol>
</svg>
<main class="app">
  <section class="list">
    <div class="top"><div><div class="brand">Edgerun Mail</div><div id="mailCount" class="mail-count"></div></div><div class="who">ken@edgerun.tech</div><div class="search"><svg><use href="#i-search"/></svg><input id="search" placeholder="Search mail" autocomplete="off"></div></div>
    <div id="messages" class="messages"></div>
  </section>
  <section class="pane">
    <div class="actions">
      <button id="composeBtn" class="icon primary" title="Compose" aria-label="Compose"><svg><use href="#i-edit"/></svg></button>
      <button id="replyMsg" class="icon" title="Reply" aria-label="Reply" disabled><svg><use href="#i-reply"/></svg></button>
      <button id="refresh" class="icon" title="Refresh" aria-label="Refresh"><svg><use href="#i-refresh"/></svg></button>
      <button id="markRead" class="icon" title="Mark read" aria-label="Mark read" disabled><svg><use href="#i-mail-open"/></svg></button>
      <button id="markUnread" class="icon" title="Mark unread" aria-label="Mark unread" disabled><svg><use href="#i-mail"/></svg></button>
      <button id="deleteMsg" class="icon danger" title="Delete" aria-label="Delete" disabled><svg><use href="#i-trash"/></svg></button>
      <div id="status" class="status"></div><div class="grow"></div>
      <er-theme-toggle></er-theme-toggle>
      <button id="logout" class="icon" title="Log out" aria-label="Log out"><svg><use href="#i-log-out"/></svg></button>
    </div>
    <form id="compose" class="compose">
      <div class="compose-grid">
        <label>From</label><div>ken@edgerun.tech</div>
        <label for="to">To</label><input id="to" placeholder="recipient@example.com" autocomplete="off">
        <label for="subject">Subject</label><input id="subject" placeholder="Subject" autocomplete="off">
        <textarea id="body" placeholder="Message"></textarea>
        <div class="attach-row"><input id="filePick" class="attach-input" type="file" multiple><button id="attachBtn" class="icon" type="button" title="Attach files" aria-label="Attach files"><svg><use href="#i-paperclip"/></svg></button><div id="composeAttachments" class="attachments"></div></div>
      </div>
      <div class="row"><button id="sendBtn" class="icon primary" type="submit" title="Send" aria-label="Send"><svg><use href="#i-send"/></svg></button><button id="cancel" class="icon" type="button" title="Cancel" aria-label="Cancel"><svg><use href="#i-x"/></svg></button><div class="grow"></div></div>
    </form>
    <article id="content" class="content"><div class="empty">Select a message</div></article>
  </section>
</main>
<script>
const root=document.documentElement;
const storedTheme=localStorage.getItem('theme');
if(storedTheme){root.dataset.theme=storedTheme}
if(!customElements.get('er-theme-toggle')){customElements.define('er-theme-toggle',class extends HTMLElement{connectedCallback(){this.attachShadow({mode:'open'}).innerHTML='<style>button{width:32px;height:32px;border:1px solid var(--line);border-radius:6px;background:var(--panel2);color:var(--ink);cursor:pointer;font:inherit}button:hover{border-color:var(--accent)}</style><button type="button" aria-label="Toggle color theme">◐</button>';this.shadowRoot.querySelector('button').onclick=()=>{const next=root.dataset.theme==='dark'?'light':'dark';root.dataset.theme=next;localStorage.setItem('theme',next)}}})}
const messagesEl=document.getElementById('messages'),content=document.getElementById('content'),statusEl=document.getElementById('status'),mailCount=document.getElementById('mailCount');
const refreshBtn=document.getElementById('refresh'),composeBtn=document.getElementById('composeBtn'),replyMsg=document.getElementById('replyMsg'),composeEl=document.getElementById('compose'),searchEl=document.getElementById('search'),filePick=document.getElementById('filePick'),composeAttachments=document.getElementById('composeAttachments');
const markRead=document.getElementById('markRead'),markUnread=document.getElementById('markUnread'),deleteMsg=document.getElementById('deleteMsg'),logout=document.getElementById('logout'),attachBtn=document.getElementById('attachBtn'),cancelBtn=document.getElementById('cancel'),sendBtn=document.getElementById('sendBtn');
const toEl=document.getElementById('to'),subjectEl=document.getElementById('subject'),bodyEl=document.getElementById('body');
let selected='',messages=[],checked=new Set(),draftAttachments=[],loading=false,sending=false,statusTimer=0,openedMessage=null;
function esc(s){return (s||'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));}
function flash(s,warn){clearTimeout(statusTimer);statusEl.textContent=s||'';statusEl.classList.toggle('warn',!!warn);if(s)statusTimer=setTimeout(()=>{statusEl.textContent='';statusEl.classList.remove('warn')},1800)}
function fmtSize(n){return n>1048576?(n/1048576).toFixed(1)+' MB':n>1024?Math.round(n/1024)+' KB':n+' B'}
function copyText(v){navigator.clipboard&&navigator.clipboard.writeText(v||'');flash('Copied')}
async function api(path,opts){const r=await fetch(path,opts);if(r.status===401){statusEl.textContent='Login required';throw new Error('auth');}if(!r.ok)throw new Error(await r.text());return r.json();}
function selectedMsg(){return messages.find(m=>m.id===selected)}
function actionIds(){return checked.size?[...checked]:(selected?[selected]:[])}
function setActionState(){const ids=actionIds(),items=ids.map(id=>messages.find(m=>m.id===id)).filter(Boolean);replyMsg.disabled=!selected;markRead.disabled=!items.some(m=>m.unread);markUnread.disabled=!items.some(m=>!m.unread);deleteMsg.disabled=!items.length}
function filteredMessages(){const q=searchEl.value.trim().toLowerCase();if(!q)return messages;return messages.filter(m=>[m.from,m.to,m.subject,m.preview,m.date].some(v=>(v||'').toLowerCase().includes(q)))}
function updateDraftStore(){localStorage.setItem('webmailDraft',JSON.stringify({to:toEl.value,subject:subjectEl.value,body:bodyEl.value,attachments:draftAttachments}))}
function restoreDraft(){try{const raw=localStorage.getItem('webmailDraft');if(!raw)return;const draft=JSON.parse(raw);toEl.value=draft.to||'';subjectEl.value=draft.subject||'';bodyEl.value=draft.body||'';draftAttachments=Array.isArray(draft.attachments)?draft.attachments:[];renderDraftAttachments()}catch(e){}}
function clearDraft(){toEl.value=subjectEl.value=bodyEl.value='';draftAttachments=[];renderDraftAttachments();localStorage.removeItem('webmailDraft')}
function openCompose(){composeEl.classList.add('open');toEl.focus()}
function closeCompose(){if(toEl.value||subjectEl.value||bodyEl.value||draftAttachments.length){updateDraftStore()}composeEl.classList.remove('open')}
function renderList(){const unread=messages.filter(m=>m.unread).length;document.title=(unread?`(${unread}) `:'')+'Edgerun Mail';mailCount.textContent=`${messages.length} total · ${unread} unread`;const visible=filteredMessages();messagesEl.innerHTML=visible.map(m=>`<div class="item ${m.unread?'unread':''} ${m.pending?'pending':''} ${m.id===selected?'active':''} ${checked.has(m.id)?'checked':''}" data-id="${encodeURIComponent(m.id)}"><input class="pick" type="checkbox" aria-label="Select message" ${checked.has(m.id)?'checked':''}><div class="from">${m.warning?'<span class="warn" title="'+esc(m.warningReason)+'"><svg width="13" height="13"><use href="#i-alert"/></svg></span>':''}${esc(m.from||'(unknown)')}${m.attachmentCount?'<span class="paperclip" title="'+m.attachmentCount+' attachment(s)"><svg><use href="#i-paperclip"/></svg></span>':''}</div><div class="subject">${esc(m.subject)}</div><div class="preview">${esc(m.preview)}</div></div>`).join('')||'<div class="empty">'+(messages.length?'No matches':'No mail')+'</div>';setActionState()}
async function load(silent){if(loading)return;loading=true;refreshBtn.classList.add('spin');try{const data=await api('/api/messages');const pending=new Map(messages.filter(m=>m.pending).map(m=>[m.id,m]));messages=data.messages.map(m=>pending.get(m.id)||m);if(selected&&!messages.some(m=>m.id===selected)){selected='';openedMessage=null;content.innerHTML='<div class="empty">Select a message</div>'}renderList();if(!silent)flash('Updated')}catch(e){if(!silent)flash(e.message,true)}finally{loading=false;refreshBtn.classList.remove('spin')}}
messagesEl.onclick=e=>{const item=e.target.closest('.item');if(!item)return;const id=decodeURIComponent(item.dataset.id);if(e.target.closest('.pick')){checked.has(id)?checked.delete(id):checked.add(id);renderList();return}openMsg(id);};
async function openMsg(id){selected=id;openedMessage=null;const local=messages.find(m=>m.id===id);if(local&&local.unread){local.unread=false;renderList();api('/api/message/'+encodeURIComponent(id)+'/read',{method:'POST'}).catch(()=>{local.unread=true;renderList();flash('Could not mark read',true)})}else renderList();content.innerHTML='<div class="empty">Loading...</div>';try{const m=await api('/api/message/'+encodeURIComponent(id));if(selected!==id)return;openedMessage=m;const attachments=(m.attachments||[]).map(a=>`<a class="attachment" href="/api/message/${encodeURIComponent(id)}/attachment/${a.index}" download="${esc(a.name)}"><svg width="15" height="15"><use href="#i-paperclip"/></svg>${esc(a.name)} <span>${fmtSize(a.size)}</span></a>`).join('');content.innerHTML=`<h1>${m.warning?'<span class="warn" title="'+esc(m.warningReason)+'"><svg width="18" height="18"><use href="#i-alert"/></svg></span>':''}${esc(m.subject)}</h1>${m.warning?'<div class="warning"><svg width="16" height="16"><use href="#i-alert"/></svg> '+esc(m.warningReason)+'</div>':''}<div class="meta"><div class="meta-line">From: <span>${esc(m.from)}</span><button class="icon copy" data-copy="${esc(m.from)}" title="Copy sender" aria-label="Copy sender"><svg><use href="#i-copy"/></svg></button></div><div class="meta-line">To: <span>${esc(m.to)}</span><button class="icon copy" data-copy="${esc(m.to)}" title="Copy recipient" aria-label="Copy recipient"><svg><use href="#i-copy"/></svg></button></div><div>${esc(m.date)}</div></div>${attachments?'<div class="attachments">'+attachments+'</div>':''}<pre>${esc(m.body||m.raw)}</pre>`;setActionState()}catch(e){content.innerHTML='<div class="empty">Could not open message</div>';flash(e.message,true)}}
document.addEventListener('click',e=>{const b=e.target.closest('.copy');if(b)copyText(b.dataset.copy||'')});
refreshBtn.onclick=()=>load(false);
searchEl.oninput=renderList;
composeBtn.onclick=()=>openCompose();
replyMsg.onclick=async()=>{let m=openedMessage;if(!m&&selected)m=await api('/api/message/'+encodeURIComponent(selected));if(!m)return;toEl.value=(m.from||'').replace(/^.*<([^>]+)>.*$/,'$1');subjectEl.value=/^re:/i.test(m.subject||'')?m.subject:'Re: '+(m.subject||'');bodyEl.value=`\n\nOn ${m.date||'the original message'}, ${m.from||'sender'} wrote:\n`+(m.body||'').split('\n').map(line=>'> '+line).join('\n');draftAttachments=[];renderDraftAttachments();updateDraftStore();openCompose()};
cancelBtn.onclick=closeCompose;
async function action(name){const ids=actionIds();if(!ids.length)return;const old=messages.map(m=>({...m}));if(name==='delete'){messages=messages.filter(m=>!ids.includes(m.id));checked.clear();selected='';content.innerHTML='<div class="empty">Select a message</div>'}else messages.forEach(m=>{if(ids.includes(m.id)){m.pending=true;m.unread=name==='unread'}});renderList();try{await Promise.all(ids.map(id=>api('/api/message/'+encodeURIComponent(id)+'/'+name,{method:'POST'})));flash(name==='delete'?'Deleted':name==='read'?'Marked read':'Marked unread');load(true)}catch(e){messages=old;renderList();flash(e.message,true)}}
markRead.onclick=()=>action('read');markUnread.onclick=()=>action('unread');deleteMsg.onclick=()=>{const n=actionIds().length;if(n&&confirm('Delete '+n+' message'+(n>1?'s':'')+'?'))action('delete')};
logout.onclick=async()=>{selected='';content.innerHTML='<div class="empty">Logged out</div>';messagesEl.innerHTML='';statusEl.textContent='Logged out';try{await fetch('/api/messages',{headers:{Authorization:'Basic '+btoa('logout:logout')}})}catch(e){}};
function renderDraftAttachments(){composeAttachments.innerHTML=draftAttachments.map((a,i)=>`<span class="chip"><svg width="13" height="13"><use href="#i-paperclip"/></svg> ${esc(a.name)} ${fmtSize(a.size)} <button type="button" data-idx="${i}" title="Remove attachment" aria-label="Remove attachment">×</button></span>`).join('')}
attachBtn.onclick=()=>filePick.click();
composeAttachments.onclick=e=>{const b=e.target.closest('button');if(b){draftAttachments.splice(Number(b.dataset.idx),1);renderDraftAttachments();updateDraftStore()}};
filePick.onchange=async()=>{for(const file of filePick.files){const data=await new Promise((ok,bad)=>{const r=new FileReader();r.onload=()=>ok(String(r.result).split(',')[1]||'');r.onerror=bad;r.readAsDataURL(file)});draftAttachments.push({name:file.name,contentType:file.type||'application/octet-stream',size:file.size,data})}filePick.value='';renderDraftAttachments();updateDraftStore();flash('Attached')};
[toEl,subjectEl,bodyEl].forEach(el=>el.addEventListener('input',updateDraftStore));
composeEl.onsubmit=async e=>{e.preventDefault();if(sending)return;sending=true;sendBtn.disabled=true;const draft={to:toEl.value,subject:subjectEl.value,body:bodyEl.value,attachments:draftAttachments.map(({name,contentType,data})=>({name,contentType,data}))};flash('Sending...');composeEl.classList.remove('open');try{await api('/api/send',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(draft)});clearDraft();flash('Sent');load(true)}catch(err){composeEl.classList.add('open');flash(err.message,true)}finally{sending=false;sendBtn.disabled=false}};
restoreDraft();
load(false).catch(err=>flash(err.message,true));
setInterval(()=>{if(!document.hidden)load(true)},10000);
</script>
</body>
</html>"##;

const BIMI_LOGO_SVG: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" version="1.2" baseProfile="tiny-ps" width="256" height="256" viewBox="0 0 256 256">
  <title>Edgerun</title>
  <desc>Edgerun square email brand mark</desc>
  <rect width="256" height="256" fill="#0f172a"/>
  <path fill="#22d3ee" d="M64 56h128v32H96v24h80v32H96v24h96v32H64z"/>
  <path fill="#f8fafc" d="M112 104h64v32h-64z"/>
</svg>
"##;

const MTA_STS_POLICY: &str =
    "version: STSv1\nmode: enforce\nmx: mail.edgerun.tech\nmax_age: 604800\n";
const DNSSEC_RESIGN_INTERVAL_SECS: u64 = 12 * 60 * 60;
const DNSSEC_RESIGN_POLL_SECS: u64 = 60;

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

fn zones_have_dnssec(zone_specs: &[DnsZoneSpec]) -> bool {
    zone_specs.iter().any(|spec| spec.dnssec.is_some())
}

async fn run_dnssec_resigner(
    dns: Arc<DnsServer>,
    zone_specs: Vec<DnsZoneSpec>,
    shutdown: CancellationToken,
) -> io::Result<()> {
    let mut elapsed = 0u64;
    while !shutdown.is_cancelled() {
        edgerun_rt::sleep(Duration::from_secs(DNSSEC_RESIGN_POLL_SECS)).await;
        if shutdown.is_cancelled() {
            break;
        }
        elapsed = elapsed.saturating_add(DNSSEC_RESIGN_POLL_SECS);
        if elapsed < DNSSEC_RESIGN_INTERVAL_SECS {
            continue;
        }
        refresh_dnssec_zones(&dns, &zone_specs).await?;
        elapsed = 0;
    }
    Ok(())
}

async fn refresh_dnssec_zones(dns: &DnsServer, zone_specs: &[DnsZoneSpec]) -> io::Result<()> {
    let mut by_origin: BTreeMap<String, Vec<&DnsZoneSpec>> = BTreeMap::new();
    for zone_spec in zone_specs {
        by_origin
            .entry(zone_spec.origin.to_ascii_lowercase())
            .or_default()
            .push(zone_spec);
    }
    for specs in by_origin.values() {
        if specs.iter().any(|spec| spec.dnssec.is_some()) {
            let zone = zone_from_config(specs)?;
            let origin = zone.origin.clone();
            dns.add_zone(zone).await;
            eprintln!("edgerun-server: dnssec refreshed zone={origin}");
        }
    }
    Ok(())
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
    let mut dnssec = None;
    for spec in specs {
        if let Some(config) = &spec.dnssec {
            if dnssec.is_some() {
                return Err(invalid_config(
                    "only one DnsZone.dnssec block is allowed per origin",
                ));
            }
            dnssec = Some(config);
        }
        for record in &spec.records {
            add_zone_record(&mut zone, record, &spec.origin)?;
        }
        if let Some(wildcards) = &spec.wildcards {
            for record in wildcards {
                add_zone_record(&mut zone, record, &spec.origin)?;
            }
        }
    }
    if let Some(config) = dnssec {
        apply_dnssec(&mut zone, config)?;
    }
    Ok(zone)
}

fn apply_dnssec(zone: &mut DnsZone, config: &DnssecConfig) -> io::Result<()> {
    let algorithm = config.algorithm.to_ascii_lowercase();
    if !matches!(
        algorithm.as_str(),
        "ecdsap256" | "ecdsap256sha256" | "ecdsa-p256-sha256" | "algorithm13"
    ) {
        return Err(invalid_config(
            "DnsZone.dnssec.algorithm must be ecdsap256 for production signing",
        ));
    }
    if config.nsec3 {
        return Err(invalid_config(
            "DnsZone.dnssec.nsec3 is parsed but not yet integrated into authoritative negative answers; use NSEC for now",
        ));
    }
    let key_path = config
        .key_path
        .as_deref()
        .ok_or_else(|| invalid_config("DnsZone.dnssec.key_path is required"))?;
    let signing_key = load_or_create_dnssec_key(Path::new(key_path))?;
    let dnskey = dnskey_from_signing_key(
        zone.origin.clone(),
        config.key_flags,
        config.key_ttl,
        &signing_key,
    );
    let ds = ds_record_for_dnskey(&dnskey, config.key_ttl)?;
    let key_tag = edgerun_dns::compute_key_tag(&dnskey);
    zone.add_record(dnskey.clone());
    add_nsec_chain(zone, config.key_ttl);
    let now = dnssec_unix_time()?;
    let inception = now.saturating_sub(300);
    let expiration = now.saturating_add(config.signature_validity);
    for rrsig in
        edgerun_dns::sign_zone_ecdsap256(zone, &dnskey, &signing_key, inception, expiration)
    {
        zone.add_record(rrsig);
    }
    eprintln!(
        "edgerun-server: dnssec zone={} algorithm=13 key_tag={} ds=\"{}\"",
        zone.origin,
        key_tag,
        ds_record_text(&ds)?
    );
    Ok(())
}

fn load_or_create_dnssec_key(path: &Path) -> io::Result<edgerun_crypto::p256::ecdsa::SigningKey> {
    if path.exists() {
        let pem = std::fs::read_to_string(path)?;
        return edgerun_tls::signing_key_from_pem(&pem).map_err(|_| {
            invalid_config(format!(
                "failed to parse DNSSEC P-256 private key at {}",
                path.display()
            ))
        });
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let key = edgerun_crypto::random_p256_signing_key();
    let pem = edgerun_tls::signing_key_to_pem(&key)
        .map_err(|_| invalid_config("failed to serialize DNSSEC P-256 private key"))?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(pem.as_bytes())?;
    Ok(key)
}

fn dnskey_from_signing_key(
    name: String,
    flags: u16,
    ttl: u32,
    key: &edgerun_crypto::p256::ecdsa::SigningKey,
) -> DnsRecord {
    use edgerun_crypto::p256::elliptic_curve::sec1::ToEncodedPoint;
    let encoded = key.verifying_key().to_encoded_point(false);
    DnsRecord::dnskey(name, flags, 3, 13, encoded.as_bytes()[1..].to_vec(), ttl)
}

fn add_nsec_chain(zone: &mut DnsZone, ttl: u32) {
    let mut names: Vec<String> = zone.names().into_iter().map(str::to_string).collect();
    names.sort_by(|left, right| dnssec_canonical_name_cmp(left, right));
    names.dedup();
    if names.is_empty() {
        return;
    }
    for (index, name) in names.iter().enumerate() {
        let next = names[(index + 1) % names.len()].clone();
        let mut types: Vec<DnsRecordType> = zone
            .get_records(name)
            .into_iter()
            .map(|record| record.rtype)
            .collect();
        types.push(DnsRecordType::NSEC);
        types.push(DnsRecordType::RRSIG);
        types.sort_by_key(|rtype| rtype.as_u16());
        types.dedup();
        zone.add_record(DnsRecord::nsec(
            name.clone(),
            next,
            edgerun_dns::nsec3_type_bitmap(&types),
            ttl,
        ));
    }
}

fn dnssec_canonical_name_cmp(left: &str, right: &str) -> std::cmp::Ordering {
    let left_lower = left.trim_end_matches('.').to_ascii_lowercase();
    let right_lower = right.trim_end_matches('.').to_ascii_lowercase();
    let left_labels: Vec<&str> = left_lower.split('.').collect();
    let right_labels: Vec<&str> = right_lower.split('.').collect();
    let mut left_iter = left_labels.iter().rev();
    let mut right_iter = right_labels.iter().rev();
    loop {
        match (left_iter.next(), right_iter.next()) {
            (Some(left), Some(right)) => match left.as_bytes().cmp(right.as_bytes()) {
                std::cmp::Ordering::Equal => {}
                order => return order,
            },
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (None, None) => return std::cmp::Ordering::Equal,
        }
    }
}

fn ds_record_for_dnskey(dnskey: &DnsRecord, ttl: u32) -> io::Result<DnsRecord> {
    if let DnsRecordData::DNSKEY { algorithm, .. } = &dnskey.data {
        let mut digest_input =
            edgerun_dns::record::encode_domain_name(&dnskey.name.to_ascii_lowercase());
        digest_input.extend_from_slice(&dnskey.data.to_wire(dnskey.rtype));
        let digest = edgerun_crypto::sha256(&digest_input);
        Ok(DnsRecord::ds(
            dnskey.name.clone(),
            edgerun_dns::compute_key_tag(dnskey),
            *algorithm,
            2,
            digest.to_vec(),
            ttl,
        ))
    } else {
        Err(invalid_config(
            "DNSSEC DS generation requires a DNSKEY record",
        ))
    }
}

fn ds_record_text(ds: &DnsRecord) -> io::Result<String> {
    if let DnsRecordData::DS {
        key_tag,
        algorithm,
        digest_type,
        digest,
    } = &ds.data
    {
        Ok(format!(
            "{} IN DS {} {} {} {}",
            ds.name,
            key_tag,
            algorithm,
            digest_type,
            hex_lower(digest)
        ))
    } else {
        Err(invalid_config("expected DS record"))
    }
}

fn dnssec_unix_time() -> io::Result<u32> {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(invalid_config)?
        .as_secs();
    u32::try_from(seconds).map_err(|_| invalid_config("system time is outside DNSSEC range"))
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
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
                format!("unsupported DNS record type in server config: {rtype}"),
            ));
        }
    }
    Ok(())
}

async fn ensure_acme_certificate(
    spec: &SmtpServerSpec,
    dns: &Arc<DnsServer>,
    zones: &[DnsZoneSpec],
) -> io::Result<()> {
    let cert_dir = PathBuf::from(
        spec.acme_cert_dir
            .as_deref()
            .unwrap_or("/etc/edgerun/server/tls"),
    );
    let fullchain_path = cert_dir.join("fullchain.pem");
    let privkey_path = cert_dir.join("privkey.pem");
    let domains = spec
        .acme_domains
        .clone()
        .unwrap_or_else(|| vec![spec.hostname.clone()]);
    if !acme_certificate_needs_renewal(&fullchain_path, &privkey_path, &domains)? {
        return Ok(());
    }

    std::fs::create_dir_all(&cert_dir)?;
    let domain_refs: Vec<&str> = domains.iter().map(String::as_str).collect();
    let account_key_path = PathBuf::from(
        spec.acme_account_key_path
            .as_deref()
            .unwrap_or("/etc/edgerun/server/acme-account.pem"),
    );
    let account_key = if account_key_path.exists() {
        let pem = std::fs::read_to_string(&account_key_path)?;
        AccountKey::from_pem(&pem).map_err(acme_io_error)?
    } else {
        let key = AccountKey::generate();
        if let Some(parent) = account_key_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&account_key_path, key.pem())?;
        key
    };

    let client = AcmeClient::new(
        AcmeConfig {
            directory_url: parse_acme_directory(spec.acme_directory.as_deref())?,
            email: spec
                .acme_contact_email
                .as_deref()
                .map(|email| vec![format!("mailto:{email}")])
                .unwrap_or_default(),
            terms_of_service_agreed: true,
        },
        account_key,
    );
    client.init().await.map_err(acme_io_error)?;
    client.create_account().await.map_err(acme_io_error)?;
    let order = client.create_order(&domains).await.map_err(acme_io_error)?;

    let mut challenge_records = Vec::new();
    for auth_url in order.authorization_urls() {
        let authorization = client
            .get_authorization(auth_url)
            .await
            .map_err(acme_io_error)?;
        if authorization.status == edgerun_acme::types::AuthorizationStatus::Valid {
            continue;
        }
        let challenge = authorization
            .challenges
            .as_deref()
            .and_then(|challenges| {
                challenges
                    .iter()
                    .find(|challenge| challenge.challenge_type == ChallengeType::Dns01)
            })
            .ok_or_else(|| invalid_config("ACME authorization has no dns-01 challenge"))?;
        let token = challenge
            .token
            .as_deref()
            .ok_or_else(|| invalid_config("ACME dns-01 challenge missing token"))?;
        let domain = authorization.identifier.value.as_str();
        let dns_manager = client.dns_manager();
        let dns_challenge = dns_manager.create_challenge(domain, token);
        challenge_records.push(ZoneRecord {
            name: dns_challenge.record_name().to_string(),
            record_type: "TXT".to_string(),
            ttl: Some(60),
            value: JsonValue::String(dns_challenge.record_value().to_string()),
        });
        publish_acme_challenge_records(dns, zones, &challenge_records).await?;
        edgerun_rt::sleep(Duration::from_secs(20)).await;
        client
            .validate_challenge(&challenge.url)
            .await
            .map_err(acme_io_error)?;
        wait_for_challenge(&client, &challenge.url).await?;
    }

    let mut ready_order = client
        .get_order(&order.inner.id)
        .await
        .map_err(acme_io_error)?;
    for _ in 0..30 {
        match ready_order.status() {
            OrderStatus::Ready | OrderStatus::Valid => break,
            OrderStatus::Invalid => return Err(invalid_config("ACME order became invalid")),
            _ => {
                edgerun_rt::sleep(Duration::from_secs(2)).await;
                ready_order = client
                    .get_order(&order.inner.id)
                    .await
                    .map_err(acme_io_error)?;
            }
        }
    }
    let finalize_url = ready_order
        .finalize_url()
        .ok_or_else(|| invalid_config("ACME order missing finalize URL"))?;
    let (csr_der, signing_key) = edgerun_tls::generate_csr(&domain_refs)
        .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;
    let mut finalized = ready_order.clone();
    for attempt in 0..60 {
        if finalized.status() == OrderStatus::Valid {
            break;
        }
        match client.finalize_order(finalize_url, &csr_der).await {
            Ok(mut order) => {
                order.inner.id = ready_order.inner.id.clone();
                finalized = order;
                break;
            }
            Err(error) => {
                if acme_error_is_retryable_finalize(&error) {
                    edgerun_rt::sleep(Duration::from_secs(2)).await;
                    match client.get_order(&order.inner.id).await {
                        Ok(order) => finalized = order,
                        Err(error) if acme_error_is_retryable_finalize(&error) => continue,
                        Err(error) => return Err(acme_io_error(error)),
                    }
                    if finalized.status() == OrderStatus::Valid {
                        break;
                    }
                } else {
                    return Err(acme_io_error(error));
                }
            }
        }
        if attempt == 59 {
            return Err(invalid_config(
                "ACME order did not become ready for finalization",
            ));
        }
    }
    for _ in 0..30 {
        match finalized.status() {
            OrderStatus::Valid => break,
            OrderStatus::Invalid => {
                return Err(invalid_config("ACME finalized order became invalid"))
            }
            _ => {
                edgerun_rt::sleep(Duration::from_secs(2)).await;
                finalized = client
                    .get_order(&finalized.inner.id)
                    .await
                    .map_err(acme_io_error)?;
            }
        }
    }
    let certificate_url = finalized
        .certificate_url()
        .ok_or_else(|| invalid_config("ACME order missing certificate URL"))?;
    let cert_pem = client
        .download_certificate(certificate_url)
        .await
        .map_err(acme_io_error)?;
    let key_pem = edgerun_tls::signing_key_to_pem(&signing_key)
        .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;
    std::fs::write(&fullchain_path, cert_pem)?;
    std::fs::write(&privkey_path, key_pem)?;
    Ok(())
}

fn acme_error_is_retryable_finalize(error: &edgerun_acme::AcmeError) -> bool {
    match error {
        edgerun_acme::AcmeError::Server(_, _) => true,
        edgerun_acme::AcmeError::Protocol(message) => {
            message.contains("orderNotReady") || message.contains("not acceptable for finalization")
        }
        _ => false,
    }
}

fn acme_certificate_needs_renewal(
    fullchain_path: &Path,
    privkey_path: &Path,
    domains: &[String],
) -> io::Result<bool> {
    if !fullchain_path.exists() || !privkey_path.exists() {
        return Ok(true);
    }
    let pem = std::fs::read_to_string(fullchain_path)?;
    let cert = match edgerun_tls::certificate::Certificate::from_pem(&pem) {
        Ok(cert) => cert,
        Err(_) => return Ok(true),
    };
    for domain in domains {
        if !cert.matches_hostname(domain) {
            return Ok(true);
        }
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let renew_before = 30 * 24 * 60 * 60;
    Ok(cert.not_after <= now.saturating_add(renew_before))
}

async fn wait_for_challenge(client: &AcmeClient, url: &edgerun_url::Url) -> io::Result<()> {
    for _ in 0..30 {
        let challenge = client.get_challenge(url).await.map_err(acme_io_error)?;
        match challenge.status() {
            ChallengeStatus::Valid => return Ok(()),
            ChallengeStatus::Invalid => {
                return Err(invalid_config("ACME challenge became invalid"))
            }
            _ => edgerun_rt::sleep(Duration::from_secs(2)).await,
        }
    }
    Err(invalid_config("ACME challenge did not become valid"))
}

async fn publish_acme_challenge_records(
    dns: &Arc<DnsServer>,
    zones: &[DnsZoneSpec],
    records: &[ZoneRecord],
) -> io::Result<()> {
    let mut by_origin: BTreeMap<String, Vec<&DnsZoneSpec>> = BTreeMap::new();
    for zone_spec in zones {
        by_origin
            .entry(zone_spec.origin.to_ascii_lowercase())
            .or_default()
            .push(zone_spec);
    }
    for (origin, specs) in by_origin {
        let mut merged = Vec::new();
        for spec in specs {
            merged.push(spec);
        }
        let mut owned = merged
            .first()
            .ok_or_else(|| invalid_config("empty DNS zone group"))?
            .to_owned()
            .clone();
        owned.records.extend(
            records
                .iter()
                .filter(|record| {
                    let fqdn = normalize_record_name(&record.name, &origin);
                    fqdn == origin || fqdn.ends_with(&format!(".{origin}"))
                })
                .cloned(),
        );
        let refs = vec![&owned];
        dns.add_zone(zone_from_config(&refs)?).await;
    }
    Ok(())
}

fn parse_acme_directory(value: Option<&str>) -> io::Result<DirectoryUrl> {
    match value.unwrap_or("letsencrypt").to_ascii_lowercase().as_str() {
        "letsencrypt" | "production" => Ok(DirectoryUrl::LetsEncrypt),
        "letsencrypt-staging" | "staging" => Ok(DirectoryUrl::LetsEncryptStaging),
        other => Err(invalid_config(format!(
            "unsupported ACME directory preset: {other}"
        ))),
    }
}

fn normalize_record_name(name: &str, origin: &str) -> String {
    let trimmed = name.trim_end_matches('.').to_ascii_lowercase();
    let origin = origin.trim_end_matches('.').to_ascii_lowercase();
    if trimmed == "@" || trimmed.is_empty() {
        origin
    } else if trimmed == origin || trimmed.ends_with(&format!(".{origin}")) {
        trimmed
    } else {
        format!("{trimmed}.{origin}")
    }
}

fn acme_io_error(error: edgerun_acme::AcmeError) -> io::Error {
    match error {
        edgerun_acme::AcmeError::Server(status, Some(detail)) => io::Error::new(
            io::ErrorKind::Other,
            format!(
                "ACME server error {status}: {}: {}",
                detail.error_type, detail.detail
            ),
        ),
        other => io::Error::new(io::ErrorKind::Other, other.to_string()),
    }
}

fn build_smtp_servers(
    spec: &SmtpServerSpec,
    imap_specs: &[ImapServerSpec],
) -> io::Result<Vec<SmtpServer>> {
    let tls_cert = load_tls_from_spec(spec.tls_cert.as_deref(), spec.tls_key.as_deref())?;
    let maildir_root = required_path(spec.maildir_root.as_deref(), "SmtpServer.maildir_root")?;
    secure_private_dir(&maildir_root)?;
    let queue_data_root = if spec.relay_enabled {
        spec.queue_dir.as_ref().map(PathBuf::from)
    } else {
        None
    };
    if let Some(queue_dir) = queue_data_root.as_deref() {
        secure_private_dir(queue_dir)?;
    }
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
        queue_data_root,
        relay_dns_server: spec
            .dns_server
            .clone()
            .unwrap_or_else(|| "127.0.0.1:53".to_string()),
        auth_mechanisms: Vec::new(),
        require_auth: false,
        #[cfg(feature = "tls")]
        tls_cert: tls_cert.clone(),
        #[cfg(feature = "dkim")]
        dkim_signer: load_dkim_signer(spec)?,
        ..Default::default()
    };

    let handler = Arc::new(MaildirStore::new(&maildir_root)?);
    let auth_enabled = register_smtp_users(&handler, spec, imap_specs)?;
    if auth_enabled {
        config.auth_mechanisms = vec!["PLAIN".to_string(), "LOGIN".to_string()];
    }
    let mut servers = vec![SmtpServer::new(config.clone(), handler.clone())?];

    if spec.smtps {
        config.bind_addr = implicit_tls_addr(&config.bind_addr, 465);
        config.smtps = true;
        config.starttls = false;
        config.require_auth = auth_enabled;
        servers.push(SmtpServer::new(config.clone(), handler.clone())?);
    }
    if spec.starttls && auth_enabled && tls_cert.is_some() {
        config.bind_addr =
            implicit_tls_addr(spec.bind_address.as_deref().unwrap_or("0.0.0.0:25"), 587);
        config.smtps = false;
        config.starttls = true;
        config.require_auth = true;
        servers.push(SmtpServer::new(config, handler)?);
    }
    Ok(servers)
}

fn build_imap_servers(spec: &ImapServerSpec) -> io::Result<Vec<ImapServer>> {
    let tls_cert = load_tls_from_spec(spec.tls_cert.as_deref(), spec.tls_key.as_deref())?;
    let maildir_root = required_path(spec.maildir_root.as_deref(), "ImapServer.maildir_root")?;
    secure_private_dir(&maildir_root)?;
    let store = Arc::new(MaildirImapStore::new(&maildir_root)?);
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

fn register_smtp_users(
    store: &MaildirStore,
    spec: &SmtpServerSpec,
    imap_specs: &[ImapServerSpec],
) -> io::Result<bool> {
    let users = configured_users(spec.users.as_deref(), &spec.local_domains);
    let mut auth_enabled = false;
    for user in users {
        let domains = user
            .domains
            .as_deref()
            .unwrap_or(spec.local_domains.as_slice());
        let domain_refs: Vec<&str> = domains.iter().map(String::as_str).collect();
        store.add_user(&user.username, &domain_refs)?;
        if let Some(password) = smtp_user_password(&user, imap_specs) {
            store.set_user_password(&user.username, &password)?;
            auth_enabled = true;
        }
    }
    if let Some(username) = spec.catch_all_user.as_deref() {
        store.set_catch_all_user(username)?;
    }
    Ok(auth_enabled)
}

fn smtp_user_password(user: &MailUserSpec, imap_specs: &[ImapServerSpec]) -> Option<String> {
    if let Some(password) = user.password.clone() {
        return Some(password);
    }
    imap_specs
        .iter()
        .filter_map(|spec| spec.users.as_deref())
        .flatten()
        .find(|imap_user| imap_user.username == user.username)
        .and_then(|imap_user| imap_user.password.clone())
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

fn secure_private_dir(path: &Path) -> io::Result<()> {
    std::fs::create_dir_all(path)?;
    secure_private_dir_mode(path)?;
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry.file_type()?.is_dir() {
            secure_private_dir_tree(&entry_path)?;
        }
    }
    Ok(())
}

fn secure_private_dir_tree(path: &Path) -> io::Result<()> {
    secure_private_dir_mode(path)?;
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry.file_type()?.is_dir() {
            secure_private_dir_tree(&entry_path)?;
        }
    }
    Ok(())
}

fn secure_private_dir_mode(path: &Path) -> io::Result<()> {
    let mut permissions = std::fs::metadata(path)?.permissions();
    let mode = permissions.mode() & 0o777;
    if mode & 0o077 != 0 {
        permissions.set_mode(mode & !0o077);
        std::fs::set_permissions(path, permissions)?;
    }
    Ok(())
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
    } else if trimmed.contains('.') {
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
    let out_dir = PathBuf::from(arg_value(args, "--out-dir").unwrap_or("/etc/edgerun/server"));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_target_keeps_dotted_parent_names_absolute() {
        assert_eq!(
            normalize_target("ns1.edgerun.tech", "nodes.edgerun.tech"),
            "ns1.edgerun.tech"
        );
        assert_eq!(
            normalize_target("mail.edgerun.tech.", "nodes.edgerun.tech"),
            "mail.edgerun.tech"
        );
    }

    #[test]
    fn normalize_target_expands_relative_labels() {
        assert_eq!(normalize_target("@", "edgerun.tech"), "edgerun.tech");
        assert_eq!(
            normalize_target("mail", "edgerun.tech"),
            "mail.edgerun.tech"
        );
    }
}
