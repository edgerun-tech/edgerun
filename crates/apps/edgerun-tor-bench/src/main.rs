use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

mod tor;

const CELL_SIZE: usize = 512;
const DEFAULT_PORT: u16 = 7890;
const TIMEOUT: Duration = Duration::from_secs(10);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage:");
        eprintln!("  edgerun-tor-bench server [--port PORT]");
        eprintln!("  edgerun-tor-bench client [--addr HOST:PORT] [--count N] [--parallel N]");
        eprintln!(
            "                       [--size N] [--tor] [--onion KEY_HEX] [--local-relay HOST:PORT]"
        );
        eprintln!("  edgerun-tor-bench keygen [--seed HEX]");
        eprintln!(
            "  edgerun-tor-bench service create --dir DIR [--target HOST:PORT] [--virtual-port PORT]"
        );
        eprintln!("                                  [--seed HEX] [--force]");
        std::process::exit(1);
    }

    match args[1].as_str() {
        "server" => run_server(&args[2..]),
        "client" => run_client(&args[2..]),
        "keygen" => run_keygen(&args[2..]),
        "service" => run_service(&args[2..]),
        _ => {
            eprintln!("Unknown subcommand: {}", args[1]);
            std::process::exit(1);
        }
    }
}

fn parse_args(args: &[String]) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let mut i = 0;
    while i < args.len() {
        if args[i].starts_with("--") {
            let key = args[i].trim_start_matches("--").to_string();
            if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                map.insert(key, args[i + 1].clone());
                i += 2;
            } else {
                map.insert(key, "true".to_string());
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    map
}

// ─── Server ────────────────────────────────────────────────────────────────

fn run_server(args: &[String]) {
    let port: u16 = parse_args(args)
        .get("port")
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);

    let addr = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(&addr).expect("failed to bind");
    eprintln!("Server listening on {addr}");

    let total = Arc::new(AtomicU64::new(0));
    let running = Arc::new(AtomicBool::new(true));

    let total_r = total.clone();
    let running_r = running.clone();
    std::thread::spawn(move || {
        while running_r.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_secs(1));
            let count = total_r.swap(0, Ordering::Relaxed);
            if count > 0 {
                eprintln!("  {count} cells/s");
            }
        }
    });

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let total = total.clone();
                std::thread::spawn(move || {
                    let mut buf = vec![0u8; CELL_SIZE];
                    loop {
                        match read_exact(&mut stream, &mut buf) {
                            Ok(()) => {}
                            Err(_) => break,
                        }
                        if let Err(e) = stream.write_all(&buf) {
                            eprintln!("write error: {e}");
                            break;
                        }
                        total.fetch_add(1, Ordering::Relaxed);
                    }
                });
            }
            Err(e) => {
                eprintln!("accept error: {e}");
            }
        }
    }
    running.store(false, Ordering::Relaxed);
}

// ─── Key generation ────────────────────────────────────────────────────────

fn run_keygen(args: &[String]) {
    let opts = parse_args(args);

    let key: edgerun_crypto::Ed25519SigningKey = if let Some(seed_hex) = opts.get("seed") {
        let seed = hex_decode(seed_hex).expect("invalid --seed hex");
        if seed.len() != 32 {
            eprintln!("--seed must be 32 bytes (64 hex chars)");
            std::process::exit(1);
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&seed);
        edgerun_crypto::Ed25519SigningKey::from_bytes(&arr)
    } else {
        edgerun_crypto::random_ed25519_key()
    };

    let pubkey = key.verifying_key().to_bytes();
    let onion = derive_onion_v3(&pubkey);

    println!("=== Tor v3 Onion Service Key ===");
    println!("Private key (hex): {}", hex_encode(&key.to_bytes()));
    println!("Public key  (hex): {}", hex_encode(&pubkey));
    println!("Onion address:     {}.onion", onion);
    println!();
    println!("To create a local service identity without Tor tools:");
    println!(
        "  edgerun-tor-bench service create --dir ./bench.onion --seed {} --target 127.0.0.1:{DEFAULT_PORT}",
        hex_encode(&key.to_bytes())
    );
}

