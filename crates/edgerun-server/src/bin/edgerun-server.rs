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

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fs::OpenOptions;
use std::future::Future;
use std::io;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, TcpStream, UdpSocket};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant as StdInstant, SystemTime, UNIX_EPOCH};

use edgerun_acme::{AccountKey, AcmeClient, AcmeConfig, HttpChallengeServer};
use edgerun_acme::{ChallengeStatus, ChallengeType, DirectoryUrl, OrderStatus};
use edgerun_analytics::{AnalyticsConfig, AnalyticsHandler};
use edgerun_blog::{BlogConfig, BlogHandler};
use edgerun_config::edgerun_json::JsonValue;
use edgerun_config::{
    BrowserAppSpec, ConfigResource, DnsServerSpec, DnsZoneSpec, DnssecConfig, ImapServerSpec,
    MailUserSpec, SmtpServerSpec, ZoneRecord,
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
use edgerun_web_ui::{PageShell, WorkspaceModule};

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
                let (dns, zones, smtp, imap, browser_apps) = count_resources(&resources);
                println!(
                    "dns_servers={dns} dns_zones={zones} smtp_servers={smtp} imap_servers={imap} browser_apps={browser_apps}"
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
            options.analytics_log_dir,
            options.dash_modules_root,
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
         options: --webmail-http-bind 0.0.0.0:80 --webmail-https-bind 0.0.0.0:443 --analytics-log-dir /var/lib/edgerun/analytics --dash-modules-root /srv/dash/modules\n\
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

fn count_resources(resources: &[ConfigResource]) -> (usize, usize, usize, usize, usize) {
    let mut dns = 0;
    let mut zones = 0;
    let mut smtp = 0;
    let mut imap = 0;
    let mut browser_apps = 0;
    for resource in resources {
        match resource {
            ConfigResource::DnsServer(_) => dns += 1,
            ConfigResource::DnsZone(_) => zones += 1,
            ConfigResource::SmtpServer(_) => smtp += 1,
            ConfigResource::ImapServer(_) => imap += 1,
            ConfigResource::BrowserApp(_) => browser_apps += 1,
            _ => {}
        }
    }
    (dns, zones, smtp, imap, browser_apps)
}

#[derive(Clone)]
struct ServerOptions {
    config_path: PathBuf,
    blog: Option<BlogMount>,
    git: Option<GitMount>,
    dash_host: Option<String>,
    webmail: WebmailBind,
    analytics_log_dir: Option<PathBuf>,
    dash_modules_root: PathBuf,
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
    let mut analytics_log_dir = None;
    let mut dash_modules_root = PathBuf::from("/srv/dash/modules");
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
            "--analytics-log-dir" if i + 1 < args.len() => {
                analytics_log_dir = Some(PathBuf::from(&args[i + 1]));
                i += 1;
            }
            "--dash-modules-root" if i + 1 < args.len() => {
                dash_modules_root = PathBuf::from(&args[i + 1]);
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
        analytics_log_dir,
        dash_modules_root,
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
    analytics_log_dir: Option<PathBuf>,
    dash_modules_root: PathBuf,
) -> io::Result<()> {
    let mut dns_servers = Vec::new();
    let mut zones = Vec::new();
    let mut smtp_specs = Vec::new();
    let mut imap_specs = Vec::new();
    let mut browser_apps = Vec::new();

    for resource in resources {
        match resource {
            ConfigResource::DnsServer(spec) => dns_servers.push(spec),
            ConfigResource::DnsZone(spec) => zones.push(spec),
            ConfigResource::SmtpServer(spec) => smtp_specs.push(spec),
            ConfigResource::ImapServer(spec) => imap_specs.push(spec),
            ConfigResource::BrowserApp(spec) => browser_apps.push(spec),
            _ => {}
        }
    }
    eprintln!(
        "edgerun-server: config dns_servers={} dns_zones={} smtp_servers={} imap_servers={} browser_apps={}",
        dns_servers.len(),
        zones.len(),
        smtp_specs.len(),
        imap_specs.len(),
        browser_apps.len()
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
                    ChallengeType::Dns01 => Some(dns_server.as_ref().ok_or_else(|| {
                        invalid_config("ACME DNS-01 requires at least one DnsZone")
                    })?),
                    ChallengeType::Http01 => None,
                    other => {
                        return Err(invalid_config(format!(
                            "unsupported ACME challenge type: {}",
                            acme_challenge_name(&other)
                        )))
                    }
                };
            ensure_acme_certificate(spec, dns, &zones).await?;
        }
    }

    if let Some(webmail) = build_webmail_config(&smtp_specs, &imap_specs)? {
        let tls = load_tls_from_spec(webmail.tls_cert.as_deref(), webmail.tls_key.as_deref())?;
        let web_handler = WebmailHandler::new(webmail.clone());
        let site_router = SiteRouter::new(
            web_handler,
            blog,
            git,
            dash_host,
            browser_apps,
            dash_modules_root,
        );
        let site_handler: Arc<dyn Handler> = if let Some(log_dir) = analytics_log_dir {
            eprintln!(
                "edgerun-server: analytics enabled log_dir={}",
                log_dir.display()
            );
            Arc::new(AnalyticsHandler::new(
                site_router,
                AnalyticsConfig::new(log_dir),
            ))
        } else {
            Arc::new(site_router)
        };
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
            let http = HttpServer::new(Arc::clone(&site_handler))
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
    browser_apps: Vec<BrowserAppSpec>,
    dash_modules_root: PathBuf,
    host_stats: Arc<Mutex<HostStatsCache>>,
    chat_state: Arc<Mutex<DashChatState>>,
    request_count: Arc<AtomicU64>,
}

impl SiteRouter {
    fn new(
        webmail: WebmailHandler,
        blog: Option<BlogMount>,
        git: Option<GitMount>,
        dash_host: Option<String>,
        browser_apps: Vec<BrowserAppSpec>,
        dash_modules_root: PathBuf,
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
            browser_apps,
            dash_modules_root,
            host_stats: Arc::new(Mutex::new(HostStatsCache::default())),
            chat_state: Arc::new(Mutex::new(DashChatState::default())),
            request_count: Arc::new(AtomicU64::new(0)),
        }
    }

    fn handle_sync(&self, request: Request) -> Response {
        self.request_count.fetch_add(1, Ordering::Relaxed);
        let host = request_host(&request);
        if self.dash_host.as_deref() == host.as_deref() {
            let target = request.uri().request_target();
            let path = target.split('?').next().unwrap_or(target.as_str());
            if path.starts_with("/modules/") {
                return match request.method().as_str() {
                    "GET" | "HEAD" => self.serve_dash_module(path),
                    _ => method_not_allowed("GET, HEAD"),
                };
            }
            if path == "/status.json" {
                return match request.method().as_str() {
                    "GET" | "HEAD" => self.dash_status_response(),
                    _ => method_not_allowed("GET, HEAD"),
                };
            }
            if path.starts_with("/api/chat") {
                return self.handle_dash_chat(request);
            }
            if path.starts_with("/surface/mail") {
                return self.webmail.handle_dash_mail(request);
            }
            return match request.method().as_str() {
                "GET" | "HEAD" => self.dash_response(&request),
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

    fn handle_dash_chat(&self, request: Request) -> Response {
        match request.method().as_str() {
            "GET" | "HEAD" => self.dash_chat_list(),
            "POST" => self.dash_chat_post(&request),
            _ => method_not_allowed("GET, HEAD, POST"),
        }
    }

    fn dash_chat_list(&self) -> Response {
        let Ok(state) = self.chat_state.lock() else {
            return Response::json(
                StatusCode::new(500).unwrap(),
                r#"{"ok":false,"error":"chat state unavailable"}"#,
            )
            .with_header("Cache-Control", "no-store")
            .with_header("X-Content-Type-Options", "nosniff");
        };
        let mut messages = String::new();
        for (index, message) in state.messages.iter().enumerate() {
            if index > 0 {
                messages.push(',');
            }
            messages.push_str(&message.to_json());
        }
        let body = format!(r#"{{"ok":true,"messages":[{}]}}"#, messages);
        Response::json(StatusCode::OK, &body)
            .with_header("Cache-Control", "no-store")
            .with_header("X-Content-Type-Options", "nosniff")
    }

    fn dash_chat_post(&self, request: &Request) -> Response {
        let body = request.body().unwrap_or_default();
        let payload: JsonValue = match edgerun_config::edgerun_json::from_json_slice(body) {
            Ok(value) => value,
            Err(error) => {
                return dash_chat_error_response(
                    StatusCode::new(400).unwrap(),
                    &format!("invalid JSON: {error}"),
                );
            }
        };
        let raw_handle = payload
            .get("name")
            .and_then(JsonValue::as_str)
            .unwrap_or("Guest")
            .trim();
        let raw_message = payload
            .get("message")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .trim();
        let handle = sanitize_chat_input(raw_handle, 24);
        let message = sanitize_chat_input(raw_message, 800);
        let sender_key = chat_sender_key(request);
        let now = chat_unix_now();
        if let Some(reason) = apply_chat_heuristics(&handle, &message, &sender_key, now) {
            return dash_chat_error_response(
                StatusCode::new(403).unwrap(),
                &format!("message blocked: {reason}"),
            );
        }

        let mut state = match self.chat_state.lock() {
            Ok(state) => state,
            Err(error) => {
                return dash_chat_error_response(
                    StatusCode::new(500).unwrap(),
                    &format!("chat unavailable: {error}"),
                );
            }
        };

        if let Some(reason) = state.apply_heuristics(&handle, &message, &sender_key, now) {
            return dash_chat_error_response(
                StatusCode::new(403).unwrap(),
                &format!("message blocked: {reason}"),
            );
        }

        let id = state.next_id.saturating_add(1);
        state.next_id = id;
        state.messages.push_back(DashChatMessage {
            id,
            handle: handle.clone(),
            message: message.clone(),
            sender_key: sender_key.clone(),
            posted_at: now,
        });
        state.register_sender(&sender_key, now);
        state.prune(now);
        Response::json(
            StatusCode::OK,
            &format!(
                r#"{{"ok":true,"message":{{"id":{},"name":"{}","message":"{}","at":{}}}}}"#,
                id,
                edgerun_web_ui::escape_json(&handle),
                edgerun_web_ui::escape_json(&message),
                now
            ),
        )
        .with_header("Cache-Control", "no-store")
        .with_header("X-Content-Type-Options", "nosniff")
    }

    fn dash_response(&self, request: &Request) -> Response {
        let target = request.uri().request_target();
        let path = target.split('?').next().unwrap_or(target.as_str());
        let body = match path {
            "/apps/catalog.json" => {
                return Response::text(
                    StatusCode::OK,
                    &render_browser_app_catalog(&self.browser_apps),
                )
                .with_header("Content-Type", "application/json")
                .with_header("Cache-Control", "no-store")
                .with_header("X-Content-Type-Options", "nosniff");
            }
            path if path == "/surface/apps" || path.starts_with("/surface/apps/") => render_dash_surface(
                "Apps",
                "browser node",
                &render_browser_apps_surface(&self.browser_apps),
            ),
            path if path == "/surface/blog" || path.starts_with("/surface/blog/") => self
                .blog
                .as_ref()
                .and_then(|blog| blog.render_dash_content(path).ok())
                .map(|content| render_dash_surface("Build Log", "blog.edgerun.tech", &content))
                .unwrap_or_else(render_dash_blog_surface),
            path if path == "/surface/git" || path.starts_with("/surface/git/") => self
                .git
                .as_ref()
                .and_then(|git| git.render_dash_content(path).ok())
                .map(|content| render_dash_surface("Code", "git.edgerun.tech", &content))
                .unwrap_or_else(render_dash_code_surface),
            _ => render_dash_html(&self.browser_apps),
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

    fn serve_dash_module(&self, path: &str) -> Response {
        let Some(name) = path.strip_prefix("/modules/") else {
            return Response::not_found();
        };
        if name.is_empty()
            || name.contains('/')
            || name.contains('\\')
            || name.contains("..")
            || !name.ends_with(".wasm")
        {
            return Response::not_found()
                .with_header("Cache-Control", "no-store")
                .with_header("X-Content-Type-Options", "nosniff");
        }

        match std::fs::read(self.dash_modules_root.join(name)) {
            Ok(bytes) => Response::new(StatusCode::OK)
                .with_header("Content-Type", "application/wasm")
                .with_header("Cache-Control", "public, max-age=31536000, immutable")
                .with_header("X-Content-Type-Options", "nosniff")
                .with_body(bytes),
            Err(_) => Response::not_found()
                .with_header("Cache-Control", "no-store")
                .with_header("X-Content-Type-Options", "nosniff"),
        }
    }

    fn dash_status_response(&self) -> Response {
        let request_count = self.request_count.load(Ordering::Relaxed);
        let body = match self.host_stats.lock() {
            Ok(mut cache) => render_host_status_json(&mut cache, request_count),
            Err(_) => String::from("{\"ok\":false,\"error\":\"status cache unavailable\"}"),
        };
        Response::json(StatusCode::OK, &body)
            .with_header("Cache-Control", "no-store")
            .with_header("X-Content-Type-Options", "nosniff")
    }
}

impl Handler for SiteRouter {
    fn handle(&self, request: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        Box::pin(async move { self.handle_sync(request) })
    }
}

fn redirect_to_dash_surface(_surface: &str) -> Response {
    Response::text(StatusCode::new(308).unwrap(), "")
        .with_header("Location", "/")
        .with_header("Cache-Control", "no-store")
        .with_header("X-Content-Type-Options", "nosniff")
}

fn render_dash_surface(label: &str, subtitle: &str, content: &str) -> String {
    format!(
        "<section class=\"dash-stage\" aria-label=\"Workspace surface\"><div class=\"dash-surface\" data-dash-surface-root data-surface-title=\"{}\" data-surface-subtitle=\"{}\">{}</div></section>",
        edgerun_web_ui::escape_attr(label),
        edgerun_web_ui::escape_attr(subtitle),
        content
    )
}

fn render_dash_blog_surface() -> String {
    render_dash_surface(
        "Build Log",
        "blog.edgerun.tech",
        "<div class=\"dash-quick-links\"><button class=\"dash-card dash-card-button\" type=\"button\" data-surface-module=\"build-log\" data-surface=\"build-log\" hx-get=\"/surface/blog/posts/edgerun-onboarding.html\" data-search-text=\"\"><strong>Onboarding</strong><span>Start with architecture, capabilities, and workflow.</span></button><button class=\"dash-card dash-card-button\" type=\"button\" data-surface-module=\"build-log\" data-surface=\"build-log\" hx-get=\"/surface/blog/posts/build-your-own-edgerun-app.html\" data-search-text=\"\"><strong>Build your own app</strong><span>Step-by-step guide for real app onboarding.</span></button><div class=\"dash-code-tools\" aria-label=\"Blog tools\"><button class=\"dash-card dash-card-button\" type=\"button\" data-surface-module=\"build-log\" data-surface=\"build-log\" hx-get=\"/surface/blog\" data-search-text=\"\"><strong>Latest posts</strong><span>Follow feature-by-feature work as it lands.</span></button><button class=\"dash-card dash-card-button\" type=\"button\" data-surface-module=\"build-log\" data-surface=\"build-log\" hx-get=\"/surface/blog/about\" data-search-text=\"\"><strong>About Edgerun</strong><span>The philosophy and direction behind the project.</span></button><button class=\"dash-card dash-card-button\" type=\"button\" data-surface-module=\"build-log\" data-surface=\"build-log\" hx-get=\"/surface/blog/feed\" data-search-text=\"\"><strong>Feed</strong><span>Subscribe to release notes and build notes.</span></button></div></div>",
    )
}

fn render_dash_code_surface() -> String {
    render_dash_surface(
        "Code",
        "git.edgerun.tech",
        "<div class=\"dash-grid\"><button class=\"dash-card dash-card-button\" type=\"button\" data-surface-module=\"code\" data-surface=\"code\" hx-get=\"/surface/git\" data-search-text=\"\"><strong>Repositories</strong><span>Browse released source surfaces.</span></button><button class=\"dash-card dash-card-button\" type=\"button\" data-surface-module=\"code\" data-surface=\"code\" hx-get=\"/surface/git/crates\" data-search-text=\"\"><strong>Crate explorer</strong><span>Navigate visible crates, metadata, APIs, and relationships.</span></button><button class=\"dash-card dash-card-button\" type=\"button\" data-surface-module=\"code\" data-surface=\"code\" hx-get=\"/surface/git/source\" data-search-text=\"\"><strong>Source tree</strong><span>Open the public source tree directly.</span></button></div>",
    )
}

fn render_dash_html(configured_apps: &[BrowserAppSpec]) -> String {
    let header_center = "";
    let header_actions = "";
    let footer = render_dash_status_footer();
    let style = format!("{}{}", edgerun_web_ui::BASE_STYLE, DASH_STYLE);
    let body = format!(
        "{}<script>{}{}{}</script>",
        DASH_BODY,
        edgerun_web_ui::THEME_TOGGLE_JS,
        edgerun_web_ui::WORKSPACE_JS,
        DASH_COHESIVE_JS
    );
    let fallback_modules = [
        WorkspaceModule {
            app_id: "edgerun.mail",
            title: "Mail",
            surface: "mail",
            selector: "er-mail-surface",
            wasm: "/modules/mail.wasm",
        },
        WorkspaceModule {
            app_id: "edgerun.git",
            title: "Code",
            surface: "git",
            selector: "er-git-surface",
            wasm: "/modules/git.wasm",
        },
        WorkspaceModule {
            app_id: "edgerun.blog",
            title: "Build Log",
            surface: "blog",
            selector: "er-blog-surface",
            wasm: "/modules/blog.wasm",
        },
    ];
    let configured_modules = workspace_modules_from_config(configured_apps);
    let modules = if configured_modules.is_empty() {
        &fallback_modules[..]
    } else {
        &configured_modules[..]
    };
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
        workspace_modules: modules,
    })
}

fn render_dash_status_footer() -> String {
    String::from(
        "<footer class=\"site-footer dash-status-footer\" aria-label=\"Server status\"><div class=\"dash-status\" data-dash-status><span>sessions <strong data-status-sessions>--</strong></span><span>req/s <strong data-status-rps>--</strong></span><span>mem <strong data-status-memory>--</strong></span><span>cpu <strong data-status-cpu>--</strong></span><span>bin <strong data-status-binary>--</strong></span><div class=\"dash-status-actions\"><section id=\"dashChatDock\" class=\"dash-chat-dock is-open\" aria-label=\"Global chat dock\"><button id=\"dashChatBubble\" class=\"dash-chat-bubble\" type=\"button\" data-chat-action=\"open\" title=\"Open chat\" aria-label=\"Open global chat\"><span aria-hidden=\"true\">💬</span></button><section id=\"dashChatPanel\" class=\"dash-chat-panel\"><header class=\"dash-chat-panel-head\"><h2>Global chat</h2><div class=\"dash-chat-panel-controls\"><button id=\"dashChatMinimize\" class=\"dash-chat-panel-button\" type=\"button\" data-chat-action=\"minimize\" aria-label=\"Minimize chat\">▁</button><button id=\"dashChatClose\" class=\"dash-chat-panel-button\" type=\"button\" data-chat-action=\"close\" aria-label=\"Close chat\">×</button></div></header><div class=\"dash-chat-shell\"><form id=\"dashChatForm\" class=\"dash-chat-form\" autocomplete=\"off\"><label for=\"dashChatName\"><span>Name</span><input id=\"dashChatName\" required maxlength=\"24\" placeholder=\"your name\"></label><label for=\"dashChatMessage\"><span>Message</span><textarea id=\"dashChatMessage\" required maxlength=\"800\" placeholder=\"Say something to everyone\"></textarea></label><button id=\"dashChatSend\" type=\"submit\" aria-label=\"Post message\">Post</button></form><p class=\"dash-chat-status\" id=\"dashChatStatus\" role=\"status\"></p><div id=\"dashChatLog\" class=\"dash-chat-log\"></div></div></section></section><button type=\"button\" class=\"dash-status-refresh\" data-status-refresh title=\"Refresh status\" aria-label=\"Refresh status\"><span aria-hidden=\"true\">⟳</span></button><span class=\"dash-status-theme\"><er-theme-toggle></er-theme-toggle></span></div></div></footer>",
    )
}

#[derive(Default)]
struct HostStatsCache {
    previous: Option<HostStatsSample>,
    previous_request_count: Option<u64>,
    previous_request_time: Option<StdInstant>,
}

#[derive(Default)]
struct DashChatState {
    next_id: u64,
    messages: VecDeque<DashChatMessage>,
    sender_timestamps: HashMap<String, VecDeque<u64>>,
}

#[derive(Clone)]
struct DashChatMessage {
    id: u64,
    handle: String,
    message: String,
    sender_key: String,
    posted_at: u64,
}

impl DashChatState {
    fn apply_heuristics(
        &mut self,
        handle: &str,
        message: &str,
        sender_key: &str,
        now: u64,
    ) -> Option<&'static str> {
        self.prune_old_sender_history(now);
        if message.is_empty() {
            return Some("empty message");
        }
        if message.len() > 800 {
            return Some("message too long");
        }
        if handle.len() > 24 {
            return Some("name too long");
        }
        if message.to_ascii_lowercase().split_whitespace().count() > 260 {
            return Some("message too long");
        }
        if message.chars().filter(|c| c.is_ascii_uppercase()).count() > 0 {
            let letters = message
                .chars()
                .filter(|c| c.is_ascii_alphabetic())
                .count() as u32;
            if letters > 16 {
                let caps = message
                    .chars()
                    .filter(|c| c.is_ascii_uppercase())
                    .count() as u32;
                if caps > (letters * 8) / 10 {
                    return Some("too much uppercase");
                }
            }
        }
        let lower = message.to_ascii_lowercase();
        if lower.contains("http://")
            || lower.contains("https://")
            || lower.contains("www.")
            || lower.contains("bit.ly")
        {
            let links = lower.matches("http://").count()
                + lower.matches("https://").count()
                + lower.matches("www.").count()
                + lower.matches("bit.ly").count();
            if links > 2 {
                return Some("too many links");
            }
        }
        let spam_terms = [
            "free",
            "earn",
            "guaranteed",
            "casino",
            "crypto",
            "bitcoin",
            "viagra",
            "pharma",
            "loan",
            "lottery",
            "click",
            "subscribe",
            "win money",
        ];
        for term in spam_terms {
            if lower.contains(term) && lower.len() > 160 {
                return Some("spam pattern detected");
            }
        }

        if let Some(timestamps) = self.sender_timestamps.get(sender_key) {
            let recent = timestamps
                .iter()
                .take_while(|timestamp| now.saturating_sub(**timestamp) <= 45)
                .count();
            if recent >= 6 {
                return Some("too many messages from this sender");
            }
            if let Some(previous) = timestamps.back() {
                if now.saturating_sub(*previous) < 4 {
                    return Some("send a bit slower");
                }
            }
        }
        if self
            .messages
            .iter()
            .any(|item| item.sender_key == sender_key && item.message == message && now.saturating_sub(item.posted_at) <= 120)
        {
            return Some("duplicate message");
        }
        None
    }

    fn register_sender(&mut self, sender_key: &str, now: u64) {
        self.sender_timestamps
            .entry(sender_key.to_string())
            .or_default()
            .push_back(now);
    }

    fn prune_old_sender_history(&mut self, now: u64) {
        self.sender_timestamps.retain(|_, timestamps| {
            while let Some(ts) = timestamps.front() {
                if now.saturating_sub(*ts) > 120 {
                    timestamps.pop_front();
                } else {
                    break;
                }
            }
            !timestamps.is_empty()
        });
    }

    fn prune(&mut self, now: u64) {
        while self.messages.len() > 150 {
            self.messages.pop_front();
        }
        while let Some(message) = self.messages.front() {
            if now.saturating_sub(message.posted_at) > 4 * 60 * 60 {
                self.messages.pop_front();
            } else {
                break;
            }
        }
    }

    fn to_json(&self) -> String {
        let mut out = String::new();
        for (index, message) in self.messages.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push_str(&message.to_json());
        }
        out
    }
}

impl DashChatMessage {
    fn to_json(&self) -> String {
        format!(
            r#"{{"id":{},"name":"{}","message":"{}","at":{}}}"#,
            self.id,
            edgerun_web_ui::escape_json(&self.handle),
            edgerun_web_ui::escape_json(&self.message),
            self.posted_at
        )
    }
}

fn dash_chat_error_response(status: StatusCode, error: &str) -> Response {
    Response::json(
        status,
        &format!(
            r#"{{"ok":false,"error":"{}"}}"#,
            edgerun_web_ui::escape_json(error)
        ),
    )
    .with_header("Cache-Control", "no-store")
    .with_header("X-Content-Type-Options", "nosniff")
}

fn chat_sender_key(request: &Request) -> String {
    request
        .headers()
        .get("x-forwarded-for")
        .or_else(|| request.headers().get("X-Forwarded-For"))
        .or_else(|| request.headers().get("X-Real-IP"))
        .and_then(|value| value.as_str().split(',').next().map(str::trim))
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| "anonymous".to_string())
}

fn sanitize_chat_input(value: &str, max_len: usize) -> String {
    let mut cleaned = String::new();
    for c in value.chars().filter(|c| !c.is_control()) {
        cleaned.push(c);
    }
    while cleaned.len() > max_len {
        cleaned.pop();
    }
    cleaned.trim().to_string()
}

fn chat_unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn apply_chat_heuristics(
    handle: &str,
    message: &str,
    sender_key: &str,
    now: u64,
) -> Option<&'static str> {
    if handle.is_empty() {
        return Some("name required");
    }
    if handle == "Guest" && sender_key == "anonymous" {
        return Some("please provide a display name");
    }
    if message.len() < 2 {
        return Some("message too short");
    }
    if handle.contains('\n') {
        return Some("invalid formatting");
    }
    if message.contains('\n') && message.len() > 700 {
        return Some("control formatting");
    }
    if message.len() > 800 {
        return Some("message too long");
    }
    if message
        .chars()
        .filter(|value| value.is_ascii_control())
        .next()
        .is_some()
    {
        return Some("invalid characters");
    }
    if now == 0 {
        return Some("clock unavailable");
    }
    if sender_key.is_empty() {
        return Some("sender unavailable");
    }
    if handle.chars().all(char::is_whitespace) {
        return Some("name required");
    }
    None
}

#[derive(Clone)]
struct HostStatsSample {
    total_ticks: u64,
    cpu_count: u64,
    processes: Vec<HostProcessStats>,
}

#[derive(Clone)]
struct HostProcessStats {
    role: &'static str,
    pid: u32,
    cpu_ticks: u64,
    memory_bytes: u64,
    binary_bytes: u64,
}

fn render_host_status_json(cache: &mut HostStatsCache, request_count: u64) -> String {
    match collect_host_stats_sample() {
        Ok(sample) => {
            let cpu_percent = host_cpu_percent(cache.previous.as_ref(), &sample);
            let now = StdInstant::now();
            let rps = match (cache.previous_request_count, cache.previous_request_time) {
                (Some(previous_count), Some(previous_time)) => {
                    let elapsed = now.duration_since(previous_time).as_secs_f64();
                    if elapsed > 0.0 {
                        request_count.saturating_sub(previous_count) as f64 / elapsed
                    } else {
                        0.0
                    }
                }
                _ => 0.0,
            };
            let sessions = count_process_sessions("edgerun-server").unwrap_or(0);
            let memory_bytes: u64 = sample
                .processes
                .iter()
                .map(|process| process.memory_bytes)
                .sum();
            let binary_bytes: u64 = sample
                .processes
                .iter()
                .map(|process| process.binary_bytes)
                .sum();
            let mut processes = String::new();
            for (index, process) in sample.processes.iter().enumerate() {
                if index > 0 {
                    processes.push(',');
                }
                processes.push_str(&format!(
                    "{{\"role\":\"{}\",\"pid\":{},\"memory_bytes\":{},\"binary_bytes\":{}}}",
                    process.role, process.pid, process.memory_bytes, process.binary_bytes
                ));
            }
            cache.previous = Some(sample);
            cache.previous_request_count = Some(request_count);
            cache.previous_request_time = Some(now);
            format!(
                "{{\"ok\":true,\"sessions\":{},\"requests_total\":{},\"requests_per_second\":{:.2},\"memory_bytes\":{},\"cpu_percent\":{:.2},\"binary_bytes\":{},\"processes\":[{}]}}",
                sessions, request_count, rps, memory_bytes, cpu_percent, binary_bytes, processes
            )
        }
        Err(error) => format!(
            "{{\"ok\":false,\"error\":\"{}\"}}",
            edgerun_web_ui::escape_json(&error.to_string())
        ),
    }
}

fn collect_host_stats_sample() -> io::Result<HostStatsSample> {
    let (total_ticks, cpu_count) = read_total_cpu_ticks()?;
    let mut processes = Vec::new();
    if let Some(process) = read_process_stats("edgerun-server", "server") {
        processes.push(process);
    }
    if let Some(process) = read_process_stats("edgerun-dns", "dns") {
        processes.push(process);
    }
    Ok(HostStatsSample {
        total_ticks,
        cpu_count,
        processes,
    })
}

fn host_cpu_percent(previous: Option<&HostStatsSample>, current: &HostStatsSample) -> f64 {
    let Some(previous) = previous else {
        return 0.0;
    };
    let total_delta = current.total_ticks.saturating_sub(previous.total_ticks);
    if total_delta == 0 {
        return 0.0;
    }
    let current_ticks: u64 = current
        .processes
        .iter()
        .map(|process| process.cpu_ticks)
        .sum();
    let previous_ticks: u64 = previous
        .processes
        .iter()
        .map(|process| process.cpu_ticks)
        .sum();
    let process_delta = current_ticks.saturating_sub(previous_ticks);
    (process_delta as f64 * current.cpu_count.max(1) as f64 * 100.0) / total_delta as f64
}

fn read_process_stats(name: &'static str, role: &'static str) -> Option<HostProcessStats> {
    let pid = find_process_pid(name)?;
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let close = stat.rfind(") ")?;
    let fields: Vec<&str> = stat[close + 2..].split_whitespace().collect();
    let utime = fields.get(11)?.parse::<u64>().ok()?;
    let stime = fields.get(12)?.parse::<u64>().ok()?;
    let memory_bytes = read_status_rss_bytes(pid).unwrap_or(0);
    let binary_bytes = process_binary_size(pid);
    Some(HostProcessStats {
        role,
        pid,
        cpu_ticks: utime + stime,
        memory_bytes,
        binary_bytes,
    })
}

fn process_binary_size(pid: u32) -> u64 {
    std::fs::read_link(format!("/proc/{pid}/exe"))
        .ok()
        .and_then(|path| std::fs::metadata(path).ok())
        .map(|metadata| metadata.len())
        .or_else(|| {
            let cmdline = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
            let first = cmdline.split(|byte| *byte == 0).next()?;
            if first.is_empty() {
                return None;
            }
            let path = String::from_utf8_lossy(first);
            std::fs::metadata(path.as_ref())
                .ok()
                .map(|metadata| metadata.len())
        })
        .unwrap_or(0)
}

fn find_process_pid(name: &str) -> Option<u32> {
    let entries = std::fs::read_dir("/proc").ok()?;
    let mut pids = Vec::new();
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(pid_text) = file_name.to_str() else {
            continue;
        };
        let Ok(pid) = pid_text.parse::<u32>() else {
            continue;
        };
        let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default();
        let cmdline = std::fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
        let cmdline = String::from_utf8_lossy(&cmdline).replace('\0', " ");
        if comm.trim() == name || cmdline.contains(name) {
            pids.push(pid);
        }
    }
    pids.into_iter().min()
}

