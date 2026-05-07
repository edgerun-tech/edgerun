use std::io::Write;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(all(
    feature = "http",
    feature = "https",
    feature = "dns",
    feature = "dhcp",
    feature = "smtp",
    feature = "imap",
    feature = "proxy",
    feature = "derived-db",
    feature = "quic",
    feature = "acme",
))]
use edgerun_http::{into_handler, Response, StatusCode};
#[cfg(all(
    feature = "http",
    feature = "https",
    feature = "dns",
    feature = "dhcp",
    feature = "smtp",
    feature = "imap",
    feature = "proxy",
    feature = "derived-db",
    feature = "quic",
    feature = "acme",
))]
use edgerun_node::rt::{CancellationToken, Runtime};

pub fn cmd_bind_check(standard_ports: bool) {
    match run_bind_check(standard_ports) {
        Ok(report) => println!("{report}"),
        Err(error) => {
            eprintln!("bind-check failed: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(all(
    feature = "http",
    feature = "https",
    feature = "dns",
    feature = "dhcp",
    feature = "smtp",
    feature = "imap",
    feature = "proxy",
    feature = "derived-db",
    feature = "quic",
    feature = "acme",
))]
fn run_bind_check(standard_ports: bool) -> Result<String, String> {
    let rt = Runtime::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("runtime build: {e}"))?;
    rt.block_on(run_bind_check_async(standard_ports))
}

#[cfg(not(all(
    feature = "http",
    feature = "https",
    feature = "dns",
    feature = "dhcp",
    feature = "smtp",
    feature = "imap",
    feature = "proxy",
    feature = "derived-db",
    feature = "quic",
    feature = "acme",
)))]
fn run_bind_check(_standard_ports: bool) -> Result<String, String> {
    Err("bind-check requires features: display,dns,dhcp,http,https,acme,quic,proxy,derived-db,smtp,imap".into())
}

#[cfg(all(
    feature = "http",
    feature = "https",
    feature = "dns",
    feature = "dhcp",
    feature = "smtp",
    feature = "imap",
    feature = "proxy",
    feature = "derived-db",
    feature = "quic",
    feature = "acme",
))]
async fn run_bind_check_async(standard_ports: bool) -> Result<String, String> {
    let ports = BindPorts::new(standard_ports);

    let derived_db_path = derived_db_probe_path();
    open_derived_db_probe(&derived_db_path)?;

    let tls_cert = edgerun_tls::generate_self_signed(&["localhost", "127.0.0.1"])
        .map_err(|e| format!("tls certificate: {e}"))?;

    let dns_config = crate::services::DnsConfig {
        bind_addr: ports.dns.to_string(),
        bind_addr_ipv6: None,
        default_ttl: 60,
        rate_limit_qps: 100,
    };
    let dhcp_config = edgerun_protocols::dhcp::DhcpServerConfig {
        server_ip: Ipv4Addr::new(10, 77, 0, 1),
        subnet_mask: Ipv4Addr::new(255, 255, 255, 0),
        router: Ipv4Addr::new(10, 77, 0, 1),
        dns_servers: vec![Ipv4Addr::new(10, 77, 0, 1)],
        lease_time: 3600,
        tftp_server: None,
        default_bootfile: None,
        bootfile_by_arch: std::collections::BTreeMap::new(),
    };
    let dhcp_server = crate::services::dhcp_runtime::DhcpServer::new_bound(
        dhcp_config,
        Ipv4Addr::new(10, 77, 0, 10),
        Ipv4Addr::new(10, 77, 0, 20),
        edgerun_node::rt::SocketAddr::from_array([127, 0, 0, 1], ports.dhcp_port),
    )
    .map_err(|e| format!("dhcp bind: {e:?}"))?;

    let _server_dhcp_config = crate::services::DhcpConfig::new(
        Ipv4Addr::new(10, 77, 0, 1),
        Ipv4Addr::new(255, 255, 255, 0),
        Ipv4Addr::new(10, 77, 0, 1),
        Ipv4Addr::new(10, 77, 0, 10),
        Ipv4Addr::new(10, 77, 0, 20),
    );
    let mut smtp_config = crate::services::SmtpConfig::default();
    smtp_config.bind_addr = ports.smtp.to_string();
    smtp_config.domain_name = "bind-check.edgerun.local".to_string();
    smtp_config.starttls = true;
    smtp_config.tls_cert = Some(tls_cert.clone());
    smtp_config.local_domains = vec!["bind-check.edgerun.local".to_string()];
    let mut imap_config = crate::services::ImapConfig::default();
    imap_config.bind_addr = ports.imap.to_string();
    imap_config.domain_name = "bind-check.edgerun.local".to_string();
    imap_config.tls_cert = Some(tls_cert.clone());
    let mut proxy_config = crate::services::ProxyConfig::default();
    proxy_config.bind_addr = ports.proxy.to_string();

    let mut bound = crate::services::NodeRuntime::new()
        .with_dns(dns_config)
        .with_smtp(smtp_config)
        .with_imap(imap_config)
        .with_proxy(proxy_config)
        .build()
        .await
        .map_err(|e| format!("server bind: {e}"))?;
    let http_handler =
        into_handler(|_| Response::text(StatusCode::new(200).unwrap(), "edgerund bind-check"));
    let https_handler =
        into_handler(|_| Response::text(StatusCode::new(200).unwrap(), "edgerund bind-check"));
    let http_bound = edgerun_http::server::HttpServer::new(http_handler)
        .bind(ports.http)
        .await
        .map_err(|e| format!("http bind: {e}"))?;
    let https_bound = edgerun_http::server::HttpServer::new(https_handler)
        .with_tls(tls_cert)
        .with_http3()
        .bind(ports.https)
        .await
        .map_err(|e| format!("https bind: {e}"))?;

    let shutdown = CancellationToken::new();
    let run_shutdown = shutdown.clone();
    let server_task = edgerun_node::rt::spawn(async move { bound.run(run_shutdown).await });
    let http_shutdown = shutdown.clone();
    let http_task =
        edgerun_node::rt::spawn(async move { http_bound.serve_with_shutdown(http_shutdown).await });
    let https_shutdown = shutdown.clone();
    let https_task =
        edgerun_node::rt::spawn(
            async move { https_bound.serve_with_shutdown(https_shutdown).await },
        );
    let dhcp_shutdown = shutdown.clone();
    let dhcp_task = edgerun_node::rt::spawn(async move {
        dhcp_server.run(dhcp_shutdown).await;
        Ok::<(), edgerun_protocols::dhcp::message::io::Error>(())
    });
    std::thread::sleep(Duration::from_millis(500));

    let report = format!(
        concat!(
            "{{",
            "\"bind_check\":\"ok\",",
            "\"http\":\"{}\",",
            "\"https\":\"{}\",",
            "\"quic_udp\":\"{}\",",
            "\"dns_tcp\":\"{}\",",
            "\"dns_udp\":\"{}\",",
            "\"dhcp_udp\":\"{}\",",
            "\"smtp\":\"{}\",",
            "\"imap\":\"{}\",",
            "\"proxy\":\"{}\",",
            "\"derived_db\":\"{}\",",
            "\"acme\":\"compiled\"",
            "}}"
        ),
        ports.http,
        ports.https,
        ports.https,
        ports.dns,
        ports.dns,
        ports.dhcp,
        ports.smtp,
        ports.imap,
        ports.proxy,
        escape_json(&derived_db_path.display().to_string()),
    );

    println!("{report}");
    let _ = std::io::stdout().flush();
    std::thread::sleep(Duration::from_secs(15));
    shutdown.cancel();
    drop(server_task);
    drop(http_task);
    drop(https_task);
    drop(dhcp_task);

    std::process::exit(0);
}