fn derive_onion_v3(pubkey: &[u8; 32]) -> String {
    let prefix = b".onion checksum";
    let version: u8 = 0x03;
    let mut preimage = Vec::with_capacity(prefix.len() + 32 + 1);
    preimage.extend_from_slice(prefix);
    preimage.extend_from_slice(pubkey);
    preimage.push(version);
    let hash = sha3_256(&preimage);
    let checksum = [hash[0], hash[1]];
    let mut addr_bytes = Vec::with_capacity(35);
    addr_bytes.extend_from_slice(pubkey);
    addr_bytes.extend_from_slice(&checksum);
    addr_bytes.push(version);
    base32_encode(&addr_bytes).to_ascii_lowercase()
}

// ─── Onion Service Creation ────────────────────────────────────────────────

const HS_SECRET_KEY_MAGIC: &[u8; 32] = b"== ed25519v1-secret: type0 ==\0\0\0";
const HS_PUBLIC_KEY_MAGIC: &[u8; 32] = b"== ed25519v1-public: type0 ==\0\0\0";

fn run_service(args: &[String]) {
    if args.first().map(|s| s.as_str()) != Some("create") {
        eprintln!("Usage:");
        eprintln!(
            "  edgerun-tor-bench service create --dir DIR [--target HOST:PORT] [--virtual-port PORT]"
        );
        eprintln!("                                  [--seed HEX] [--force]");
        std::process::exit(1);
    }

    let opts = parse_args(&args[1..]);
    let Some(dir) = opts.get("dir").map(PathBuf::from) else {
        eprintln!("service create requires --dir DIR");
        std::process::exit(1);
    };
    let target = opts
        .get("target")
        .cloned()
        .unwrap_or_else(|| format!("127.0.0.1:{DEFAULT_PORT}"));
    let virtual_port = opts
        .get("virtual-port")
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(80);
    let force = opts.contains_key("force");

    let key = match service_key_from_opts(&opts) {
        Ok(key) => key,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    match create_hidden_service(&dir, &key, virtual_port, &target, force) {
        Ok(service) => {
            println!("Created onion service identity");
            println!("Directory:        {}", service.dir.display());
            println!("Onion address:    {}", service.hostname);
            println!("Private key hex:  {}", hex_encode(&service.seed));
            println!(
                "Target:           {} -> {}",
                service.virtual_port, service.target
            );
        }
        Err(e) => {
            eprintln!("service create failed: {e}");
            std::process::exit(1);
        }
    }
}

fn service_key_from_opts(
    opts: &std::collections::HashMap<String, String>,
) -> Result<edgerun_crypto::Ed25519SigningKey, String> {
    if let Some(seed_hex) = opts.get("seed") {
        let seed = hex_decode(seed_hex).map_err(|e| format!("invalid --seed hex: {e}"))?;
        if seed.len() != 32 {
            return Err("--seed must be 32 bytes (64 hex chars)".to_string());
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&seed);
        Ok(edgerun_crypto::Ed25519SigningKey::from_bytes(&arr))
    } else {
        Ok(edgerun_crypto::random_ed25519_key())
    }
}

#[derive(Debug)]
struct CreatedHiddenService {
    dir: PathBuf,
    hostname: String,
    seed: [u8; 32],
    virtual_port: u16,
    target: String,
}

fn create_hidden_service(
    dir: &Path,
    key: &edgerun_crypto::Ed25519SigningKey,
    virtual_port: u16,
    target: &str,
    force: bool,
) -> Result<CreatedHiddenService, String> {
    let seed = key.to_bytes();
    let public_key = key.verifying_key().to_bytes();
    let hostname = format!("{}.onion", derive_onion_v3(&public_key));

    std::fs::create_dir_all(dir).map_err(|e| format!("create dir {}: {e}", dir.display()))?;

    write_new_file(&dir.join("hostname"), hostname.as_bytes(), false, force)?;
    write_new_file(
        &dir.join("hs_ed25519_public_key"),
        &tor_public_key_file(&public_key),
        false,
        force,
    )?;
    write_new_file(
        &dir.join("hs_ed25519_secret_key"),
        &tor_secret_key_file(&seed),
        true,
        force,
    )?;
    write_new_file(
        &dir.join("edgerun-service.toml"),
        format!("virtual_port = {virtual_port}\ntarget = \"{target}\"\n").as_bytes(),
        false,
        force,
    )?;

    Ok(CreatedHiddenService {
        dir: dir.to_path_buf(),
        hostname,
        seed,
        virtual_port,
        target: target.to_string(),
    })
}

fn write_new_file(path: &Path, data: &[u8], secret: bool, force: bool) -> Result<(), String> {
    if path.exists() && !force {
        return Err(format!(
            "{} already exists; pass --force to overwrite",
            path.display()
        ));
    }
    std::fs::write(path, data).map_err(|e| format!("write {}: {e}", path.display()))?;
    #[cfg(unix)]
    if secret {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("chmod {}: {e}", path.display()))?;
    }
    Ok(())
}