fn read_status_rss_bytes(pid: u32) -> Option<u64> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    for line in status.lines() {
        if let Some(value) = line.strip_prefix("VmRSS:") {
            let kb = value.split_whitespace().next()?.parse::<u64>().ok()?;
            return Some(kb * 1024);
        }
    }
    None
}

fn read_total_cpu_ticks() -> io::Result<(u64, u64)> {
    let stat = std::fs::read_to_string("/proc/stat")?;
    let mut total = 0u64;
    let mut cpu_count = 0u64;
    for line in stat.lines() {
        if let Some(rest) = line.strip_prefix("cpu ") {
            total = rest
                .split_whitespace()
                .filter_map(|value| value.parse::<u64>().ok())
                .sum();
        } else if line
            .as_bytes()
            .get(0..3)
            .map(|prefix| prefix == b"cpu")
            .unwrap_or(false)
            && line
                .as_bytes()
                .get(3)
                .map(|byte| byte.is_ascii_digit())
                .unwrap_or(false)
        {
            cpu_count += 1;
        }
    }
    Ok((total, cpu_count.max(1)))
}

fn count_process_sessions(name: &str) -> io::Result<usize> {
    let Some(pid) = find_process_pid(name) else {
        return Ok(0);
    };
    let inodes = process_socket_inodes(pid)?;
    Ok(count_established_sockets(&inodes))
}

