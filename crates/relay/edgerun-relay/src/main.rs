use edgerun_relay::Relay;
use std::env;
#[cfg(feature = "virtio")]
use std::net::Ipv4Addr;

fn main() {
    let mut args = env::args().skip(1);
    let first = args.next();
    let (transport, addr) = match first.as_deref() {
        Some("tcp" | "udp" | "ws" | "wss" | "virtio-udp" | "virtio-tcp") => (
            first.expect("transport is present"),
            args.next().unwrap_or_else(|| "127.0.0.1:7373".to_owned()),
        ),
        Some(addr) => ("tcp".to_owned(), addr.to_owned()),
        None => ("tcp".to_owned(), "127.0.0.1:7373".to_owned()),
    };

    eprintln!("edgerun-relay listening on {transport}://{addr}");
    let relay = Relay::new();
    let result = match transport.as_str() {
        "tcp" => relay.serve(&addr),
        "udp" => relay.serve_udp(&addr),
        "ws" => relay.serve_ws(&addr),
        "wss" => relay.serve_wss(&addr),
        "virtio-udp" => serve_virtio_udp(&relay, &addr),
        "virtio-tcp" => serve_virtio_tcp(&relay, &addr),
        _ => unreachable!(),
    };
    if let Err(error) = result {
        eprintln!("edgerun-relay failed: {error}");
        std::process::exit(1);
    }
}

#[cfg(feature = "virtio")]
fn serve_virtio_udp(relay: &Relay, spec: &str) -> std::io::Result<()> {
    let config = VirtioUdpConfig::parse(spec)?;
    let net = edgerun_virtio::try_find_initialized_virtio_net()
        .map_err(|error| std::io::Error::other(format!("virtio net init failed: {error:?}")))?;
    relay.serve_virtio_udp(
        net,
        to_edgerun_ip(config.ip),
        to_edgerun_ip(config.netmask),
        to_edgerun_ip(config.gateway),
        config.port,
    )
}

#[cfg(not(feature = "virtio"))]
fn serve_virtio_udp(_relay: &Relay, _spec: &str) -> std::io::Result<()> {
    Err(std::io::Error::other(
        "virtio-udp requires building edgerun-relay with --features virtio",
    ))
}

#[cfg(feature = "virtio")]
fn serve_virtio_tcp(relay: &Relay, spec: &str) -> std::io::Result<()> {
    let config = VirtioUdpConfig::parse(spec)?;
    let net = edgerun_virtio::try_find_initialized_virtio_net()
        .map_err(|error| std::io::Error::other(format!("virtio net init failed: {error:?}")))?;
    relay.serve_virtio_tcp(
        net,
        to_edgerun_ip(config.ip),
        to_edgerun_ip(config.netmask),
        to_edgerun_ip(config.gateway),
        config.port,
    )
}

#[cfg(not(feature = "virtio"))]
fn serve_virtio_tcp(_relay: &Relay, _spec: &str) -> std::io::Result<()> {
    Err(std::io::Error::other(
        "virtio-tcp requires building edgerun-relay with --features virtio",
    ))
}

#[cfg(feature = "virtio")]
struct VirtioUdpConfig {
    ip: Ipv4Addr,
    netmask: Ipv4Addr,
    gateway: Ipv4Addr,
    port: u16,
}

#[cfg(feature = "virtio")]
impl VirtioUdpConfig {
    fn parse(spec: &str) -> std::io::Result<Self> {
        let mut parts = spec.split(',');
        let ip = parse_ipv4(parts.next().unwrap_or("10.0.2.15"), "ip")?;
        let netmask = parse_ipv4(parts.next().unwrap_or("255.255.255.0"), "netmask")?;
        let gateway = parse_ipv4(parts.next().unwrap_or("10.0.2.2"), "gateway")?;
        let port = parts
            .next()
            .unwrap_or("7373")
            .parse::<u16>()
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error))?;
        Ok(Self {
            ip,
            netmask,
            gateway,
            port,
        })
    }
}

#[cfg(feature = "virtio")]
fn parse_ipv4(value: &str, field: &str) -> std::io::Result<Ipv4Addr> {
    value.parse::<Ipv4Addr>().map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid virtio udp {field}: {error}"),
        )
    })
}

#[cfg(feature = "virtio")]
fn to_edgerun_ip(ip: Ipv4Addr) -> edgerun_protocols::ethernet_ipv4::IpAddr {
    let [a, b, c, d] = ip.octets();
    edgerun_protocols::ethernet_ipv4::IpAddr::new(a, b, c, d)
}