fn tor_public_key_file(public_key: &[u8; 32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    out.extend_from_slice(HS_PUBLIC_KEY_MAGIC);
    out.extend_from_slice(public_key);
    out
}

fn tor_secret_key_file(seed: &[u8; 32]) -> Vec<u8> {
    let expanded = expanded_ed25519_secret(seed);
    let mut out = Vec::with_capacity(96);
    out.extend_from_slice(HS_SECRET_KEY_MAGIC);
    out.extend_from_slice(&expanded);
    out
}

fn expanded_ed25519_secret(seed: &[u8; 32]) -> [u8; 64] {
    let mut expanded = edgerun_crypto::sha512(seed);
    expanded[0] &= 248;
    expanded[31] &= 63;
    expanded[31] |= 64;
    expanded
}

fn sha3_256(data: &[u8]) -> [u8; 32] {
    const RC: [u64; 24] = [
        0x0000000000000001,
        0x0000000000008082,
        0x800000000000808A,
        0x8000000080008000,
        0x000000000000808B,
        0x0000000080000001,
        0x8000000080008081,
        0x8000000000008009,
        0x000000000000008A,
        0x0000000000000088,
        0x0000000080008009,
        0x000000008000000A,
        0x000000008000808B,
        0x800000000000008B,
        0x8000000000008089,
        0x8000000000008003,
        0x8000000000008002,
        0x8000000000000080,
        0x000000000000800A,
        0x800000008000000A,
        0x8000000080008081,
        0x8000000000008080,
        0x0000000080000001,
        0x8000000080008008,
    ];
    let rate = 136;
    let mut st = [0u64; 25];
    let mut off = 0;
    while off + rate <= data.len() {
        for i in 0..rate / 8 {
            st[i] ^= u64::from_le_bytes(data[off + i * 8..off + (i + 1) * 8].try_into().unwrap());
        }
        keccak_f1600(&mut st, &RC);
        off += rate;
    }
    let mut block = vec![0u8; rate];
    let rem = data.len() - off;
    block[..rem].copy_from_slice(&data[off..]);
    block[rem] = 0x06;
    block[rate - 1] |= 0x80;
    for i in 0..rate / 8 {
        st[i] ^= u64::from_le_bytes(block[i * 8..(i + 1) * 8].try_into().unwrap());
    }
    keccak_f1600(&mut st, &RC);
    let mut out = [0u8; 32];
    for i in 0..4 {
        out[i * 8..(i + 1) * 8].copy_from_slice(&st[i].to_le_bytes());
    }
    out
}

fn keccak_f1600(st: &mut [u64; 25], rc: &[u64; 24]) {
    for r in 0..24 {
        let c = [
            st[0] ^ st[5] ^ st[10] ^ st[15] ^ st[20],
            st[1] ^ st[6] ^ st[11] ^ st[16] ^ st[21],
            st[2] ^ st[7] ^ st[12] ^ st[17] ^ st[22],
            st[3] ^ st[8] ^ st[13] ^ st[18] ^ st[23],
            st[4] ^ st[9] ^ st[14] ^ st[19] ^ st[24],
        ];
        let d = [
            c[4] ^ c[1].rotate_left(1),
            c[0] ^ c[2].rotate_left(1),
            c[1] ^ c[3].rotate_left(1),
            c[2] ^ c[4].rotate_left(1),
            c[3] ^ c[0].rotate_left(1),
        ];
        for y in 0..5 {
            for x in 0..5 {
                st[y * 5 + x] ^= d[x];
            }
        }
        let mut cur = st[1];
        let mut x = 1;
        let mut y = 0;
        for t in 0..24 {
            let off = ((t + 1) * (t + 2) / 2) % 64;
            let temp = st[((2 + 3 * y) % 5) * 5 + ((0 + 2 * x) % 5)];
            st[((2 + 3 * y) % 5) * 5 + ((0 + 2 * x) % 5)] = cur.rotate_left(off as u32);
            cur = temp;
            let nx = y;
            let ny = (2 * x + 3 * y) % 5;
            x = nx;
            y = ny;
        }
        for y in 0..5 {
            let r = y * 5;
            let t = [st[r], st[r + 1], st[r + 2], st[r + 3], st[r + 4]];
            st[r] = t[0] ^ (!t[1] & t[2]);
            st[r + 1] = t[1] ^ (!t[2] & t[3]);
            st[r + 2] = t[2] ^ (!t[3] & t[4]);
            st[r + 3] = t[3] ^ (!t[4] & t[0]);
            st[r + 4] = t[4] ^ (!t[0] & t[1]);
        }
        st[0] ^= rc[r];
    }
}

fn base32_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let out_len = (data.len() * 8 + 4) / 5;
    let mut out = String::with_capacity(out_len);
    let mut bits: u64 = 0;
    let mut bit_len: u32 = 0;
    for &b in data {
        bits = (bits << 8) | (b as u64);
        bit_len += 8;
        while bit_len >= 5 {
            bit_len -= 5;
            let idx = (bits >> bit_len) as usize & 0x1F;
            out.push(ALPHABET[idx] as char);
        }
    }
    if bit_len > 0 {
        let idx = (bits << (5 - bit_len)) as usize & 0x1F;
        out.push(ALPHABET[idx] as char);
    }
    out
}