fn process_socket_inodes(pid: u32) -> io::Result<Vec<String>> {
    let mut inodes = Vec::new();
    for entry in std::fs::read_dir(format!("/proc/{pid}/fd"))?.flatten() {
        let Ok(target) = std::fs::read_link(entry.path()) else {
            continue;
        };
        let text = target.to_string_lossy();
        if let Some(inode) = text
            .strip_prefix("socket:[")
            .and_then(|value| value.strip_suffix(']'))
        {
            inodes.push(inode.to_string());
        }
    }
    Ok(inodes)
}

fn count_established_sockets(inodes: &[String]) -> usize {
    count_established_sockets_file("/proc/net/tcp", inodes)
        + count_established_sockets_file("/proc/net/tcp6", inodes)
}

fn count_established_sockets_file(path: &str, inodes: &[String]) -> usize {
    let Ok(content) = std::fs::read_to_string(path) else {
        return 0;
    };
    content
        .lines()
        .skip(1)
        .filter(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            fields.get(3) == Some(&"01")
                && fields
                    .get(9)
                    .map(|inode| inodes.iter().any(|candidate| candidate == inode))
                    .unwrap_or(false)
        })
        .count()
}

fn workspace_modules_from_config(apps: &[BrowserAppSpec]) -> Vec<WorkspaceModule<'_>> {
    let mut modules = Vec::new();
    for app in apps {
        for surface in &app.surfaces {
            modules.push(WorkspaceModule {
                app_id: app.app_id.as_str(),
                title: app.title.as_str(),
                surface: surface.as_str(),
                selector: selector_for_surface(surface),
                wasm: app.module.url.as_str(),
            });
        }
    }
    modules
}

fn selector_for_surface(surface: &str) -> &'static str {
    match surface {
        "mail" => "er-mail-surface",
        "git" | "code" => "er-git-surface",
        "blog" | "build-log" => "er-blog-surface",
        _ => "er-app-surface",
    }
}

