use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
struct Config {
    host: String,
    port: u16,
    count: usize,
    concurrency: usize,
    from: String,
    to: String,
    message_bytes: usize,
    timeout: Duration,
}

fn main() -> Result<(), String> {
    let config = Config::parse(std::env::args().skip(1))?;
    let next = Arc::new(AtomicUsize::new(0));
    let failures = Arc::new(AtomicUsize::new(0));
    let latencies = Arc::new(Mutex::new(Vec::with_capacity(config.count)));
    let started = Instant::now();

    let mut workers = Vec::new();
    for _ in 0..config.concurrency {
        let config = config.clone();
        let next = Arc::clone(&next);
        let failures = Arc::clone(&failures);
        let latencies = Arc::clone(&latencies);
        workers.push(thread::spawn(move || loop {
            let index = next.fetch_add(1, Ordering::Relaxed);
            if index >= config.count {
                break;
            }
            let before = Instant::now();
            match smtp_once(&config, index) {
                Ok(()) => {
                    let elapsed = before.elapsed().as_micros();
                    latencies
                        .lock()
                        .expect("latency mutex poisoned")
                        .push(elapsed);
                }
                Err(_) => {
                    failures.fetch_add(1, Ordering::Relaxed);
                }
            }
        }));
    }

    for worker in workers {
        worker
            .join()
            .map_err(|_| "worker thread panicked".to_string())?;
    }

    let elapsed = started.elapsed();
    let failures = failures.load(Ordering::Relaxed);
    let mut latencies = Arc::try_unwrap(latencies)
        .map_err(|_| "latency references still active".to_string())?
        .into_inner()
        .map_err(|_| "latency mutex poisoned".to_string())?;
    latencies.sort_unstable();

    let ok = latencies.len();
    let elapsed_ms = elapsed.as_millis();
    let throughput = if elapsed.as_secs_f64() > 0.0 {
        ok as f64 / elapsed.as_secs_f64()
    } else {
        0.0
    };
    let avg = if ok == 0 {
        0.0
    } else {
        latencies.iter().sum::<u128>() as f64 / ok as f64 / 1000.0
    };

    println!("mode=smtp");
    println!("target={}:{}", config.host, config.port);
    println!("requested={}", config.count);
    println!("concurrency={}", config.concurrency);
    println!("ok={ok}");
    println!("fail={failures}");
    println!("elapsed_ms={elapsed_ms}");
    println!("throughput_ops_per_sec={throughput:.2}");
    println!("latency_avg_ms={avg:.2}");
    println!("latency_p50_ms={:.3}", percentile_ms(&latencies, 0.50));
    println!("latency_p95_ms={:.3}", percentile_ms(&latencies, 0.95));
    println!("latency_p99_ms={:.3}", percentile_ms(&latencies, 0.99));
    Ok(())
}

impl Config {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut config = Self {
            host: "127.0.0.1".to_string(),
            port: 25,
            count: 10_000,
            concurrency: 16,
            from: "bench@example.test".to_string(),
            to: "bench@example.test".to_string(),
            message_bytes: 1024,
            timeout: Duration::from_secs(10),
        };

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--host" => config.host = next_value(&mut args, "--host")?,
                "--port" => config.port = parse_value(&mut args, "--port")?,
                "--count" => config.count = parse_value(&mut args, "--count")?,
                "--concurrency" => config.concurrency = parse_value(&mut args, "--concurrency")?,
                "--from" => config.from = next_value(&mut args, "--from")?,
                "--to" => config.to = next_value(&mut args, "--to")?,
                "--message-bytes" => {
                    config.message_bytes = parse_value(&mut args, "--message-bytes")?
                }
                "--timeout-ms" => {
                    let millis: u64 = parse_value(&mut args, "--timeout-ms")?;
                    config.timeout = Duration::from_millis(millis);
                }
                "--help" | "-h" => return Err(usage()),
                _ => return Err(format!("unknown argument: {arg}\n{}", usage())),
            }
        }

        if config.count == 0 {
            return Err("--count must be greater than zero".to_string());
        }
        if config.concurrency == 0 {
            return Err("--concurrency must be greater than zero".to_string());
        }
        Ok(config)
    }
}