fn hex_encode(data: &[u8]) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut out = String::with_capacity(data.len() * 2);
    for &b in data {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0F) as usize] as char);
    }
    out
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 {
        return Err("hex length".to_string());
    }
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len() / 2);
    for c in b.chunks(2) {
        let hi = (c[0] as char).to_digit(16).ok_or("bad hex")? as u8;
        let lo = (c[1] as char).to_digit(16).ok_or("bad hex")? as u8;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

// ─── Client Benchmark ──────────────────────────────────────────────────────

fn run_client(args: &[String]) {
    let opts = parse_args(args);
    let addr = opts
        .get("addr")
        .cloned()
        .unwrap_or_else(|| format!("127.0.0.1:{DEFAULT_PORT}"));
    let count: usize = opts
        .get("count")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000);
    let parallel: usize = opts
        .get("parallel")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let cell_size: usize = opts
        .get("size")
        .and_then(|v| v.parse().ok())
        .unwrap_or(CELL_SIZE);
    let use_tor = opts.contains_key("tor");
    let onion_key = opts.get("onion").cloned();
    let local_relay = opts.get("local-relay").cloned();

    if use_tor {
        if let Some(lr) = local_relay {
            eprintln!("Tor mode: connecting to local relay {lr}");
            run_tor_local_benchmark(&addr, &lr, count, cell_size);
        } else {
            eprintln!("Tor mode: connecting through Tor network");
            run_tor_benchmark(&addr, count, parallel, cell_size, onion_key);
        }
    } else {
        eprintln!("Direct mode: connecting directly to {addr}");
        eprintln!("Benchmark: {count} cells, {parallel} parallel, {cell_size}B each");
        eprintln!();
        run_direct_benchmark(&addr, count, parallel, cell_size);
    }
}

fn run_tor_local_benchmark(addr: &str, relay: &str, count: usize, cell_size: usize) {
    use std::sync::Mutex;

    let (rhost, rport) = split_host_port(relay, 19001);
    let mut session = tor::TorSession::new();

    eprintln!("  connecting to local relay {rhost}:{rport}...");
    let mut circ = match tor::connect_relay_direct(rhost, rport, &mut session) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("circuit direct failed: {e}");
            return;
        }
    };

    // Open TCP stream through circuit
    let (host, port) = split_host_port(addr, DEFAULT_PORT);
    eprintln!("  opening stream to {host}:{port}...");
    match tor::tor_relay_begin(&mut session, &mut circ, host, port) {
        Ok(sid) => eprintln!("  stream {sid} opened"),
        Err(e) => {
            eprintln!("stream failed: {e}");
            return;
        }
    }

    let cell = vec![0x42u8; cell_size];
    eprintln!();
    eprintln!("Benchmark: {count} cells, {cell_size}B each, through local Tor relay");
    eprintln!();

    let start = Instant::now();
    let mut failures = 0usize;
    let mut latencies = Vec::new();

    for i in 0..count {
        let t0 = Instant::now();
        if let Err(e) = tor::tor_send_data(&mut session, &mut circ, 1, &cell) {
            failures += 1;
            continue;
        }
        match tor::tor_recv_data(&mut session, &mut circ, 1, cell_size) {
            Ok(reply) => {
                if reply == cell {
                    latencies.push(t0.elapsed().as_secs_f64() * 1_000_000.0);
                } else {
                    failures += 1;
                }
            }
            Err(_) => {
                failures += 1;
            }
        }
        let pct = ((i + 1) as f64 / count as f64 * 100.0) as u32;
        if (i + 1) % (count / 10).max(1) == 0 {
            eprintln!("  {pct}% complete ({}/{count})", i + 1);
        }
    }

    let elapsed = start.elapsed();
    print_tor_results(&latencies, failures, cell_size, elapsed);
}