fn render_browser_app_catalog(configured_apps: &[BrowserAppSpec]) -> String {
    let fallback = default_browser_apps();
    let apps = if configured_apps.is_empty() {
        &fallback[..]
    } else {
        configured_apps
    };
    let mut out = String::from("{\"apps\":[");
    for (index, app) in apps.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"app_id\":\"{}\",\"title\":\"{}\",\"module\":{{\"url\":\"{}\"",
            edgerun_web_ui::escape_json(&app.app_id),
            edgerun_web_ui::escape_json(&app.title),
            edgerun_web_ui::escape_json(&app.module.url)
        ));
        if let Some(sha256) = &app.module.sha256 {
            out.push_str(&format!(
                ",\"sha256\":\"{}\"",
                edgerun_web_ui::escape_json(sha256)
            ));
        }
        out.push_str("},\"surfaces\":[");
        for (surface_index, surface) in app.surfaces.iter().enumerate() {
            if surface_index > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(&edgerun_web_ui::escape_json(surface));
            out.push('"');
        }
        out.push_str("],\"required_capabilities\":");
        render_browser_capabilities(&mut out, &app.required_capabilities);
        out.push_str(",\"optional_capabilities\":");
        render_browser_capabilities(&mut out, &app.optional_capabilities);
        out.push('}');
    }
    out.push_str("]}");
    out
}

fn render_browser_capabilities(
    out: &mut String,
    capabilities: &[edgerun_config::BrowserAppCapabilitySpec],
) {
    out.push('[');
    for (index, capability) in capabilities.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"selector\":\"{}\",\"operations\":[",
            edgerun_web_ui::escape_json(&capability.selector)
        ));
        for (operation_index, operation) in capability.operations.iter().enumerate() {
            if operation_index > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(&edgerun_web_ui::escape_json(operation));
            out.push('"');
        }
        out.push_str("],\"constraints\":[");
        for (constraint_index, constraint) in capability.constraints.iter().enumerate() {
            if constraint_index > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(&edgerun_web_ui::escape_json(constraint));
            out.push('"');
        }
        out.push_str("]}");
    }
    out.push(']');
}

fn render_browser_apps_surface(configured_apps: &[BrowserAppSpec]) -> String {
    let fallback = default_browser_apps();
    let apps = if configured_apps.is_empty() {
        &fallback[..]
    } else {
        configured_apps
    };
    let mut cards = String::new();
    for app in apps {
        let surfaces = if app.surfaces.is_empty() {
            String::from("No surfaces")
        } else {
            app.surfaces.join(", ")
        };
        let capabilities = render_capability_pills(app);
        cards.push_str(&format!(
            "<article class=\"dash-card dash-app-card\" data-search-card data-search-text=\"{} {} {}\"><strong>{}</strong><span>{}</span><small>{}</small>{}</article>",
            edgerun_web_ui::escape_attr(&app.app_id),
            edgerun_web_ui::escape_attr(&app.title),
            edgerun_web_ui::escape_attr(&surfaces),
            edgerun_web_ui::escape_html(&app.title),
            edgerun_web_ui::escape_html(&app.app_id),
            edgerun_web_ui::escape_html(&surfaces),
            capabilities
        ));
    }
    format!(
        "<div class=\"dash-code\"><section class=\"dash-code-hero\"><p>Browser node</p><h2>Apps</h2><span>Configured Wasm agents and the capability selectors they request.</span></section><section class=\"dash-code-tools\" aria-label=\"App tools\"><label><span>Filter apps</span><input type=\"search\" data-workspace-search-scope placeholder=\"Search apps and capabilities\"></label></section><section class=\"dash-code-summary\" aria-label=\"App summary\"><div><span>Apps</span><strong>{}</strong></div><div><span>Modules</span><strong>{}</strong></div><div><span>Required caps</span><strong>{}</strong></div><div><span>Optional caps</span><strong>{}</strong></div></section><section><h2>Installed apps</h2><div class=\"dash-grid\">{}</div></section><p class=\"dash-search-empty\" data-search-empty hidden>No matching apps.</p></div>",
        apps.len(),
        apps.iter().filter(|app| !app.module.url.is_empty()).count(),
        apps.iter().map(|app| app.required_capabilities.len()).sum::<usize>(),
        apps.iter().map(|app| app.optional_capabilities.len()).sum::<usize>(),
        cards
    )
}

fn render_capability_pills(app: &BrowserAppSpec) -> String {
    let mut out = String::from("<div class=\"pill-row\">");
    for capability in app
        .required_capabilities
        .iter()
        .chain(app.optional_capabilities.iter())
    {
        out.push_str(&format!(
            "<span>{}</span>",
            edgerun_web_ui::escape_html(&capability.selector)
        ));
    }
    if app.required_capabilities.is_empty() && app.optional_capabilities.is_empty() {
        out.push_str("<span>No requested capabilities</span>");
    }
    out.push_str("</div>");
    out
}