struct BindPorts {
    http: &'static str,
    https: &'static str,
    dns: &'static str,
    dhcp: &'static str,
    dhcp_port: u16,
    smtp: &'static str,
    imap: &'static str,
    proxy: &'static str,
}

impl BindPorts {
    fn new(standard_ports: bool) -> Self {
        if standard_ports {
            Self {
                http: "127.0.0.1:80",
                https: "127.0.0.1:443",
                dns: "127.0.0.1:53",
                dhcp: "127.0.0.1:67",
                dhcp_port: 67,
                smtp: "127.0.0.1:25",
                imap: "127.0.0.1:143",
                proxy: "127.0.0.1:8080",
            }
        } else {
            Self {
                http: "127.0.0.1:18080",
                https: "127.0.0.1:18443",
                dns: "127.0.0.1:15353",
                dhcp: "127.0.0.1:11067",
                dhcp_port: 11067,
                smtp: "127.0.0.1:12525",
                imap: "127.0.0.1:11143",
                proxy: "127.0.0.1:18081",
            }
        }
    }
}

#[cfg(all(
    feature = "http",
    feature = "https",
    feature = "dns",
    feature = "dhcp",
    feature = "smtp",
    feature = "imap",
    feature = "proxy",
    feature = "derived-db",
    feature = "quic",
    feature = "acme",
))]
fn open_derived_db_probe(path: &PathBuf) -> Result<(), String> {
    let mut db = edgerun_derived_db::open_file_database(path)
        .map_err(|e| format!("derived db open: {e}"))?;
    db.put_meta(b"bind_check", b"ok", unix_now())
        .map_err(|e| format!("derived db initialize: {e}"))?;
    Ok(())
}

#[cfg(all(
    feature = "http",
    feature = "https",
    feature = "dns",
    feature = "dhcp",
    feature = "smtp",
    feature = "imap",
    feature = "proxy",
    feature = "derived-db",
    feature = "quic",
    feature = "acme",
))]
fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn derived_db_probe_path() -> PathBuf {
    let mut path = std::env::temp_dir();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    path.push(format!(
        "edgerund-bind-check-{}-{now}.edb",
        std::process::id()
    ));
    path
}

fn escape_json(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}