fn run_direct_benchmark(addr: &str, count: usize, parallel: usize, cell_size: usize) {
    use std::sync::Mutex;
    let stats = Arc::new(Mutex::new(BenchStats::default()));
    let start = Instant::now();

    eprint!("Warmup... ");
    for _ in 0..10 {
        let _ = ping_pong(addr, cell_size);
    }
    eprintln!("done");

    run_workers(addr, count, parallel, cell_size, None, stats, start);
}

fn run_tor_benchmark(
    addr: &str,
    count: usize,
    parallel: usize,
    cell_size: usize,
    onion_key: Option<String>,
) {
    use std::sync::Mutex;

    eprintln!("  fetching Tor consensus...");
    let relays = match tor::fetch_relays() {
        Ok(r) => {
            eprintln!("  got {} relays", r.len());
            r
        }
        Err(e) => {
            eprintln!("  consensus failed: {e}");
            return;
        }
    };

    let mut session = tor::TorSession::new();

    let cell = vec![0x42u8; cell_size];
    let use_onion = onion_key.is_some();

    // Build circuit
    let circ = if use_onion {
        let key = onion_key.as_ref().unwrap();
        let seed = hex_decode(key).expect("invalid --onion key hex");
        if seed.len() != 32 {
            eprintln!("--onion key must be 32 bytes");
            return;
        }
        let mut hsv = [0u8; 32];
        hsv.copy_from_slice(&seed);
        eprintln!("  connecting to onion service {addr}...");
        match tor::connect_onion(&mut session, &relays, addr, DEFAULT_PORT, &hsv) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("onion connect failed: {e}");
                return;
            }
        }
    } else {
        eprintln!("  building 3-hop circuit...");
        match tor::build_3hop_circuit(&mut session, &relays) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("circuit failed: {e}");
                return;
            }
        }
    };

    let mut circ = circ; // mutable

    // Open TCP stream through circuit
    if !use_onion {
        let (host, port) = split_host_port(addr, DEFAULT_PORT);
        eprintln!("  opening stream to {host}:{port}...");
        match tor::tor_relay_begin(&mut session, &mut circ, host, port) {
            Ok(sid) => eprintln!("  stream {sid} opened"),
            Err(e) => {
                eprintln!("stream failed: {e}");
                return;
            }
        }
    }

    // Benchmark through Tor circuit
    eprintln!();
    eprintln!("Benchmark: {count} cells, {cell_size}B each, through Tor circuit");
    eprintln!();

    let stats = Arc::new(Mutex::new(BenchStats::default()));
    let start = Instant::now();

    // Since Tor circuits are per-session, we do sequential benchmark
    let mut failures = 0usize;
    let mut latencies = Vec::new();

    for i in 0..count {
        let t0 = Instant::now();

        // Send cell through Tor circuit
        if let Err(e) = tor::tor_send_data(&mut session, &mut circ, 1, &cell) {
            failures += 1;
            continue;
        }

        // Receive echo through Tor circuit
        match tor::tor_recv_data(&mut session, &mut circ, 1, cell_size) {
            Ok(reply) => {
                if reply == cell {
                    latencies.push(t0.elapsed().as_secs_f64() * 1_000_000.0);
                } else {
                    failures += 1;
                }
            }
            Err(_) => {
                failures += 1;
            }
        }

        let pct = ((i + 1) as f64 / count as f64 * 100.0) as u32;
        if (i + 1) % (count / 10).max(1) == 0 {
            eprintln!("  {pct}% complete ({}/{count})", i + 1);
        }
    }

    let elapsed = start.elapsed();
    print_tor_results(&latencies, failures, cell_size, elapsed);
}