fn default_browser_apps() -> Vec<BrowserAppSpec> {
    use edgerun_config::{BrowserAppModuleSpec, BrowserAppSpec};
    vec![
        BrowserAppSpec {
            app_id: "edgerun.mail".to_string(),
            title: "Mail".to_string(),
            module: BrowserAppModuleSpec {
                url: "/modules/mail.wasm".to_string(),
                sha256: None,
            },
            surfaces: vec!["mail".to_string()],
            required_capabilities: Vec::new(),
            optional_capabilities: Vec::new(),
        },
        BrowserAppSpec {
            app_id: "edgerun.git".to_string(),
            title: "Code".to_string(),
            module: BrowserAppModuleSpec {
                url: "/modules/git.wasm".to_string(),
                sha256: None,
            },
            surfaces: vec!["git".to_string()],
            required_capabilities: Vec::new(),
            optional_capabilities: Vec::new(),
        },
        BrowserAppSpec {
            app_id: "edgerun.blog".to_string(),
            title: "Build Log".to_string(),
            module: BrowserAppModuleSpec {
                url: "/modules/blog.wasm".to_string(),
                sha256: None,
            },
            surfaces: vec!["blog".to_string()],
            required_capabilities: Vec::new(),
            optional_capabilities: Vec::new(),
        },
    ]
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
<main id="content" class="dash-shell">
  <section class="dash-shell-inner">
      <section class="dash-workspace" id="dashWorkspace" aria-label="Core services workspace">
      <article class="dash-module" data-surface-module="build-log" data-module-key="build-log">
        <div id="dashSurfaceBuildLog" class="dash-surface" data-active-surface="build-log">Loading build log…</div>
      </article>
      <article class="dash-module" data-surface-module="code" data-module-key="code">
        <div id="dashSurfaceCode" class="dash-surface" data-active-surface="code">Loading code…</div>
      </article>
      <article class="dash-module" data-surface-module="mail" data-module-key="mail">
        <div id="dashSurfaceMail" class="dash-surface" data-active-surface="mail">Loading mail…</div>
      </article>
      <article class="dash-module" data-surface-module="apps" data-module-key="apps">
        <div id="dashSurfaceApps" class="dash-surface" data-active-surface="apps">Loading apps…</div>
      </article>
    </section>
    <section id="dashDock" class="dash-dock" aria-label="Surface launcher dock">
      <button class="dash-dock-button" type="button" data-dock-module="build-log" title="Build Log" aria-label="Open Build Log">🛠</button>
      <button class="dash-dock-button" type="button" data-dock-module="code" title="Code" aria-label="Open Code">💻</button>
      <button class="dash-dock-button" type="button" data-dock-module="mail" title="Mail" aria-label="Open Mail">✉</button>
      <button class="dash-dock-button" type="button" data-dock-module="apps" title="Apps" aria-label="Open Apps">◎</button>
    </section>
  </section>
</main>
"##;

const DASH_STYLE: &str = r#"
.dash-shell{min-height:calc(100vh - var(--topbar-h) - var(--footer-h));background:linear-gradient(180deg,color-mix(in srgb,var(--bg) 90%,var(--panel)) 0,var(--bg) 220px);padding:16px}
.dash-shell-inner{max-width:none;margin:0 auto;display:grid;gap:18px}
.dash-panels-header{display:grid;gap:6px;padding:0 2px 10px;border-bottom:1px solid var(--line)}
.dash-panels-header p{margin:0;color:var(--muted)}
.dash-panels-header h1{margin:8px 0 0}
.dash-panels-header .dash-eyebrow{font-weight:800;letter-spacing:.12em;text-transform:uppercase;font-size:12px;color:var(--accent)}
.dash-tools-grid{display:grid;grid-template-columns:repeat(2,minmax(260px,1fr));gap:14px}
.dash-module-minimized{background:color-mix(in srgb,var(--panel) 80%,transparent)}
.dash-module-minimized .dash-surface{display:none}
.dash-module-minimized .dash-surface-path{display:none}
.dash-surface-path{font-size:12px;color:var(--muted);background:color-mix(in srgb,var(--panel) 84%,transparent);padding:4px 10px;border-radius:999px;border:1px solid var(--line);display:inline-flex}
.dash-surface{height:100%;overflow:auto;padding:12px;border:0;border-radius:0;background:var(--bg)}
.dash-surface:focus-within{outline:2px solid color-mix(in srgb,var(--accent) 45%,transparent);outline-offset:-2px}
.dash-surface-empty{color:var(--muted);font-size:13px}
.dash-surface .dash-surface-empty{background:color-mix(in srgb,var(--panel) 84%,transparent);padding:10px;border:1px dashed var(--line);border-radius:8px}
.dash-surface .dash-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(180px,1fr));gap:14px}
.dash-surface .dash-card{border:1px solid var(--line);border-radius:8px;background:var(--panel);padding:12px;display:block;text-decoration:none;color:var(--text)}
.dash-surface .dash-card:hover{border-color:var(--accent)}
.dash-surface .dash-card span{color:var(--muted)}
.dash-surface .dash-card-button{all:unset;display:block;cursor:pointer;text-align:left}
.dash-surface .dash-card-button:hover{border-color:var(--accent)}
.dash-surface .dash-code{display:grid;gap:12px}
.dash-surface .dash-code-hero{display:grid;gap:8px}
.dash-surface .dash-code-hero h2{margin:0}
.dash-surface .dash-code-tools{display:grid;gap:8px}
.dash-surface .dash-code-tools label{display:grid;gap:4px}
.dash-surface .dash-code-tools span{font-size:12px;color:var(--muted)}
.dash-surface .dash-code-tools input{font:inherit;width:100%;padding:9px 10px;border:1px solid var(--line);border-radius:8px;background:var(--bg);color:var(--text)}
.dash-surface .dash-quick-links{display:grid;grid-template-columns:repeat(auto-fit,minmax(180px,1fr));gap:10px}
.dash-surface .dash-quick-links .dash-card-button{display:block;padding:12px}
.dash-surface .dash-code-summary{display:grid;grid-template-columns:repeat(auto-fit,minmax(130px,1fr));gap:10px}
.dash-surface .dash-code-summary div{display:grid;padding:10px 12px;background:color-mix(in srgb,var(--panel) 86%,var(--code));border:1px solid var(--line);border-radius:8px}
.dash-surface .dash-code-summary span{color:var(--muted);font-size:12px}
.dash-surface .dash-code-summary strong{font-size:22px}
.dash-search-empty{color:var(--muted);margin:2px 4px 0}
.dash-surface .dash-stage .dash-card-button{width:100%}
.dash-surface .dash-surface-content{display:block}
.dash-surface .dash-stage{background:transparent}
.dash-surface .dash-stage header{border-bottom:1px solid var(--line);padding:0 0 10px;margin:0 0 10px}
.dash-surface .dash-stage header strong{font-size:16px}
.dash-surface .dash-stage header span{color:var(--muted)}
.dash-surface .content .related-posts{margin-top:22px;padding-top:18px;border-top:1px solid var(--line)}
.dash-surface .content .related-posts>div{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,220px),1fr));gap:14px;margin-top:12px}
.dash-surface .content .related-posts article{margin:0;border:1px solid var(--line);border-radius:8px;background:color-mix(in srgb,var(--panel) 88%,var(--code));padding:10px}
.dash-surface .content .related-posts article + article{margin-top:10px}
.dash-chat-shell{display:grid;gap:10px}
.dash-chat-form{display:grid;gap:8px}
.dash-chat-form label{display:grid;gap:6px}
.dash-chat-form span{font-size:12px;font-weight:750;color:var(--muted);text-transform:uppercase;letter-spacing:.08em}
.dash-chat-form input,.dash-chat-form textarea{font:inherit;width:100%;padding:9px 10px;border:1px solid var(--line);border-radius:8px;background:var(--bg);color:var(--text)}
.dash-chat-form textarea{min-height:100px;resize:vertical}
.dash-chat-form button{justify-self:end;width:110px;height:34px;border:1px solid var(--line);border-radius:8px;background:var(--bg);font:inherit;color:var(--text);cursor:pointer}
.dash-chat-form button:hover{border-color:var(--accent);color:var(--accent)}
.dash-chat-status{margin:0;min-height:18px;font-size:12px;color:var(--muted)}
.dash-chat-log{display:grid;gap:10px;min-height:180px;max-height:260px;overflow:auto;padding-right:2px}
.dash-chat-entry{padding:10px;border:1px solid var(--line);border-radius:8px;background:var(--panel);display:grid;gap:6px}
.dash-chat-meta{display:flex;justify-content:space-between;align-items:center;gap:10px;color:var(--muted);font-size:12px}
.dash-chat-name{font-weight:800;color:var(--text)}
.dash-chat-text{line-height:1.45;white-space:pre-wrap;overflow-wrap:anywhere}
.dash-chat-log .dash-chat-entry + .dash-chat-entry{margin-top:10px}
.dash-chat-log .dash-chat-entry{line-height:1.4}
body{--footer-h:42px;--topbar-h:0px}
.topbar{display:none!important}
.dash-panels-header,
.dash-tools-grid,
.dash-module-minimized .dash-surface-path{display:none!important}
.dash-shell{
  min-height:calc(100vh - var(--footer-h));
  background:linear-gradient(180deg,color-mix(in srgb,var(--bg) 92%,var(--panel)) 0,var(--bg) 220px);
  padding:12px 12px 88px;
}
.dash-shell-inner{
  margin:0;
  width:100%;
  max-width:none;
  display:block;
  gap:0;
}
.dash-workspace{
  position:relative;
  min-height:calc(100vh - 188px);
  height:calc(100vh - 188px);
  overflow:hidden;
}
.dash-workspace::before{
  content:'';
  position:fixed;
  inset:0;
  pointer-events:none;
  background:radial-gradient(1200px 340px at 18% -2%, color-mix(in srgb,var(--panel) 72%,transparent), transparent 72%), radial-gradient(980px 260px at 84% 20%, color-mix(in srgb,var(--panel) 66%,transparent), transparent 78%);
  z-index:-1;
}
.dash-module{
  position:absolute;
  box-sizing:border-box;
  min-width:320px;
  min-height:230px;
  max-width:clamp(320px, min(85vw, 760px), 760px);
  max-height:calc(100vh - 210px);
  width:min(380px, calc(100vw - 22px));
  height:min(420px, calc(100vh - 210px));
  display:block;
  gap:0;
  padding:0;
  background:var(--panel);
  border:1px solid var(--line);
  border-radius:14px;
  box-shadow:0 13px 34px color-mix(in srgb,var(--text) 8%,transparent);
  z-index:10;
  user-select:auto;
  transition:transform .18s ease, box-shadow .18s ease, width .14s ease, height .14s ease, left .14s ease, top .14s ease;
  resize:both;
  overflow:hidden;
}
.dash-module.dash-module-dragging{
  opacity:.65;
  box-shadow:0 1px 0 color-mix(in srgb,var(--text) 10%,transparent),0 12px 24px color-mix(in srgb,var(--text) 18%,transparent);
}
.dash-module:active{
  cursor:grab;
}
.dash-workspace.dash-maximized .dash-module{display:none}
.dash-workspace.dash-maximized .dash-module.dash-module-maximized{display:block; inset:10px; left:10px; top:10px; width:calc(100vw - 34px); height:calc(100vh - 204px)}
.dash-module.dash-module-minimized{height:56px;overflow:hidden}
.dash-module.dash-module-maximized{inset:10px; left:10px; top:10px; width:calc(100vw - 34px); height:calc(100vh - 204px)}
.dash-module.dash-module-maximized .dash-surface{height:100%}
.dash-module:not(.dash-module-maximized) .dash-surface{
  height:100%;
}
.dash-module.is-dragging{
  cursor:grabbing;
  box-shadow:0 0 0 1px color-mix(in srgb,var(--accent) 35%,transparent),0 18px 36px color-mix(in srgb,var(--text) 15%,transparent);
}
.dash-module.dash-module-focused{
  z-index:20;
}
.dash-dock{
  position:fixed;
  left:50%;
  bottom:calc(var(--footer-h) + 14px);
  transform:translateX(-50%);
  display:flex;
  gap:10px;
  align-items:center;
  padding:10px;
  border:1px solid color-mix(in srgb,var(--line) 80%,transparent);
  border-radius:999px;
  background:transparent;
  backdrop-filter:blur(6px);
  z-index:20;
  box-shadow:none;
}
.dash-dock-button{
  --dash-icon-scale: 1;
  --dash-icon-dy: 0px;
  --dash-icon-size: 22px;
  width:34px;
  height:34px;
  border-radius:999px;
  border:1px solid color-mix(in srgb,var(--line) 70%,transparent);
  background:transparent;
  color:var(--text);
  display:flex;
  align-items:center;
  justify-content:center;
  cursor:pointer;
  font-size:var(--dash-icon-size);
  line-height:1;
  padding:0;
  transform:translateY(var(--dash-icon-dy)) scale(var(--dash-icon-scale));
  transition:transform .18s ease,border-color .18s ease;
}
.dash-dock-button:hover,
.dash-dock-button:focus-visible{
  border-color:var(--accent);
  transform:translateY(calc(var(--dash-icon-dy) - 1px)) scale(calc(var(--dash-icon-scale) * 1.01));
}
.dash-dock-button.is-restored{
  border-color:color-mix(in srgb,var(--accent) 45%,transparent);
  box-shadow:0 8px 18px color-mix(in srgb,var(--accent) 24%,transparent);
}
.dash-icon-button{
  height:30px;
  width:30px;
  min-width:30px;
  border:1px solid color-mix(in srgb,var(--line) 60%,transparent);
  border-radius:999px;
  background:transparent;
  color:var(--muted);
  font-size:14px;
  line-height:1;
  display:inline-flex;
  align-items:center;
  justify-content:center;
  padding:0;
  cursor:pointer;
}
.dash-chat-panel-button,
.dash-status button,
.dash-status-refresh,
.dash-chat-bubble{
  height:30px;
  width:30px;
  min-width:30px;
  border:1px solid color-mix(in srgb,var(--line) 70%,transparent);
  border-radius:999px;
  background:transparent;
  color:var(--muted);
  display:inline-flex;
  align-items:center;
  justify-content:center;
  padding:0;
  cursor:pointer;
  line-height:1;
}
.dash-chat-panel-button,
.dash-status-refresh{font-size:13px;}
.dash-chat-bubble{font-size:18px;line-height:1}
.dash-chat-panel-button:hover,
.dash-status button:hover,
.dash-status-refresh:hover,
.dash-chat-bubble:hover,
.dash-chat-panel-button:focus-visible,
.dash-status button:focus-visible,
.dash-status-refresh:focus-visible,
.dash-chat-bubble:focus-visible{
  color:var(--text);
  border-color:var(--accent);
}
.dash-chat-dock{
  position:relative;
  display:inline-flex;
  align-items:flex-end;
  margin-right:2px;
}
.dash-chat-bubble{
  box-shadow:0 10px 24px color-mix(in srgb,var(--text) 12%,transparent);
}
.dash-chat-bubble span{line-height:1}
.dash-chat-panel{
  position:absolute;
  right:0;
  bottom:calc(100% + 12px);
  width:min(390px, calc(100vw - 24px));
  border:1px solid var(--line);
  border-radius:14px;
  background:var(--panel);
  padding:12px;
  display:grid;
  gap:10px;
  max-height:calc(100vh - 200px);
  box-shadow:0 13px 28px color-mix(in srgb,var(--text) 18%,transparent);
  transition:max-height .25s ease, opacity .2s ease, transform .25s ease;
  transform-origin:100% 100%;
}
.dash-chat-dock:not(.is-open) .dash-chat-panel,
.dash-chat-dock.is-minimized .dash-chat-panel{
  display:none;
}
.dash-chat-dock.is-open .dash-chat-panel{
  display:grid;
}
.dash-chat-dock.is-closed .dash-chat-panel{
  display:none;
}
.dash-chat-panel.dash-chat-closed{
  display:none;
}
.dash-chat-panel-head{
  display:flex;
  align-items:center;
  justify-content:space-between;
  gap:8px;
}
.dash-chat-panel-head h2{
  margin:0;
  font-size:16px;
}
.dash-chat-panel-controls{
  display:flex;
  gap:8px;
}
.dash-chat-panel-button{
  border-color:color-mix(in srgb,var(--line) 70%,transparent);
}
.dash-status-footer{
  min-height:42px;
  height:42px;
  display:flex;
  align-items:center;
  justify-content:center;
  padding:0 12px;
  overflow:visible;
  position:fixed;
  left:0;
  right:0;
  bottom:0;
  z-index:30;
}
.dash-status{width:100%;display:flex;align-items:center;gap:14px;white-space:nowrap;font-size:12px;line-height:1;min-width:0}
.dash-status span{display:inline-flex;align-items:baseline;gap:5px;color:var(--muted)}
.dash-status strong{color:var(--text);font-weight:800}
.dash-status-actions{margin-left:auto;display:flex;align-items:flex-end;gap:12px;position:relative}
.dash-status-refresh{
  width:30px;
  height:30px;
  display:inline-flex;
  align-items:center;
  justify-content:center;
  border:1px solid color-mix(in srgb,var(--line) 70%,transparent);
  border-radius:10px;
  color:var(--muted);
}
.dash-status-refresh span{line-height:1;font-size:16px}
.dash-status-refresh.is-loading span{animation:dash-spin 1s linear infinite}
.dash-status-theme er-theme-toggle{
  display:block;
  transform:scale(0.95);
}
.dash-module-context-menu{
  position:fixed;
  min-width:176px;
  display:none;
  gap:6px;
  padding:6px;
  border:1px solid color-mix(in srgb,var(--line) 72%,transparent);
  border-radius:12px;
  background:color-mix(in srgb,var(--panel) 96%,transparent);
  backdrop-filter:blur(8px);
  box-shadow:0 14px 34px color-mix(in srgb,var(--text) 18%,transparent);
  z-index:45;
  user-select:none;
}
.dash-module-context-menu.is-open{display:grid}
.dash-module-context-menu button{
  border:0;
  border-radius:10px;
  background:transparent;
  color:var(--text);
  font:inherit;
  padding:9px 10px;
  text-align:left;
  display:flex;
  align-items:center;
  gap:8px;
  cursor:pointer;
}
.dash-module-context-menu button:hover{
  background:color-mix(in srgb,var(--panel) 88%,transparent);
}
@keyframes dash-spin{
  to{transform:rotate(360deg)}
}
@media(max-width:1200px){.dash-tools-grid{grid-template-columns:1fr}.dash-dock-grid{grid-template-columns:repeat(2,minmax(0,1fr))}}
@media(max-width:1000px){
  .dash-workspace{display:block}
  .dash-module{
    width:calc(100vw - 24px);
    max-width:100%;
  }
}
@media(max-width:760px){.dash-shell{padding:10px 10px 96px}.dash-shell-inner{gap:12px}.dash-status{justify-content:flex-start;overflow-x:auto;gap:12px}}
"#;