fn smtp_once(config: &Config, index: usize) -> io::Result<()> {
    let addr = format!("{}:{}", config.host, config.port);
    let socket = addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "address did not resolve"))?;
    let mut stream = TcpStream::connect_timeout(&socket, config.timeout)?;
    stream.set_read_timeout(Some(config.timeout))?;
    stream.set_write_timeout(Some(config.timeout))?;
    let mut reader = BufReader::new(stream.try_clone()?);

    expect_reply(&mut reader, 220)?;
    send_expect(&mut stream, &mut reader, 250, "EHLO benchmark.local\r\n")?;
    send_expect(
        &mut stream,
        &mut reader,
        250,
        &format!("MAIL FROM:<{}>\r\n", config.from),
    )?;
    send_expect(
        &mut stream,
        &mut reader,
        250,
        &format!("RCPT TO:<{}>\r\n", config.to),
    )?;
    send_expect(&mut stream, &mut reader, 354, "DATA\r\n")?;

    let message_id = format!("bench-{index}-{:?}@{}", Instant::now(), config.host);
    write!(
        stream,
        "From: <{}>\r\nTo: <{}>\r\nSubject: benchmark {index}\r\nMessage-ID: <{}>\r\nDate: Thu, 30 Apr 2026 00:00:00 +0000\r\n\r\n",
        config.from, config.to, message_id
    )?;
    write_payload(&mut stream, config.message_bytes)?;
    stream.write_all(b".\r\n")?;
    stream.flush()?;
    expect_reply(&mut reader, 250)?;
    stream.write_all(b"QUIT\r\n")?;
    let _ = expect_reply(&mut reader, 221);
    Ok(())
}

fn send_expect(
    stream: &mut TcpStream,
    reader: &mut BufReader<TcpStream>,
    code: u16,
    command: &str,
) -> io::Result<()> {
    stream.write_all(command.as_bytes())?;
    stream.flush()?;
    expect_reply(reader, code)
}

fn expect_reply(reader: &mut BufReader<TcpStream>, expected: u16) -> io::Result<()> {
    let expected = expected.to_string();
    let mut line = String::new();
    loop {
        line.clear();
        let len = reader.read_line(&mut line)?;
        if len == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "connection closed while reading SMTP reply",
            ));
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        let bytes = trimmed.as_bytes();
        if bytes.len() < 4 {
            continue;
        }
        if &trimmed[0..3] != expected.as_str() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unexpected SMTP reply: {trimmed}"),
            ));
        }
        if bytes[3] == b' ' {
            return Ok(());
        }
    }
}

fn write_payload(stream: &mut TcpStream, message_bytes: usize) -> io::Result<()> {
    const LINE: &[u8] = b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\r\n";
    let mut remaining = message_bytes;
    while remaining > 0 {
        let chunk = remaining.min(LINE.len());
        stream.write_all(&LINE[..chunk])?;
        if chunk < LINE.len() && !LINE[..chunk].ends_with(b"\n") {
            stream.write_all(b"\r\n")?;
        }
        remaining = remaining.saturating_sub(chunk);
    }
    Ok(())
}

fn percentile_ms(values: &[u128], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let index = ((values.len() - 1) as f64 * percentile).floor() as usize;
    values[index] as f64 / 1000.0
}

fn next_value(args: &mut impl Iterator<Item = String>, name: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{name} requires a value"))
}

fn parse_value<T>(args: &mut impl Iterator<Item = String>, name: &str) -> Result<T, String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    next_value(args, name)?
        .parse()
        .map_err(|error| format!("invalid {name}: {error}"))
}

fn usage() -> String {
    "usage: edgerun-smtp-bench [--host HOST] [--port PORT] [--count N] [--concurrency N] [--from ADDR] [--to ADDR] [--message-bytes N] [--timeout-ms N]".to_string()
}