fn run_workers(
    addr: &str,
    count: usize,
    parallel: usize,
    cell_size: usize,
    _socks5: Option<String>,
    stats: Arc<std::sync::Mutex<BenchStats>>,
    start: Instant,
) {
    use std::sync::Mutex;
    let wg = Arc::new(std::sync::Barrier::new(parallel + 1));

    for _ in 0..parallel {
        let s = stats.clone();
        let a = addr.to_string();
        let wg = wg.clone();
        std::thread::spawn(move || {
            wg.wait();
            loop {
                let mut st = s.lock().unwrap();
                let done = st.latencies.len() + st.failures;
                if done >= count {
                    break;
                }
                drop(st);

                match ping_pong(&a, cell_size) {
                    Ok(lat) => {
                        let mut st = s.lock().unwrap();
                        if st.latencies.len() + st.failures < count {
                            st.latencies.push(lat);
                            st.bytes_sent += cell_size as u64;
                            st.bytes_rcvd += cell_size as u64;
                        }
                    }
                    Err(_) => {
                        let mut st = s.lock().unwrap();
                        if st.latencies.len() + st.failures < count {
                            st.failures += 1;
                        }
                    }
                }
            }
        });
    }

    wg.wait();
    loop {
        let st = stats.lock().unwrap();
        if st.latencies.len() + st.failures >= count {
            break;
        }
        drop(st);
        std::thread::sleep(Duration::from_millis(1));
    }

    let elapsed = start.elapsed();
    let st = stats.lock().unwrap();
    print_results(
        &st.latencies,
        st.failures,
        cell_size,
        elapsed,
        count,
        parallel,
    );
}

#[derive(Default)]
struct BenchStats {
    latencies: Vec<f64>,
    failures: usize,
    bytes_sent: u64,
    bytes_rcvd: u64,
}

fn print_results(
    latencies: &[f64],
    failures: usize,
    cell_size: usize,
    elapsed: Duration,
    total: usize,
    parallel: usize,
) {
    let mut sorted = latencies.to_vec();
    sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
    let success = sorted.len();

    println!();
    println!("═══ Direct TCP Benchmark Results ═══");
    println!("  Total attempts:   {total}");
    println!("  Successful:       {success}");
    println!(
        "  Failed:           {} ({:.1}%)",
        failures,
        if total > 0 {
            failures as f64 / total as f64 * 100.0
        } else {
            0.0
        }
    );
    println!("  Cell size:        {cell_size} bytes");
    println!("  Parallelism:      {parallel}");
    println!("  Duration:         {:.3}s", elapsed.as_secs_f64());
    println!();

    if !sorted.is_empty() {
        let sum: f64 = sorted.iter().sum();
        let mean = sum / sorted.len() as f64;
        println!("  Latency (μs):");
        println!("    min:   {:>8.0}", sorted[0]);
        println!("    mean:  {:>8.0}", mean);
        println!("    max:   {:>8.0}", sorted[sorted.len() - 1]);
        println!("    p50:   {:>8.0}", percentile(&sorted, 50.0));
        println!("    p95:   {:>8.0}", percentile(&sorted, 95.0));
        println!("    p99:   {:>8.0}", percentile(&sorted, 99.0));
        println!();

        let cells_s = success as f64 / elapsed.as_secs_f64();
        let bw = cell_size as f64 * cells_s / 1024.0;
        println!("  Throughput: {cells_s:>8.0} cells/s, {bw:.2} KiB/s");
    }
}

fn print_tor_results(latencies: &[f64], failures: usize, cell_size: usize, elapsed: Duration) {
    let mut sorted = latencies.to_vec();
    sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
    let success = sorted.len();
    let total = success + failures;

    println!();
    println!("═══ Tor Circuit Benchmark Results ═══");
    println!("  Total attempts:   {total}");
    println!("  Successful:       {success}");
    println!(
        "  Failed:           {} ({:.1}%)",
        failures,
        if total > 0 {
            failures as f64 / total as f64 * 100.0
        } else {
            0.0
        }
    );
    println!("  Cell size:        {cell_size} bytes");
    println!("  Duration:         {:.3}s", elapsed.as_secs_f64());
    println!();

    if !sorted.is_empty() {
        let sum: f64 = sorted.iter().sum();
        let mean = sum / sorted.len() as f64;
        println!("  Latency (ms):");
        println!("    min:   {:>8.1}", sorted[0] / 1000.0);
        println!("    mean:  {:>8.1}", mean / 1000.0);
        println!("    max:   {:>8.1}", sorted[sorted.len() - 1] / 1000.0);
        println!("    p50:   {:>8.1}", percentile(&sorted, 50.0) / 1000.0);
        println!("    p95:   {:>8.1}", percentile(&sorted, 95.0) / 1000.0);
        println!("    p99:   {:>8.1}", percentile(&sorted, 99.0) / 1000.0);
        println!();

        let cells_s = success as f64 / elapsed.as_secs_f64();
        println!("  Throughput: {cells_s:>8.2} cells/s");
    }
}