const DASH_COHESIVE_JS: &str = r#"
const DASH_SURFACES = [
  {
    key: 'build-log',
    label: 'Build Log',
    pathPrefix: '/surface/blog',
    slotId: 'dashSurfaceBuildLog',
    fallback: 'Build log is unavailable right now.',
  },
  {
    key: 'code',
    label: 'Code',
    pathPrefix: '/surface/git',
    slotId: 'dashSurfaceCode',
    fallback: 'Code surface is unavailable right now.',
  },
  {
    key: 'mail',
    label: 'Mail',
    pathPrefix: '/surface/mail',
    slotId: 'dashSurfaceMail',
    fallback: 'Mail surface is unavailable right now.',
  },
  {
    key: 'apps',
    label: 'Apps',
    pathPrefix: '/surface/apps',
    slotId: 'dashSurfaceApps',
    fallback: 'Apps surface is unavailable right now.',
  },
];

const DASH_WORKSPACE_STATE_KEY = 'edgerun-dashboard-workspace-v1';
const DASH_CHAT_STATE_KEY = 'edgerun-dashboard-chat-state-v1';
const DASH_CHAT_MIN_DELAY_MS = 2400;
const DASH_CHAT_DUP_WINDOW_MS = 12000;
const DASH_CHAT_REPEAT_WINDOW_LIMIT = 4;
const DASH_CHAT_MESSAGE_MAX = 800;
const DASH_MODULE_MIN_WIDTH = 260;
const DASH_MODULE_MIN_HEIGHT = 220;
const DASH_WORKSPACE_PADDING = 12;
const DASH_MODULE_EDGE_GAP = 12;

function clamp(value, min, max) {
  return Math.min(Math.max(value, min), max);
}

function clampModuleGeometry(left, top, width, height, workspace = getWorkspaceRoot()) {
  if (!workspace) {
    return null;
  }
  const workspaceWidth = Math.max(0, workspace.clientWidth - DASH_WORKSPACE_PADDING * 2);
  const workspaceHeight = Math.max(0, workspace.clientHeight - DASH_WORKSPACE_PADDING * 2);
  const safeWidth = clamp(
    width,
    DASH_MODULE_MIN_WIDTH,
    Math.max(DASH_MODULE_MIN_WIDTH, workspaceWidth),
  );
  const safeHeight = clamp(
    height,
    DASH_MODULE_MIN_HEIGHT,
    Math.max(DASH_MODULE_MIN_HEIGHT, workspaceHeight),
  );
  const maxLeft = Math.max(
    DASH_WORKSPACE_PADDING,
    workspace.clientWidth - safeWidth - DASH_WORKSPACE_PADDING,
  );
  const maxTop = Math.max(
    DASH_WORKSPACE_PADDING,
    workspace.clientHeight - safeHeight - DASH_WORKSPACE_PADDING,
  );
  return {
    left: clamp(left, DASH_WORKSPACE_PADDING, maxLeft),
    top: clamp(top, DASH_WORKSPACE_PADDING, maxTop),
    width: safeWidth,
    height: safeHeight,
  };
}

function parseGeometry(raw = {}, fallback = {}) {
  const toNumber = (value) => {
    const parsed = Number.parseFloat(value);
    return Number.isFinite(parsed) ? parsed : null;
  };
  const left = toNumber(raw.left);
  const top = toNumber(raw.top);
  const width = toNumber(raw.width);
  const height = toNumber(raw.height);
  if (left === null || top === null || width === null || height === null) {
    return null;
  }
  return { left, top, width, height };
}

function applyModuleGeometry(module, geometry) {
  if (!module || !geometry) {
    return;
  }
  const clamped = clampModuleGeometry(
    geometry.left,
    geometry.top,
    geometry.width,
    geometry.height,
  );
  if (!clamped) {
    return;
  }
  module.style.left = `${clamped.left}px`;
  module.style.top = `${clamped.top}px`;
  module.style.width = `${clamped.width}px`;
  module.style.height = `${clamped.height}px`;
}

function getModuleGeometry(module) {
  if (!module) {
    return null;
  }
  return parseGeometry({
    left: module.style.left,
    top: module.style.top,
    width: module.style.width,
    height: module.style.height,
  });
}

function applyDefaultLayout() {
  const workspace = getWorkspaceRoot();
  if (!workspace) return;
  const modules = getWorkspaceModules();
  if (!modules.length) {
    return;
  }
  const width = Math.max(0, workspace.clientWidth - DASH_WORKSPACE_PADDING * 2);
  const columns = width >= 980 ? 2 : 1;
  const moduleWidth = clamp(
    columns === 2 ? (width - DASH_MODULE_EDGE_GAP) / 2 : width,
    DASH_MODULE_MIN_WIDTH,
    Math.min(520, width),
  );
  const moduleHeight = clamp(
    Math.max(DASH_MODULE_MIN_HEIGHT, Math.floor(workspace.clientHeight * 0.43)),
    DASH_MODULE_MIN_HEIGHT,
    460,
  );
  for (const module of modules) {
    const geometry = getModuleGeometry(module);
    if (geometry) {
      applyModuleGeometry(module, geometry);
      continue;
    }
    const index = modules.indexOf(module);
    const left = DASH_WORKSPACE_PADDING + (index % columns) * (moduleWidth + DASH_MODULE_EDGE_GAP);
    const top = DASH_WORKSPACE_PADDING + Math.floor(index / columns) * (moduleHeight + DASH_MODULE_EDGE_GAP);
    applyModuleGeometry(module, { left, top, width: moduleWidth, height: moduleHeight });
  }
}

function focusModule(module) {
  if (!module) return;
  const workspace = getWorkspaceRoot();
  if (!workspace) return;
  let topLayer = 1000;
  for (const next of getWorkspaceModules()) {
    if (next === module) continue;
    const z = Number.parseInt(next.style.zIndex, 10);
    if (Number.isFinite(z) && z > topLayer) {
      topLayer = z;
    }
    next.classList.remove('dash-module-focused');
  }
  module.style.zIndex = String(topLayer + 1);
  module.classList.add('dash-module-focused');
}

function getWorkspaceRoot() {
  return document.getElementById('dashWorkspace');
}

function getSurfaceForKey(key) {
  return DASH_SURFACES.find((surface) => surface.key === key);
}

function getModuleByKey(key) {
  const workspace = getWorkspaceRoot();
  if (!workspace || !key) {
    return null;
  }
  return workspace.querySelector(`[data-surface-module="${key}"]`);
}

function getWorkspaceModules() {
  const workspace = getWorkspaceRoot();
  if (!workspace) {
    return [];
  }
  return [...workspace.querySelectorAll('[data-surface-module]')];
}

function getModuleKeyFromElement(element) {
  return element ? element.getAttribute('data-surface-module') : null;
}

function currentMaximizedModule() {
  const workspace = getWorkspaceRoot();
  return workspace?.getAttribute('data-maximized-module') || '';
}

function loadWorkspaceState() {
  try {
    const raw = localStorage.getItem(DASH_WORKSPACE_STATE_KEY);
    if (!raw) {
      return {};
    }
    const state = JSON.parse(raw);
    return state && typeof state === 'object' ? state : {};
  } catch (_error) {
    return {};
  }
}

function persistWorkspaceState() {
  const workspace = getWorkspaceRoot();
  if (!workspace) return;
  const modules = getWorkspaceModules();
  const geometry = {};
  for (const module of modules) {
    const key = module.dataset.moduleKey;
    if (!key) continue;
    const moduleGeometry = getModuleGeometry(module);
    if (moduleGeometry) {
      geometry[key] = moduleGeometry;
    }
  }
  const state = {
    order: modules.map((module) => module.dataset.moduleKey).filter(Boolean),
    minimized: modules
      .filter((module) => module.classList.contains('dash-module-minimized'))
      .map((module) => module.dataset.moduleKey)
      .filter(Boolean),
    maximized: workspace.getAttribute('data-maximized-module') || '',
    geometry,
  };
  try {
    localStorage.setItem(DASH_WORKSPACE_STATE_KEY, JSON.stringify(state));
  } catch (_error) {
    // ignore persistence failures
  }
}

function applyModuleOrder(order) {
  const workspace = getWorkspaceRoot();
  if (!workspace || !Array.isArray(order)) {
    return;
  }
  const modules = getWorkspaceModules();
  const byKey = new Map(modules.map((module) => [module.dataset.moduleKey, module]));
  const seen = new Set();
  for (const key of order) {
    const module = byKey.get(key);
    if (!module || seen.has(key)) {
      continue;
    }
    workspace.appendChild(module);
    seen.add(key);
  }
  for (const module of modules) {
    if (!seen.has(module.dataset.moduleKey)) {
      workspace.appendChild(module);
    }
  }
}

function refreshDockButtonState() {
  const buttons = document.querySelectorAll('.dash-dock-button[data-dock-module]');
  for (const button of buttons) {
    const key = button.getAttribute('data-dock-module');
    const module = getModuleByKey(key);
    if (!module) {
      continue;
    }
    button.classList.toggle('is-restored', !module.classList.contains('dash-module-minimized'));
  }
}

function setModuleMinimized(module, minimized) {
  if (!module) return;
  if (minimized) {
    module.classList.add('dash-module-minimized');
  } else {
    module.classList.remove('dash-module-minimized');
  }
  refreshDockButtonState();
}

function isInteractiveSurfaceTarget(target) {
  if (!target) {
    return false;
  }
  return !!target.closest(
    'a, button, input, textarea, select, option, label, .dash-chat-panel-button, [data-chat-action], [data-surface], [href], [data-module-context-action]',
  );
}

function ensureModuleContextMenu() {
  const existing = document.getElementById('dashModuleContextMenu');
  if (existing) {
    return existing;
  }
  const menu = document.createElement('section');
  menu.id = 'dashModuleContextMenu';
  menu.className = 'dash-module-context-menu';
  menu.setAttribute('role', 'menu');
  menu.setAttribute('aria-hidden', 'true');
  document.body.appendChild(menu);
  return menu;
}

function hideModuleContextMenu() {
  const menu = document.getElementById('dashModuleContextMenu');
  if (!menu) return;
  menu.classList.remove('is-open');
  menu.setAttribute('aria-hidden', 'true');
}

function clampMenuPosition(event) {
  const viewportHeight = Math.max(0, window.innerHeight - 12);
  const viewportWidth = Math.max(0, window.innerWidth - 12);
  const menu = ensureModuleContextMenu();
  const rect = menu.getBoundingClientRect();
  const estimatedWidth = Math.max(170, rect.width || 170);
  const estimatedHeight = Math.max(150, rect.height || 150);
  return {
    left: clamp(event.clientX, 12, viewportWidth - estimatedWidth),
    top: clamp(event.clientY, 12, viewportHeight - estimatedHeight),
  };
}

async function applyModuleContextAction(module, action) {
  const key = getModuleKeyFromElement(module);
  const surface = getSurfaceForKey(key);
  if (!key) return;
  const maximized = currentMaximizedModule();
  if (action === 'open') {
    if (module.classList.contains('dash-module-minimized')) {
      setModuleMinimized(module, false);
    }
    focusModule(module);
    if (maximized && maximized !== key) {
      setMaximizedModule('');
    }
    if (surface) {
      await hydrateSurface(surface, surface.pathPrefix);
    }
    return;
  }
  if (action === 'minimize') {
    const shouldMinimize = !module.classList.contains('dash-module-minimized');
    setModuleMinimized(module, shouldMinimize);
    if (shouldMinimize && maximized === key) {
      setMaximizedModule('');
    }
    return;
  }
  if (action === 'maximize') {
    setMaximizedModule(maximized === key ? '' : key);
    focusModule(module);
    return;
  }
  if (action === 'refresh') {
    if (surface) {
      await hydrateSurface(surface, surface.pathPrefix);
    }
  }
}

