//! Host Edgerun server binary.
//!
//! Loads Kubernetes-style Edgerun YAML via `edgerun-config`, serves
//! authoritative DNS zones with `edgerun-dns`, accepts SMTP, relays outbound
//! mail through the built-in queue, and exposes IMAP over the same Maildir.

#[path = "edgerun-server/webmail.rs"]
mod webmail;

use webmail::{
    build_webmail_config, http_date_now, method_not_allowed, normalize_crlf, sanitize_header,
    unique_webmail_id, WebmailHandler,
};

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

use edgerun_acme::{AccountKey, AcmeClient, AcmeConfig, HttpChallengeServer};
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
use edgerun_git::{GitConfig, GitHandler};
use edgerun_http::{Handler, HttpServer, Request, Response, StatusCode};
use edgerun_machine_report::{gather_machine_report, render_machine_report, OutputFormat};
use edgerun_rt::CancellationToken;
use edgerun_tls::CertificateAndKey;
use edgerun_web_ui::{FooterLink, PageShell, WorkspaceModule};

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
        if let Err(error) = run(
            resources,
            options.blog,
            options.git,
            options.dash_host,
            options.webmail,
        )
        .await
        {
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
         usage: {program} --config /etc/edgerun/server/server.yaml --blog-host blog.edgerun.tech --blog-root /srv/edgerun_core [--blog-content-dir docs/blog] [--blog-static-root /srv/blog/.generated]\n\
         usage: {program} --config /etc/edgerun/server/server.yaml --git-host git.edgerun.tech --git-root /srv/git [--dash-host dash.edgerun.tech]\n\
         options: --webmail-http-bind 0.0.0.0:80 --webmail-https-bind 0.0.0.0:443\n\
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
    git: Option<GitMount>,
    dash_host: Option<String>,
    webmail: WebmailBind,
}

#[derive(Clone)]
struct WebmailBind {
    http: String,
    https: String,
}

#[derive(Clone)]
struct BlogMount {
    host: String,
    root: PathBuf,
    content_dir: PathBuf,
    static_root: Option<PathBuf>,
    title: String,
    description: String,
    base_url: String,
}

#[derive(Clone)]
struct GitMount {
    host: String,
    root: PathBuf,
    title: String,
    description: String,
    base_url: String,
}