fn ping_pong(addr: &str, cell_size: usize) -> Result<f64, String> {
    let mut stream = TcpStream::connect(addr).map_err(|e| format!("connect: {e}"))?;
    stream
        .set_read_timeout(Some(TIMEOUT))
        .map_err(|e| format!("timeout: {e}"))?;

    let cell = vec![0x42u8; cell_size];
    let start = Instant::now();
    stream.write_all(&cell).map_err(|e| format!("write: {e}"))?;

    let mut reply = vec![0u8; cell_size];
    read_exact(&mut stream, &mut reply).map_err(|e| format!("read: {e}"))?;

    let latency = start.elapsed().as_secs_f64() * 1_000_000.0;
    if reply != cell {
        return Err("mismatch".to_string());
    }
    Ok(latency)
}

fn split_host_port(target: &str, default_port: u16) -> (&str, u16) {
    if let Some(idx) = target.rfind(':') {
        let host = &target[..idx];
        if let Ok(port) = target[idx + 1..].parse::<u16>() {
            return (host, port);
        }
    }
    (target, default_port)
}

fn read_exact(stream: &mut TcpStream, buf: &mut [u8]) -> Result<(), String> {
    let mut off = 0;
    while off < buf.len() {
        match stream.read(&mut buf[off..]) {
            Ok(0) => return Err("eof".to_string()),
            Ok(n) => off += n,
            Err(e) => return Err(format!("read: {e}")),
        }
    }
    Ok(())
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (p / 100.0 * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("edgerun-tor-bench-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn creates_tor_v3_hidden_service_files() {
        let dir = test_dir("create-service");
        let seed = [7u8; 32];
        let key = edgerun_crypto::Ed25519SigningKey::from_bytes(&seed);

        let service = create_hidden_service(&dir, &key, 80, "127.0.0.1:7890", false).unwrap();

        let hostname = std::fs::read_to_string(dir.join("hostname")).unwrap();
        assert_eq!(hostname, service.hostname);
        assert!(hostname.ends_with(".onion"));
        assert!(hostname.chars().all(|c| !c.is_ascii_uppercase()));

        let public_key = std::fs::read(dir.join("hs_ed25519_public_key")).unwrap();
        assert_eq!(public_key.len(), 64);
        assert_eq!(&public_key[..32], HS_PUBLIC_KEY_MAGIC);
        assert_eq!(&public_key[32..], key.verifying_key().to_bytes().as_slice());

        let secret_key = std::fs::read(dir.join("hs_ed25519_secret_key")).unwrap();
        assert_eq!(secret_key.len(), 96);
        assert_eq!(&secret_key[..32], HS_SECRET_KEY_MAGIC);
        assert_eq!(&secret_key[32..], expanded_ed25519_secret(&seed).as_slice());

        let metadata = std::fs::read_to_string(dir.join("edgerun-service.toml")).unwrap();
        assert!(metadata.contains("virtual_port = 80"));
        assert!(metadata.contains("target = \"127.0.0.1:7890\""));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hidden_service_create_refuses_overwrite_without_force() {
        let dir = test_dir("overwrite-service");
        let key = edgerun_crypto::Ed25519SigningKey::from_bytes(&[8u8; 32]);

        create_hidden_service(&dir, &key, 80, "127.0.0.1:7890", false).unwrap();
        let err = create_hidden_service(&dir, &key, 80, "127.0.0.1:7890", false).unwrap_err();

        assert!(err.contains("already exists"));
        create_hidden_service(&dir, &key, 80, "127.0.0.1:7890", true).unwrap();

        let _ = std::fs::remove_dir_all(&dir);
    }
}