function setMaximizedModule(key) {
  const workspace = getWorkspaceRoot();
  if (!workspace) return;
  const modules = getWorkspaceModules();
  workspace.classList.remove('dash-maximized');
  workspace.removeAttribute('data-maximized-module');
  for (const module of modules) {
    module.classList.remove('dash-module-maximized');
  }
  if (!key) {
    refreshDockButtonState();
    persistWorkspaceState();
    return;
  }
  const module = getModuleByKey(key);
  if (!module) return;
  setModuleMinimized(module, false);
  module.classList.add('dash-module-maximized');
  workspace.classList.add('dash-maximized');
  workspace.setAttribute('data-maximized-module', key);
  refreshDockButtonState();
  persistWorkspaceState();
}

function hydrateAll() {
  return Promise.all(
    DASH_SURFACES.map((surface) => hydrateSurface(surface, surface.pathPrefix)),
  );
}

function applyWorkspaceState(rawState) {
  const workspace = getWorkspaceRoot();
  if (!workspace) return;
  const state = rawState || {};
  const available = DASH_SURFACES.map((surface) => surface.key);
  const order = Array.isArray(state.order) ? state.order.filter((key) => available.includes(key)) : [];
  if (order.length === available.length) {
    applyModuleOrder(order);
  }
  const minimized = new Set(
    Array.isArray(state.minimized) ? state.minimized.filter((key) => available.includes(key)) : [],
  );
  for (const module of getWorkspaceModules()) {
    setModuleMinimized(module, minimized.has(module.dataset.moduleKey));
  }
  const savedGeometry =
    state.geometry && typeof state.geometry === 'object' ? state.geometry : {};
  for (const module of getWorkspaceModules()) {
    const key = module.dataset.moduleKey;
    const rawGeometry = key ? parseGeometry(savedGeometry[key] || {}) : null;
    if (!key || !rawGeometry) {
      continue;
    }
    applyModuleGeometry(module, rawGeometry);
  }
  applyDefaultLayout();
  const maximized = typeof state.maximized === 'string' ? state.maximized : '';
  if (maximized && available.includes(maximized)) {
    setMaximizedModule(maximized);
  } else {
    setMaximizedModule('');
  }
  refreshDockButtonState();
}

function initWorkspaceState() {
  applyWorkspaceState(loadWorkspaceState());
}

function normalizeSurfacePath(path) {
  return path.startsWith('/') ? path : `/${path}`;
}

function pathToSurfaceKey(path) {
  if (!path) {
    return null;
  }
  if (path.startsWith('/surface/blog')) {
    return 'build-log';
  }
  if (path.startsWith('/surface/git')) {
    return 'code';
  }
  if (path.startsWith('/surface/mail')) {
    return 'mail';
  }
  if (path.startsWith('/surface/apps')) {
    return 'apps';
  }
  return null;
}

function formatBytes(bytes) {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return '--';
  }
  const units = ['B', 'KB', 'MB', 'GB'];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return (value >= 10 || unit === 0 ? value.toFixed(0) : value.toFixed(1)) + ' ' + units[unit];
}

  function extractSurfaceContent(html) {
    const doc = new DOMParser().parseFromString(html, 'text/html');
    const wrapped = doc.querySelector('[data-dash-surface-root]') || doc.querySelector('#surfaceSlot .dash-surface');
    if (wrapped) {
      return wrapped.innerHTML;
    }
  return doc.querySelector('.dash-surface')?.innerHTML || html;
}

async function hydrateSurface(surface, path) {
  if (!surface) {
    return;
  }
  const fallback = surface.fallback;
  const resolvedPath = normalizeSurfacePath(path || surface.pathPrefix);
  const slot = document.getElementById(surface.slotId);
  if (!slot) {
    return;
  }
  slot.classList.remove('dash-surface-empty');
  try {
    const response = await fetch(resolvedPath, { cache: 'no-store' });
    if (!response.ok) {
      throw new Error('surface unavailable');
    }
    const html = await response.text();
    slot.innerHTML = `<div class="dash-surface-content">${extractSurfaceContent(html)}</div>`;
  } catch (_error) {
    slot.textContent = fallback;
    slot.classList.add('dash-surface-empty');
  }
  bindSurfaceSearch(slot);
}

function bindSurfaceSearch(container) {
  const input = container.querySelector('[data-workspace-search-scope]');
  if (!input) {
    return;
  }
  const cards = [...container.querySelectorAll('[data-search-card]')];
  const empty = container.querySelector('[data-search-empty]');
  if (!cards.length) {
    return;
  }
  const apply = () => {
    const term = input.value.trim().toLowerCase();
    let visible = 0;
    for (const card of cards) {
      const haystack = (card.getAttribute('data-search-text') || '').toLowerCase();
      const show = term.length === 0 || haystack.includes(term);
      card.hidden = !show;
      if (show) {
        visible += 1;
      }
    }
    if (empty) {
      empty.hidden = visible > 0;
    }
  };
  input.addEventListener('input', apply);
  apply();
}

function resolveSurfaceFromClickTarget(target) {
  const raw =
    target.getAttribute('hx-get') ||
    target.getAttribute('href');
  if (!raw) {
    return null;
  }
  if (raw.startsWith('#')) {
    return null;
  }
  if (raw.startsWith('http')) {
    const parsed = new URL(raw, location.origin);
    if (parsed.origin !== location.origin) {
      return null;
    }
    if (!parsed.pathname.startsWith('/surface/')) {
      return null;
    }
    return {
      key: pathToSurfaceKey(parsed.pathname),
      path: parsed.pathname,
    };
  }
  if (raw.startsWith('/surface/')) {
    return { key: pathToSurfaceKey(raw), path: raw };
  }
  return null;
}

function bindSurfaceLinks() {
  document.addEventListener('click', (event) => {
    const button = event.target.closest('[hx-get], [href], [data-surface]');
    if (!button) {
      return;
    }
    const resolved = resolveSurfaceFromClickTarget(button);
    if (!resolved || !resolved.path || !resolved.key) {
      return;
    }
    const container = button.closest('[data-surface-module]');
    const explicit = button.getAttribute('data-surface');
    const moduleKey = explicit || getModuleKeyFromElement(container);
    const surface =
      getSurfaceForKey(moduleKey) ||
      getSurfaceForKey(resolved.key);
    if (!surface) {
      return;
    }
    event.preventDefault();
    const resolvedPath = normalizeSurfacePath(resolved.path);
    hydrateSurface(surface, resolvedPath);
  });
}

async function refreshDashStatus(options = {}) {
  const setText = (selector, text) => {
    for (const node of document.querySelectorAll(selector)) {
      node.textContent = text;
    }
  };
  const setRefreshState = (disabled) => {
    for (const node of document.querySelectorAll('[data-status-refresh]')) {
      node.disabled = disabled;
      node.classList.toggle('is-loading', disabled);
    }
  };
  try {
    if (options.manual) {
      setRefreshState(true);
    }
    const response = await fetch('/status.json', { cache: 'no-store' });
    if (!response.ok) return;
    const data = await response.json();
    if (!data.ok) return;
    setText('[data-status-sessions]', String(data.sessions));
    setText('[data-status-rps]', (Number(data.requests_per_second) || 0).toFixed(1));
    setText('[data-status-memory]', formatBytes(data.memory_bytes));
    setText('[data-status-cpu]', (Number(data.cpu_percent) || 0).toFixed(1) + '%');
    setText('[data-status-binary]', formatBytes(data.binary_bytes));
  } catch (_error) {
    // no-op on status refresh failure
  } finally {
    setRefreshState(false);
  }
}

function chatMessageTemplate(message) {
  const safeName = String(message.name || 'Guest');
  const safeText = String(message.message || '');
  return `<article class="dash-chat-entry"><header class="dash-chat-meta"><span class="dash-chat-name">${escapeHtml(safeName)}</span><time>${new Date((message.at || 0) * 1000).toLocaleTimeString([], {hour:'2-digit', minute:'2-digit'})}</time></header><p class="dash-chat-text">${escapeHtml(safeText)}</p></article>`;
}

function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, (match) => {
    const map = {
      '&': '&amp;',
      '<': '&lt;',
      '>': '&gt;',
      '"': '&quot;',
      "'": '&#39;',
    };
    return map[match];
  });
}

async function refreshChatLog() {
  const status = document.getElementById('dashChatStatus');
  const log = document.getElementById('dashChatLog');
  if (!log) return;
  try {
    const response = await fetch('/api/chat', { cache: 'no-store' });
    if (!response.ok) {
      if (status) status.textContent = 'Unable to load chat messages.';
      log.innerHTML = '';
      return;
    }
    const data = await response.json();
    const messages = (data && Array.isArray(data.messages)) ? data.messages : [];
    if (messages.length === 0) {
      log.innerHTML = '<p class="dash-surface-empty">No messages yet.</p>';
      if (status) {
        status.textContent = '';
      }
      return;
    }
    log.innerHTML = messages.map(chatMessageTemplate).join('');
    log.scrollTop = log.scrollHeight;
  } catch (_error) {
    if (status) status.textContent = 'Unable to load chat messages.';
  }
}

function wireGlobalChat() {
  const form = document.getElementById('dashChatForm');
  const status = document.getElementById('dashChatStatus');
  const nameInput = document.getElementById('dashChatName');
  const messageInput = document.getElementById('dashChatMessage');
  const submit = document.getElementById('dashChatSend');
  const chatState = {
    lastPostTs: 0,
    messageHistory: new Map(),
  };

  function isLikelySpam(name, message) {
    const normalizedName = String(name || '').trim().toLowerCase();
    const normalizedMessage = String(message || '').trim();
    if (!normalizedName || !normalizedMessage) {
      return 'Name and message are required.';
    }
    if (normalizedName.length > 24) {
      return 'Name is too long.';
    }
    if (normalizedMessage.length > DASH_CHAT_MESSAGE_MAX) {
      return `Message must be under ${DASH_CHAT_MESSAGE_MAX} characters.`;
    }
    if (/(.)\1{12,}/.test(normalizedMessage)) {
      return 'Please avoid repetitive characters.';
    }
    const now = Date.now();
    if (now - chatState.lastPostTs < DASH_CHAT_MIN_DELAY_MS) {
      return 'Please wait before posting again.';
    }
    const history = (chatState.messageHistory.get(normalizedName) || []).filter(
      (entry) => now - entry.time < DASH_CHAT_DUP_WINDOW_MS,
    );
    if (history.length >= DASH_CHAT_REPEAT_WINDOW_LIMIT) {
      if (history.some((entry) => entry.message === normalizedMessage)) {
        return 'This looks spammy. Please adjust message.';
      }
      if (history.length > DASH_CHAT_REPEAT_WINDOW_LIMIT) {
        return 'Posting too frequently from this name.';
      }
    }
    if (history.some((entry) => entry.message === normalizedMessage)) {
      return 'You already posted this message just now.';
    }
    history.push({
      message: normalizedMessage,
      time: now,
    });
    chatState.messageHistory.set(normalizedName, history);
    chatState.lastPostTs = now;
    return '';
  }

  if (!form || !status || !nameInput || !messageInput || !submit) return;
  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    const name = nameInput.value.trim();
    const message = messageInput.value.trim();
    const reason = isLikelySpam(name, message);
    if (reason) {
      status.textContent = reason;
      return;
    }
    submit.disabled = true;
    submit.textContent = 'Sending…';
    status.textContent = '';
    try {
      const response = await fetch('/api/chat', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Accept: 'application/json',
        },
        body: JSON.stringify({
          name,
          message,
        }),
      });
      const result = await response.json();
      if (!response.ok || !result || result.ok !== true) {
        status.textContent = result?.error || 'Post failed';
        return;
      }
      nameInput.value = name;
      messageInput.value = '';
      await refreshChatLog();
      status.textContent = 'Posted.';
      setTimeout(() => {
        status.textContent = '';
      }, 1500);
    } catch (_error) {
      status.textContent = 'Could not send chat message.';
    } finally {
      submit.disabled = false;
      submit.textContent = 'Post';
    }
  });
}

function getChatDockState() {
  try {
    const raw = localStorage.getItem(DASH_CHAT_STATE_KEY);
    if (raw === 'open' || raw === 'minimized' || raw === 'closed') {
      return raw;
    }
  } catch (_error) {
    // ignore
  }
  return 'minimized';
}