fn parse_server_options(args: &[String]) -> Result<ServerOptions, String> {
    let mut config = None;
    let mut blog_host = None;
    let mut blog_root = None;
    let mut blog_content_dir = PathBuf::from(".");
    let mut blog_static_root = None;
    let mut blog_title = "EdgeRun Build Log".to_string();
    let mut blog_description =
        "Feature-by-feature notes on building Edgerun from its source tree.".to_string();
    let mut blog_base_url = String::new();
    let mut git_host = None;
    let mut git_root = None;
    let mut git_title = "Edgerun Git".to_string();
    let mut git_description = "Code released from the Edgerun project.".to_string();
    let mut git_base_url = String::new();
    let mut dash_host = None;
    let mut webmail_http_bind = "0.0.0.0:80".to_string();
    let mut webmail_https_bind = "0.0.0.0:443".to_string();
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
            "--blog-content-dir" if i + 1 < args.len() => {
                blog_content_dir = PathBuf::from(&args[i + 1]);
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
            "--git-host" if i + 1 < args.len() => {
                git_host = Some(args[i + 1].clone());
                i += 1;
            }
            "--git-root" if i + 1 < args.len() => {
                git_root = Some(PathBuf::from(&args[i + 1]));
                i += 1;
            }
            "--git-title" if i + 1 < args.len() => {
                git_title = args[i + 1].clone();
                i += 1;
            }
            "--git-description" if i + 1 < args.len() => {
                git_description = args[i + 1].clone();
                i += 1;
            }
            "--git-base-url" if i + 1 < args.len() => {
                git_base_url = args[i + 1].trim_end_matches('/').to_string();
                i += 1;
            }
            "--dash-host" if i + 1 < args.len() => {
                dash_host = Some(args[i + 1].clone());
                i += 1;
            }
            "--webmail-http-bind" if i + 1 < args.len() => {
                webmail_http_bind = args[i + 1].clone();
                i += 1;
            }
            "--webmail-https-bind" if i + 1 < args.len() => {
                webmail_https_bind = args[i + 1].clone();
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
            content_dir: blog_content_dir,
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
    let git = match (git_host, git_root) {
        (Some(host), Some(root)) => Some(GitMount {
            host,
            root,
            title: git_title,
            description: git_description,
            base_url: git_base_url,
        }),
        (None, None) => None,
        _ => {
            return Err(
                "--git-host and --git-root must be provided together when enabling the git explorer"
                    .to_string(),
            );
        }
    };
    Ok(ServerOptions {
        config_path,
        blog,
        git,
        dash_host,
        webmail: WebmailBind {
            http: webmail_http_bind,
            https: webmail_https_bind,
        },
    })
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

async fn run(
    resources: Vec<ConfigResource>,
    blog: Option<BlogMount>,
    git: Option<GitMount>,
    dash_host: Option<String>,
    webmail_bind: WebmailBind,
) -> io::Result<()> {
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
            let dns =
                match acme_challenge_method(spec)? {
                    AcmeChallengeMethod::Dns01 => Some(dns_server.as_ref().ok_or_else(|| {
                        invalid_config("ACME DNS-01 requires at least one DnsZone")
                    })?),
                    AcmeChallengeMethod::Http01 => None,
                };
            ensure_acme_certificate(spec, dns, &zones).await?;
        }
    }

    if let Some(webmail) = build_webmail_config(&smtp_specs, &imap_specs)? {
        let tls = load_tls_from_spec(webmail.tls_cert.as_deref(), webmail.tls_key.as_deref())?;
        let web_handler = WebmailHandler::new(webmail.clone());
        let site_handler = SiteRouter::new(web_handler, blog, git, dash_host);
        if tls.is_some() {
            let http = HttpServer::new(HttpsRedirectHandler::new(webmail.hostname.clone()))
                .bind(webmail_bind.http.clone())
                .await
                .map_err(to_io_error)?;
            let token = shutdown.clone();
            tasks.push(edgerun_rt::spawn(async move {
                http.serve_with_shutdown(token).await.map_err(to_io_error)
            }));
        } else {
            let http = HttpServer::new(site_handler.clone())
                .bind(webmail_bind.http.clone())
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
                .bind(webmail_bind.https.clone())
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
struct SiteRouter {
    webmail: WebmailHandler,
    dash_host: Option<String>,
    blog_host: Option<String>,
    blog: Option<BlogHandler>,
    git_host: Option<String>,
    git: Option<GitHandler>,
}

impl SiteRouter {
    fn new(
        webmail: WebmailHandler,
        blog: Option<BlogMount>,
        git: Option<GitMount>,
        dash_host: Option<String>,
    ) -> Self {
        let (blog_host, blog) = match blog {
            Some(blog) => {
                let base_url = if blog.base_url.is_empty() {
                    format!("https://{}", blog.host)
                } else {
                    blog.base_url
                };
                let config = BlogConfig {
                    root: blog.root,
                    content_dir: blog.content_dir,
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
        let (git_host, git) = match git {
            Some(git) => {
                let base_url = if git.base_url.is_empty() {
                    format!("https://{}", git.host)
                } else {
                    git.base_url
                };
                let config = GitConfig {
                    root: git.root,
                    bind_addr: String::new(),
                    title: git.title,
                    description: git.description,
                    base_url,
                };
                (
                    Some(normalize_host(&git.host)),
                    Some(GitHandler::new(config)),
                )
            }
            None => (None, None),
        };
        Self {
            webmail,
            dash_host: dash_host.map(|host| normalize_host(&host)),
            blog_host,
            blog,
            git_host,
            git,
        }
    }

    fn handle_sync(&self, request: Request) -> Response {
        let host = request_host(&request);
        if self.dash_host.as_deref() == host.as_deref() {
            let target = request.uri().request_target();
            let path = target.split('?').next().unwrap_or(target.as_str());
            if path.starts_with("/surface/mail") {
                return self.webmail.handle_dash_mail(request);
            }
            return match request.method().as_str() {
                "GET" | "HEAD" => dash_response(&request),
                _ => method_not_allowed("GET, HEAD"),
            };
        }
        if self.blog_host.as_deref() == host.as_deref() {
            return match request.method().as_str() {
                "GET" | "HEAD" => redirect_to_dash_surface("build-log"),
                _ => method_not_allowed("GET, HEAD"),
            };
        }
        if self.git_host.as_deref() == host.as_deref() {
            return match request.method().as_str() {
                "GET" | "HEAD" => redirect_to_dash_surface("code"),
                _ => method_not_allowed("GET, HEAD"),
            };
        }
        if host.as_deref() == Some(normalize_host(self.webmail.hostname()).as_str()) {
            let target = request.uri().request_target();
            let path = target.split('?').next().unwrap_or(target.as_str());
            if path == "/" || path == "/index.html" {
                return match request.method().as_str() {
                    "GET" | "HEAD" => redirect_to_dash_surface("mail"),
                    _ => method_not_allowed("GET, HEAD"),
                };
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

fn dash_response(request: &Request) -> Response {
    let target = request.uri().request_target();
    let path = target.split('?').next().unwrap_or(target.as_str());
    let body = match path {
        "/surface/blog" => render_dash_blog_surface(),
        "/surface/git" => render_dash_code_surface(),
        _ => render_dash_html(),
    };
    Response::html(StatusCode::OK, &body)
        .with_header("Cache-Control", "no-store")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_header("Referrer-Policy", "strict-origin-when-cross-origin")
        .with_header(
            "Permissions-Policy",
            "camera=(), microphone=(), geolocation=()",
        )
}

fn redirect_to_dash_surface(surface: &str) -> Response {
    Response::text(StatusCode::new(308).unwrap(), "")
        .with_header("Location", &format!("https://dash.edgerun.tech/#{surface}"))
        .with_header("Cache-Control", "no-store")
        .with_header("X-Content-Type-Options", "nosniff")
}

fn render_dash_surface(label: &str, subtitle: &str, content: &str) -> String {
    format!(
        "<section id=\"surfaceSlot\" class=\"dash-stage\" aria-label=\"Workspace surface\"><header><div><strong id=\"surfaceTitle\">{}</strong><span id=\"surfaceUrl\">backend: {}</span></div></header><div class=\"dash-surface\">{}</div></section>",
        edgerun_web_ui::escape_html(label),
        edgerun_web_ui::escape_html(subtitle),
        content
    )
}

fn render_dash_blog_surface() -> String {
    render_dash_surface(
        "Build Log",
        "blog.edgerun.tech",
        "<div class=\"dash-grid\"><a class=\"dash-card\" href=\"https://dash.edgerun.tech/#build-log\"><strong>Latest posts</strong><span>Follow feature-by-feature work as it lands.</span></a><a class=\"dash-card\" href=\"https://dash.edgerun.tech/#about\"><strong>About Edgerun</strong><span>The philosophy and direction behind the project.</span></a><a class=\"dash-card\" href=\"https://dash.edgerun.tech/#feed\"><strong>Feed</strong><span>Subscribe to release notes and build notes.</span></a></div>",
    )
}

fn render_dash_code_surface() -> String {
    render_dash_surface(
        "Code",
        "git.edgerun.tech",
        "<div class=\"dash-grid\"><a class=\"dash-card\" href=\"https://dash.edgerun.tech/#code\"><strong>Repositories</strong><span>Browse released source surfaces.</span></a><a class=\"dash-card\" href=\"https://dash.edgerun.tech/#crates\"><strong>Crate explorer</strong><span>Navigate visible crates, metadata, APIs, and relationships.</span></a><a class=\"dash-card\" href=\"https://dash.edgerun.tech/#source\"><strong>Source tree</strong><span>Open the public source tree directly.</span></a></div>",
    )
}

fn render_dash_html() -> String {
    let header_center = edgerun_web_ui::render_header_search_input(
        "workspaceSearch",
        "Search workspace",
        "Search workspace",
    );
    let header_actions = edgerun_web_ui::render_workspace_actions("dash", "");
    let local_links = [FooterLink {
        href: "mailto:ken@edgerun.tech",
        label: "Contact",
    }];
    let footer = edgerun_web_ui::render_common_footer("dash", &local_links, "");
    let style = format!("{}{}", edgerun_web_ui::BASE_STYLE, DASH_STYLE);
    let body = format!(
        "{}<script>{}{}{}</script>",
        DASH_BODY,
        edgerun_web_ui::THEME_TOGGLE_JS,
        edgerun_web_ui::WORKSPACE_JS,
        DASH_JS
    );
    let modules = [
        WorkspaceModule {
            surface: "mail",
            selector: "er-mail-surface",
            wasm: "/modules/mail.wasm",
        },
        WorkspaceModule {
            surface: "git",
            selector: "er-git-surface",
            wasm: "/modules/git.wasm",
        },
        WorkspaceModule {
            surface: "blog",
            selector: "er-blog-surface",
            wasm: "/modules/blog.wasm",
        },
    ];
    edgerun_web_ui::render_page(&PageShell {
        lang: "en",
        title: "Edgerun Dash",
        description: "Persistent Edgerun workspace.",
        theme_color: "#146c63",
        generator: "edgerun-server",
        extra_head: "<link rel=\"icon\" href='data:image/svg+xml,<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 100 100\"><text y=\"76\" font-size=\"76\">⌘</text></svg>'>",
        style: &style,
        brand_href: "/",
        brand_label: "Edgerun Dash home",
        brand_text: "Edgerun Dash",
        header_center: &header_center,
        header_actions: &header_actions,
        footer: &footer,
        body: &body,
        script_src: None,
        workspace_modules: &modules,
    })
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

const DASH_BODY: &str = r##"
<main id="content" class="dash">
  <nav class="dash-rail" aria-label="Workspace surfaces">
    <button class="dash-tab" type="button" hx-get="/surface/blog" hx-target="#surfaceSlot" hx-swap="outerHTML" aria-current="page">Build Log</button>
    <button class="dash-tab" type="button" hx-get="/surface/git" hx-target="#surfaceSlot" hx-swap="outerHTML">Code</button>
    <button class="dash-tab" type="button" hx-get="/surface/mail" hx-target="#surfaceSlot" hx-swap="outerHTML">Mail</button>
  </nav>
  <section id="surfaceSlot" class="dash-stage" aria-label="Workspace surface">
    <header><div><strong id="surfaceTitle">Build Log</strong><span id="surfaceUrl">backend: blog.edgerun.tech</span></div></header>
    <div class="dash-surface"><div class="dash-grid"><a class="dash-card" href="https://dash.edgerun.tech/#build-log"><strong>Latest posts</strong><span>Follow feature-by-feature work as it lands.</span></a><a class="dash-card" href="https://dash.edgerun.tech/#about"><strong>About Edgerun</strong><span>The philosophy and direction behind the project.</span></a><a class="dash-card" href="https://dash.edgerun.tech/#feed"><strong>Feed</strong><span>Subscribe to release notes and build notes.</span></a></div></div>
  </section>
</main>
"##;

const DASH_STYLE: &str = r#"
.dash{height:calc(100vh - var(--topbar-h) - var(--footer-h));min-height:0;display:grid;grid-template-columns:220px minmax(0,1fr);background:var(--bg)}
.dash-rail{border-right:1px solid var(--line);padding:18px;display:flex;flex-direction:column;gap:8px;background:var(--panel)}
.dash-tab{height:42px;border:1px solid transparent;border-radius:8px;background:transparent;color:var(--muted);text-align:left;padding:0 12px;cursor:pointer;font-weight:750}
.dash-tab:hover,.dash-tab[aria-current=page]{border-color:var(--line);background:var(--bg);color:var(--accent)}
.dash-stage{min-width:0;display:grid;grid-template-rows:56px 1fr}
.dash-stage header{display:flex;align-items:center;justify-content:space-between;gap:14px;padding:0 18px;border-bottom:1px solid var(--line);background:var(--panel)}
.dash-stage header div{display:grid;line-height:1.2}.dash-stage header span{color:var(--muted);font-size:12px}.dash-stage header a{color:var(--muted);text-decoration:none}.dash-stage header a:hover{color:var(--accent)}
	.dash-surface{overflow:auto;padding:24px}.dash-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(220px,1fr));gap:14px;max-width:980px}.dash-card{min-height:140px;display:flex;flex-direction:column;justify-content:space-between;gap:18px;border:1px solid var(--line);border-radius:8px;background:var(--panel);color:var(--text);padding:18px;text-decoration:none}.dash-card:hover{border-color:var(--accent)}.dash-card span{color:var(--muted)}.dash-card-button{text-align:left;font:inherit;cursor:pointer}
	.dash-mail{display:grid;grid-template-columns:minmax(260px,360px) minmax(0,1fr);gap:14px;min-height:520px}.dash-mail-toolbar{display:flex;align-items:center;justify-content:space-between;gap:12px;margin:0 0 14px}.dash-mail-button{height:40px;border:1px solid var(--line);border-radius:8px;background:var(--panel);color:var(--text);display:inline-flex;align-items:center;gap:8px;padding:0 12px;text-decoration:none;cursor:pointer}.dash-mail-button:hover{border-color:var(--accent);color:var(--accent)}.dash-mail-list,.dash-mail-detail,.dash-mail-login{border:1px solid var(--line);border-radius:8px;background:var(--panel)}.dash-mail-list{overflow:auto}.dash-mail-item{display:grid;gap:4px;padding:13px 14px;border-bottom:1px solid var(--line);text-decoration:none}.dash-mail-item:hover{background:color-mix(in srgb,var(--accent) 8%,transparent)}.dash-mail-item[aria-current=true]{border-left:3px solid var(--accent);padding-left:11px}.dash-mail-item strong{line-height:1.25}.dash-mail-item span,.dash-mail-meta,.dash-mail-preview,.dash-mail-empty,.dash-mail-toolbar span{color:var(--muted)}.dash-mail-meta{display:flex;gap:8px;flex-wrap:wrap;font-size:13px}.dash-mail-unread{color:var(--accent);font-weight:800}.dash-mail-detail{min-width:0;padding:18px;overflow:auto}.dash-mail-detail header{display:block;padding:0 0 14px;border:0;background:transparent}.dash-mail-detail h2{margin:0 0 8px;font-size:26px;line-height:1.15}.dash-mail-body{white-space:pre-wrap;overflow-wrap:anywhere;color:var(--text)}.dash-mail-warning{margin:12px 0;padding:10px 12px;border:1px solid var(--accent-2);border-radius:8px;color:var(--accent-2)}.dash-mail-login{max-width:460px;padding:18px;display:grid;gap:12px}.dash-mail-login label,.dash-mail-compose label{display:grid;gap:6px;color:var(--muted);font-weight:750}.dash-mail-login input,.dash-mail-compose input,.dash-mail-compose textarea{width:100%;border:1px solid var(--line);border-radius:8px;background:var(--bg);color:var(--text);padding:10px 12px;font:inherit}.dash-mail-compose{display:grid;gap:12px}.dash-mail-compose textarea{min-height:220px;resize:vertical}.dash-mail-actions{display:flex;gap:10px;align-items:center;flex-wrap:wrap}
	@media(max-width:760px){.dash{grid-template-columns:1fr;grid-template-rows:auto 1fr}.dash-rail{border-right:0;border-bottom:1px solid var(--line);flex-direction:row;overflow:auto}.dash-tab{white-space:nowrap}.dash-stage header{padding:0 12px}}
	@media(max-width:900px){.dash-mail{grid-template-columns:1fr}.dash-mail-list{max-height:320px}}
	"#;

const DASH_JS: &str = r#"
const search=document.getElementById('workspaceSearch');
const dashSurfaceMap={'#build-log':'/surface/blog','#blog':'/surface/blog','#code':'/surface/git','#git':'/surface/git','#mail':'/surface/mail','#crates':'/surface/git','#source':'/surface/git','#about':'/surface/blog','#feed':'/surface/blog'};
async function loadDashHash(){const path=dashSurfaceMap[location.hash];if(!path)return;const slot=document.querySelector('#surfaceSlot');if(!slot)return;const response=await fetch(path,{headers:workspaceHeaders(path)});if(!response.ok)return;slot.outerHTML=await response.text();document.querySelectorAll('.dash-tab[aria-current]').forEach(node=>node.removeAttribute('aria-current'));const tab=document.querySelector('[hx-get="'+path+'"]');if(tab)tab.setAttribute('aria-current','page')}
addEventListener('hashchange',loadDashHash);loadDashHash();
search&&search.addEventListener('keydown',event=>{if(event.key!=='Enter')return;event.preventDefault();const q=search.value.trim();if(!q)return;location.href='https://dash.edgerun.tech/#code?q='+encodeURIComponent(q)});
"#;

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

const DNSSEC_RESIGN_INTERVAL_SECS: u64 = 12 * 60 * 60;
const DNSSEC_RESIGN_POLL_SECS: u64 = 60;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AcmeChallengeMethod {
    Http01,
    Dns01,
}

fn acme_challenge_method(spec: &SmtpServerSpec) -> io::Result<AcmeChallengeMethod> {
    let value = spec
        .acme_challenge
        .as_deref()
        .unwrap_or("dns-01")
        .to_ascii_lowercase();
    match value.as_str() {
        "http" | "http-01" => Ok(AcmeChallengeMethod::Http01),
        "dns" | "dns-01" => Ok(AcmeChallengeMethod::Dns01),
        other => Err(invalid_config(format!(
            "unsupported ACME challenge type: {other}"
        ))),
    }
}

fn acme_challenge_name(method: AcmeChallengeMethod) -> &'static str {
    match method {
        AcmeChallengeMethod::Http01 => "http-01",
        AcmeChallengeMethod::Dns01 => "dns-01",
    }
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
    dns: Option<&Arc<DnsServer>>,
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
    let challenge_method = acme_challenge_method(spec)?;
    let http_challenges = if challenge_method == AcmeChallengeMethod::Http01 {
        let handler = HttpChallengeServer::new(client.thumbprint());
        let server = HttpServer::new(handler.clone())
            .bind("0.0.0.0:80")
            .await
            .map_err(to_io_error)?;
        let shutdown = CancellationToken::new();
        let token = shutdown.clone();
        edgerun_rt::spawn(
            async move { server.serve_with_shutdown(token).await.map_err(to_io_error) },
        );
        Some((handler, shutdown))
    } else {
        None
    };

    let mut challenge_records = Vec::new();
    for auth_url in order.authorization_urls() {
        let authorization = client
            .get_authorization(auth_url)
            .await
            .map_err(acme_io_error)?;
        if authorization.status == edgerun_acme::types::AuthorizationStatus::Valid {
            continue;
        }
        let challenge_type = match challenge_method {
            AcmeChallengeMethod::Http01 => ChallengeType::Http01,
            AcmeChallengeMethod::Dns01 => ChallengeType::Dns01,
        };
        let challenge = authorization
            .challenges
            .as_deref()
            .and_then(|challenges| {
                challenges
                    .iter()
                    .find(|challenge| challenge.challenge_type == challenge_type)
            })
            .ok_or_else(|| {
                invalid_config(format!(
                    "ACME authorization has no {} challenge",
                    acme_challenge_name(challenge_method)
                ))
            })?;
        let token = challenge.token.as_deref().ok_or_else(|| {
            invalid_config(format!(
                "ACME {} challenge missing token",
                acme_challenge_name(challenge_method)
            ))
        })?;
        match challenge_method {
            AcmeChallengeMethod::Http01 => {
                let Some((handler, _shutdown)) = &http_challenges else {
                    return Err(invalid_config("ACME HTTP-01 challenge server not running"));
                };
                handler.add_challenge(token);
            }
            AcmeChallengeMethod::Dns01 => {
                let Some(dns) = dns else {
                    return Err(invalid_config("ACME DNS-01 requires at least one DnsZone"));
                };
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
            }
        }
        client
            .validate_challenge(&challenge.url)
            .await
            .map_err(acme_io_error)?;
        wait_for_challenge(&client, &challenge.url).await?;
        if let Some((handler, _shutdown)) = &http_challenges {
            handler.remove_challenge(token);
        }
    }
    if let Some((handler, shutdown)) = &http_challenges {
        handler.clear_all();
        shutdown.cancel();
        edgerun_rt::sleep(Duration::from_millis(100)).await;
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