function setChatDockState(state) {
  const dock = document.getElementById('dashChatDock');
  const panel = document.getElementById('dashChatPanel');
  if (!dock || !panel) return;
  const normalized = state === 'minimized' || state === 'closed' ? state : 'open';
  dock.classList.remove('is-open', 'is-minimized', 'is-closed');
  panel.classList.remove('dash-chat-minimized', 'dash-chat-closed');
  if (normalized === 'open') {
    dock.classList.add('is-open');
  }
  if (normalized === 'minimized') {
    dock.classList.add('is-minimized');
    panel.classList.add('dash-chat-minimized');
  }
  if (normalized === 'closed') {
    dock.classList.add('is-closed');
    panel.classList.add('dash-chat-closed');
  }
  try {
    localStorage.setItem(DASH_CHAT_STATE_KEY, normalized);
  } catch (_error) {
    // ignore
  }
}

function wireChatDock() {
  const dock = document.getElementById('dashChatDock');
  if (!dock) return;
  dock.addEventListener('click', (event) => {
    const button = event.target.closest('[data-chat-action]');
    if (!button) return;
    event.preventDefault();
    const action = button.getAttribute('data-chat-action');
    if (action === 'open') {
      setChatDockState('open');
      return;
    }
    if (action === 'minimize') {
      setChatDockState('minimized');
      return;
    }
    if (action === 'close') {
      setChatDockState('closed');
      return;
    }
  });
  if (!document.getElementById('dashChatPanel')) {
    return;
  }
  setChatDockState(getChatDockState());
}

function wireModuleControls() {
  const workspace = getWorkspaceRoot();
  if (!workspace) return;
  const menu = ensureModuleContextMenu();
  menu.addEventListener('click', (event) => {
    const button = event.target.closest('[data-module-context-action]');
    if (!button) return;
    const action = button.getAttribute('data-module-context-action');
    const key = button.getAttribute('data-module-context-key');
    const module = getModuleByKey(key);
    if (!module) {
      hideModuleContextMenu();
      return;
    }
    event.preventDefault();
    applyModuleContextAction(module, action).finally(() => {
      hideModuleContextMenu();
      persistWorkspaceState();
      refreshDockButtonState();
    });
  });

  workspace.addEventListener('contextmenu', async (event) => {
    const module = event.target.closest('.dash-module');
    if (!module) return;
    if (isInteractiveSurfaceTarget(event.target)) {
      return;
    }
    event.preventDefault();
    focusModule(module);
    const key = getModuleKeyFromElement(module);
    const minimized = module.classList.contains('dash-module-minimized');
    const maximized = currentMaximizedModule() === key;
    const items = [
      {
        action: 'open',
        icon: '🡹',
        label: module.classList.contains('dash-module-minimized') ? 'Restore' : 'Open',
      },
      { action: 'refresh', icon: '↻', label: 'Refresh content' },
      { action: minimized ? 'open' : 'minimize', icon: minimized ? '📌' : '▾', label: minimized ? 'Unminimize' : 'Minimize' },
      { action: 'maximize', icon: maximized ? '⤢' : '▣', label: maximized ? 'Restore layout' : 'Maximize' },
    ];
    const entryHtml = items
      .map(
        (entry) =>
          `<button type="button" role="menuitem" data-module-context-key="${key}" data-module-context-action="${entry.action}" aria-label="${entry.label}"><span>${entry.icon}</span>${entry.label}</button>`,
      )
      .join('');
    menu.innerHTML = entryHtml;
    menu.setAttribute('aria-hidden', 'false');
    menu.classList.add('is-open');
    const menuPosition = clampMenuPosition(event);
    menu.style.left = `${menuPosition.left}px`;
    menu.style.top = `${menuPosition.top}px`;
  });

  addEventListener('click', (event) => {
    if (!menu.classList.contains('is-open')) {
      return;
    }
    const isMenu = event.target.closest('#dashModuleContextMenu');
    if (isMenu) {
      return;
    }
    hideModuleContextMenu();
  });

  addEventListener('keydown', (event) => {
    if (event.key === 'Escape') {
      hideModuleContextMenu();
    }
  });
  addEventListener('scroll', hideModuleContextMenu);
  addEventListener('resize', hideModuleContextMenu);
}

function wireWorkspaceDock() {
  const dock = document.getElementById('dashDock');
  if (!dock) return;
  const buttons = [...dock.querySelectorAll('.dash-dock-button')];

  function resetDockIcons() {
    for (const button of buttons) {
      button.style.setProperty('--dash-icon-scale', '1');
      button.style.setProperty('--dash-icon-dy', '0px');
    }
  }

  dock.addEventListener('click', async (event) => {
    const button = event.target.closest('[data-dock-module]');
    if (!button) return;
    event.preventDefault();
    const key = button.getAttribute('data-dock-module');
    const surface = getSurfaceForKey(key);
    const module = getModuleByKey(key);
    if (!surface || !module) {
      return;
    }
    if (module.classList.contains('dash-module-minimized')) {
      setModuleMinimized(module, false);
      focusModule(module);
      await hydrateSurface(surface, surface.pathPrefix);
      return;
    }
    const maximized = currentMaximizedModule();
    if (maximized === key) {
      setMaximizedModule('');
    } else {
      setMaximizedModule(key);
    }
    focusModule(module);
    module.scrollIntoView({ behavior: 'smooth', block: 'start' });
    await hydrateSurface(surface, surface.pathPrefix);
  });

  dock.addEventListener('pointermove', (event) => {
    if (buttons.length === 0) {
      return;
    }
    const dockRect = dock.getBoundingClientRect();
    const cursorX = event.clientX;
    for (const dockButton of buttons) {
      const buttonRect = dockButton.getBoundingClientRect();
      const centerX = buttonRect.left + buttonRect.width / 2;
      const distance = Math.abs(cursorX - centerX);
      const radius = Math.max(dockRect.width / 2, 180);
      const intensity = Math.max(0, 1 - distance / radius);
      const scale = 1 + (intensity * 0.46);
      const up = (intensity * 10) - 2;
      dockButton.style.setProperty('--dash-icon-scale', `${scale}`);
      dockButton.style.setProperty('--dash-icon-dy', `${up * -1}px`);
    }
  });

  dock.addEventListener('pointerleave', resetDockIcons);
  if (window.matchMedia && window.matchMedia('(hover: none)').matches) {
    resetDockIcons();
  }
}

function wireModuleDragReorder() {
  const workspace = getWorkspaceRoot();
  if (!workspace) return;
  let dragging = null;
  let pointerId = null;
  let pointerStartX = 0;
  let pointerStartY = 0;
  let startLeft = 0;
  let startTop = 0;

  const onPointerMove = (event) => {
    if (!dragging || event.pointerId !== pointerId) {
      return;
    }
    const moduleWidth = dragging.clientWidth;
    const moduleHeight = dragging.clientHeight;
    const left = startLeft + (event.clientX - pointerStartX);
    const top = startTop + (event.clientY - pointerStartY);
    const clamped = clampModuleGeometry(left, top, moduleWidth, moduleHeight, workspace);
    if (!clamped) return;
    dragging.style.left = `${clamped.left}px`;
    dragging.style.top = `${clamped.top}px`;
  };

  const stopDragging = (event) => {
    if (!dragging || event.pointerId !== pointerId) {
      return;
    }
    dragging.classList.remove('dash-module-dragging');
    dragging.classList.remove('is-dragging');
    try {
      dragging.releasePointerCapture(pointerId);
    } catch (_error) {
      // ignore
    }
    dragging = null;
    pointerId = null;
    removeEventListener('pointermove', onPointerMove);
    removeEventListener('pointerup', stopDragging);
    removeEventListener('pointercancel', stopDragging);
    persistWorkspaceState();
  };

  const onPointerDown = (event) => {
    const module = event.target.closest('.dash-module');
    if (!module || event.button !== 0 || isInteractiveSurfaceTarget(event.target) || currentMaximizedModule()) {
      if (module) {
        focusModule(module);
      }
      return;
    }
    event.preventDefault();
    dragging = module;
    pointerId = event.pointerId;
    pointerStartX = event.clientX;
    pointerStartY = event.clientY;
    startLeft = module.offsetLeft;
    startTop = module.offsetTop;
    dragging.classList.add('dash-module-dragging');
    dragging.classList.add('is-dragging');
    focusModule(dragging);
    try {
      dragging.setPointerCapture(pointerId);
    } catch (_error) {
      // ignore
    }
    addEventListener('pointermove', onPointerMove);
    addEventListener('pointerup', stopDragging);
    addEventListener('pointercancel', stopDragging);
  };

  const observer = new ResizeObserver(() => {
    if (!dragging) {
      persistWorkspaceState();
    }
  });

  for (const module of getWorkspaceModules()) {
    module.addEventListener('pointerdown', onPointerDown);
    observer.observe(module);
  }
  addEventListener('resize', () => {
    for (const module of getWorkspaceModules()) {
      const geometry = getModuleGeometry(module);
      if (geometry) {
        applyModuleGeometry(module, geometry);
      }
    }
    persistWorkspaceState();
  });
}

addEventListener('click', (event) => {
  const refresh = event.target.closest('[data-status-refresh]');
  if (!refresh) return;
  event.preventDefault();
  refreshDashStatus({ manual: true });
});

async function initDashboard() {
  initWorkspaceState();
  bindSurfaceLinks();
  if (typeof wireModuleControls === 'function') {
    wireModuleControls();
  }
  if (typeof wireWorkspaceDock === 'function') {
    wireWorkspaceDock();
  }
  if (typeof wireModuleDragReorder === 'function') {
    wireModuleDragReorder();
  }
  if (typeof wireChatDock === 'function') {
    wireChatDock();
  }
  await hydrateAll();
  await refreshChatLog();
  wireGlobalChat();
  refreshDashStatus();
  setInterval(refreshDashStatus, 5000);
  setInterval(refreshChatLog, 8000);
}

if (document.readyState === 'loading') {
  addEventListener('DOMContentLoaded', initDashboard);
} else {
  initDashboard();
}
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

fn acme_challenge_method(spec: &SmtpServerSpec) -> io::Result<ChallengeType> {
    let value = spec
        .acme_challenge
        .as_deref()
        .unwrap_or("dns-01")
        .to_ascii_lowercase();
    match value.as_str() {
        "http" | "http-01" => Ok(ChallengeType::Http01),
        "dns" | "dns-01" => Ok(ChallengeType::Dns01),
        other => Err(invalid_config(format!(
            "unsupported ACME challenge type: {other}"
        ))),
    }
}

fn acme_challenge_name(method: &ChallengeType) -> &str {
    match method {
        ChallengeType::Http01 => "http-01",
        ChallengeType::Dns01 => "dns-01",
        ChallengeType::TlsAlpn01 => "tls-alpn-01",
        ChallengeType::Other(value) => value.as_str(),
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
    let http_challenges = if challenge_method == ChallengeType::Http01 {
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
        let challenge = authorization
            .challenges
            .as_deref()
            .and_then(|challenges| {
                challenges
                    .iter()
                    .find(|challenge| challenge.challenge_type == challenge_method)
            })
            .ok_or_else(|| {
                invalid_config(format!(
                    "ACME authorization has no {} challenge",
                    acme_challenge_name(&challenge_method)
                ))
            })?;
        let token = challenge.token.as_deref().ok_or_else(|| {
            invalid_config(format!(
                "ACME {} challenge missing token",
                acme_challenge_name(&challenge_method)
            ))
        })?;
        match challenge_method {
            ChallengeType::Http01 => {
                let Some((handler, _shutdown)) = &http_challenges else {
                    return Err(invalid_config("ACME HTTP-01 challenge server not running"));
                };
                handler.add_challenge(token);
            }
            ChallengeType::Dns01 => {
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
            _ => return Err(invalid_config("unsupported ACME challenge type")),
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
