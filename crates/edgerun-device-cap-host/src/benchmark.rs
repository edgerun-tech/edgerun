// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use edgerun_device_cap_core::{CapabilityReport, CapabilityValue, ProbeConfidence};
use serde::Serialize;

#[derive(Debug, Clone, Copy)]
pub enum BenchmarkProfile {
    RouterLite,
    EdgeStandard,
    EdgePerformance,
}

impl BenchmarkProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RouterLite => "router-lite",
            Self::EdgeStandard => "edge-standard",
            Self::EdgePerformance => "edge-performance",
        }
    }

    const fn ttl_s(self) -> u32 {
        match self {
            Self::RouterLite => 30,
            Self::EdgeStandard => 45,
            Self::EdgePerformance => 60,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkStatus {
    Pass,
    Degraded,
    Fail,
    Blocked,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkCaseResult {
    pub domain: &'static str,
    pub case: &'static str,
    pub status: BenchmarkStatus,
    pub score_milli: u16,
    pub duration_ms: u64,
    pub sample_count: u32,
    pub error_code: Option<String>,
    pub source_path_or_api: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct DomainAvailabilitySummary {
    pub effective_availability_milli: u16,
    pub confidence: &'static str,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkReport {
    pub profile: &'static str,
    pub collected_unix_s: u64,
    pub ttl_s: u32,
    pub cases: Vec<BenchmarkCaseResult>,
    pub effective: BTreeMap<&'static str, DomainAvailabilitySummary>,
}

pub fn run_benchmark_suite(
    profile: BenchmarkProfile,
    output_root: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let capability_report = crate::probe_capabilities_with_host();
    let cases = run_cases(profile, output_root)?;

    let collected_unix_s = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let report = BenchmarkReport {
        profile: profile.as_str(),
        collected_unix_s,
        ttl_s: profile.ttl_s(),
        effective: compute_effective_summary(&capability_report, collected_unix_s, &cases),
        cases,
    };

    fs::create_dir_all(output_root)?;
    let output_path = output_root.join("report.pb");
    let payload = crate::proto::encode_benchmark_report(&report)?;
    fs::write(&output_path, payload)?;

    Ok(output_path)
}

fn run_cases(
    profile: BenchmarkProfile,
    output_root: &Path,
) -> Result<Vec<BenchmarkCaseResult>, Box<dyn std::error::Error + Send + Sync>> {
    let cases = vec![
        cpu_compute_int_case(profile),
        ram_bandwidth_case(profile),
        storage_seq_write_case(profile, output_root)?,
        storage_seq_read_case(profile, output_root)?,
        network_loopback_rtt_case(profile),
        network_loopback_throughput_case(profile)?,
    ];

    Ok(cases)
}

fn cpu_compute_int_case(profile: BenchmarkProfile) -> BenchmarkCaseResult {
    let duration = match profile {
        BenchmarkProfile::RouterLite => Duration::from_millis(180),
        BenchmarkProfile::EdgeStandard => Duration::from_millis(250),
        BenchmarkProfile::EdgePerformance => Duration::from_millis(400),
    };
    let (bad_ops_per_s, good_ops_per_s) = match profile {
        BenchmarkProfile::RouterLite => (10_000_000.0, 45_000_000.0),
        BenchmarkProfile::EdgeStandard => (20_000_000.0, 90_000_000.0),
        BenchmarkProfile::EdgePerformance => (35_000_000.0, 140_000_000.0),
    };

    let start = Instant::now();
    let mut x: u64 = 0x1234_5678_9abc_def0;
    let mut ops: u64 = 0;
    while start.elapsed() < duration {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
        x ^= x >> 7;
        x = x.rotate_left(13);
        ops = ops.saturating_add(1);
    }
    std::hint::black_box(x);

    let elapsed = start.elapsed();
    let ops_per_s = if elapsed.as_secs_f64() > 0.0 {
        ops as f64 / elapsed.as_secs_f64()
    } else {
        0.0
    };
    let score = throughput_score_milli(ops_per_s, bad_ops_per_s, good_ops_per_s);

    BenchmarkCaseResult {
        domain: "cpu",
        case: "cpu.compute.int",
        status: score_to_status(score),
        score_milli: score,
        duration_ms: elapsed.as_millis() as u64,
        sample_count: ops.min(u32::MAX as u64) as u32,
        error_code: None,
        source_path_or_api: "std::time::Instant+integer_loop",
    }
}

fn ram_bandwidth_case(profile: BenchmarkProfile) -> BenchmarkCaseResult {
    let (buf_bytes, loops) = match profile {
        BenchmarkProfile::RouterLite => (4 * 1024 * 1024usize, 4u32),
        BenchmarkProfile::EdgeStandard => (16 * 1024 * 1024usize, 6u32),
        BenchmarkProfile::EdgePerformance => (32 * 1024 * 1024usize, 8u32),
    };
    let (bad_mib_s, good_mib_s) = match profile {
        BenchmarkProfile::RouterLite => (150.0, 1000.0),
        BenchmarkProfile::EdgeStandard => (600.0, 5000.0),
        BenchmarkProfile::EdgePerformance => (1200.0, 9000.0),
    };

    let src = vec![0xA5u8; buf_bytes];
    let mut dst = vec![0u8; buf_bytes];

    let start = Instant::now();
    for _ in 0..loops {
        dst.copy_from_slice(&src);
    }
    std::hint::black_box(&dst);
    let elapsed = start.elapsed();

    let total_bytes = (buf_bytes as u128) * (loops as u128);
    let mib_s = if elapsed.as_secs_f64() > 0.0 {
        (total_bytes as f64 / (1024.0 * 1024.0)) / elapsed.as_secs_f64()
    } else {
        0.0
    };
    let score = throughput_score_milli(mib_s, bad_mib_s, good_mib_s);

    BenchmarkCaseResult {
        domain: "ram",
        case: "ram.bandwidth.copy",
        status: score_to_status(score),
        score_milli: score,
        duration_ms: elapsed.as_millis() as u64,
        sample_count: loops,
        error_code: None,
        source_path_or_api: "memory_copy_loop",
    }
}

fn storage_seq_write_case(
    profile: BenchmarkProfile,
    output_root: &Path,
) -> Result<BenchmarkCaseResult, Box<dyn std::error::Error + Send + Sync>> {
    let file_size = match profile {
        BenchmarkProfile::RouterLite => 16 * 1024 * 1024usize,
        BenchmarkProfile::EdgeStandard => 128 * 1024 * 1024usize,
        BenchmarkProfile::EdgePerformance => 256 * 1024 * 1024usize,
    };
    let (bad_mib_s, good_mib_s) = match profile {
        BenchmarkProfile::RouterLite => (10.0, 80.0),
        BenchmarkProfile::EdgeStandard => (40.0, 450.0),
        BenchmarkProfile::EdgePerformance => (80.0, 800.0),
    };

    let tmp_dir = output_root.join("tmp");
    fs::create_dir_all(&tmp_dir)?;
    let path = tmp_dir.join("storage_seq.bin");
    let data = vec![0x5Au8; file_size];

    let start = Instant::now();
    let mut f = File::create(&path)?;
    f.write_all(&data)?;
    f.sync_all()?;
    drop(f);
    let elapsed = start.elapsed();

    let mib_s = if elapsed.as_secs_f64() > 0.0 {
        (file_size as f64 / (1024.0 * 1024.0)) / elapsed.as_secs_f64()
    } else {
        0.0
    };
    let score = throughput_score_milli(mib_s, bad_mib_s, good_mib_s);

    Ok(BenchmarkCaseResult {
        domain: "storage",
        case: "storage.seq.write",
        status: score_to_status(score),
        score_milli: score,
        duration_ms: elapsed.as_millis() as u64,
        sample_count: 1,
        error_code: None,
        source_path_or_api: "out/bench/tmp/storage_seq.bin",
    })
}

fn storage_seq_read_case(
    profile: BenchmarkProfile,
    output_root: &Path,
) -> Result<BenchmarkCaseResult, Box<dyn std::error::Error + Send + Sync>> {
    let (bad_mib_s, good_mib_s) = match profile {
        BenchmarkProfile::RouterLite => (15.0, 120.0),
        BenchmarkProfile::EdgeStandard => (60.0, 750.0),
        BenchmarkProfile::EdgePerformance => (120.0, 1400.0),
    };

    let path = output_root.join("tmp").join("storage_seq.bin");
    let mut data = Vec::new();
    let start = Instant::now();
    let mut f = File::open(&path)?;
    f.read_to_end(&mut data)?;
    let elapsed = start.elapsed();

    let mib_s = if elapsed.as_secs_f64() > 0.0 {
        (data.len() as f64 / (1024.0 * 1024.0)) / elapsed.as_secs_f64()
    } else {
        0.0
    };
    let score = throughput_score_milli(mib_s, bad_mib_s, good_mib_s);

    Ok(BenchmarkCaseResult {
        domain: "storage",
        case: "storage.seq.read",
        status: score_to_status(score),
        score_milli: score,
        duration_ms: elapsed.as_millis() as u64,
        sample_count: 1,
        error_code: None,
        source_path_or_api: "out/bench/tmp/storage_seq.bin",
    })
}

fn network_loopback_rtt_case(profile: BenchmarkProfile) -> BenchmarkCaseResult {
    let sample_count = match profile {
        BenchmarkProfile::RouterLite => 32u32,
        BenchmarkProfile::EdgeStandard => 64u32,
        BenchmarkProfile::EdgePerformance => 128u32,
    };
    let (good_us, bad_us) = match profile {
        BenchmarkProfile::RouterLite => (700.0, 6_000.0),
        BenchmarkProfile::EdgeStandard => (400.0, 3_000.0),
        BenchmarkProfile::EdgePerformance => (250.0, 1_800.0),
    };

    let start = Instant::now();
    let a = match std::net::UdpSocket::bind("127.0.0.1:0") {
        Ok(v) => v,
        Err(e) => return failed_network_case("network.loopback.rtt", e),
    };
    let b = match std::net::UdpSocket::bind("127.0.0.1:0") {
        Ok(v) => v,
        Err(e) => return failed_network_case("network.loopback.rtt", e),
    };
    let b_addr = match b.local_addr() {
        Ok(v) => v,
        Err(e) => return failed_network_case("network.loopback.rtt", e),
    };
    let _ = a.set_read_timeout(Some(Duration::from_millis(250)));
    let _ = b.set_read_timeout(Some(Duration::from_millis(250)));

    let mut rtts_us = Vec::with_capacity(sample_count as usize);
    let mut rx_buf = [0u8; 64];
    for i in 0..sample_count {
        let payload = i.to_le_bytes();
        let t0 = Instant::now();
        if let Err(e) = a.send_to(&payload, b_addr) {
            return failed_network_case("network.loopback.rtt", e);
        }
        let (n, src) = match b.recv_from(&mut rx_buf) {
            Ok(v) => v,
            Err(e) => return failed_network_case("network.loopback.rtt", e),
        };
        if let Err(e) = b.send_to(&rx_buf[..n], src) {
            return failed_network_case("network.loopback.rtt", e);
        }
        if let Err(e) = a.recv_from(&mut rx_buf) {
            return failed_network_case("network.loopback.rtt", e);
        }
        rtts_us.push(t0.elapsed().as_secs_f64() * 1_000_000.0);
    }

    rtts_us.sort_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap_or(std::cmp::Ordering::Equal));
    let p95_idx = ((rtts_us.len().saturating_sub(1)) * 95) / 100;
    let p95_us = rtts_us.get(p95_idx).copied().unwrap_or(bad_us);
    let score = latency_score_milli(p95_us, good_us, bad_us);

    BenchmarkCaseResult {
        domain: "network",
        case: "network.loopback.rtt",
        status: score_to_status(score),
        score_milli: score,
        duration_ms: start.elapsed().as_millis() as u64,
        sample_count,
        error_code: None,
        source_path_or_api: "std::net::UdpSocket(loopback_ping_pong)",
    }
}

fn network_loopback_throughput_case(
    profile: BenchmarkProfile,
) -> Result<BenchmarkCaseResult, Box<dyn std::error::Error + Send + Sync>> {
    let total_bytes = match profile {
        BenchmarkProfile::RouterLite => 2 * 1024 * 1024usize,
        BenchmarkProfile::EdgeStandard => 16 * 1024 * 1024usize,
        BenchmarkProfile::EdgePerformance => 64 * 1024 * 1024usize,
    };
    let (bad_mib_s, good_mib_s) = match profile {
        BenchmarkProfile::RouterLite => (20.0, 150.0),
        BenchmarkProfile::EdgeStandard => (80.0, 700.0),
        BenchmarkProfile::EdgePerformance => (120.0, 1300.0),
    };

    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let listen_addr = listener.local_addr()?;
    let receiver = std::thread::spawn(move || -> std::io::Result<usize> {
        let (mut conn, _) = listener.accept()?;
        let mut received = 0usize;
        let mut buf = [0u8; 64 * 1024];
        while received < total_bytes {
            let n = conn.read(&mut buf)?;
            if n == 0 {
                break;
            }
            received = received.saturating_add(n);
        }
        Ok(received)
    });

    let mut sender = std::net::TcpStream::connect(listen_addr)?;
    sender.set_nodelay(true)?;
    let chunk = vec![0x5Au8; 64 * 1024];
    let mut sent = 0usize;
    let start = Instant::now();
    while sent < total_bytes {
        let remaining = total_bytes - sent;
        let write_len = remaining.min(chunk.len());
        sender.write_all(&chunk[..write_len])?;
        sent = sent.saturating_add(write_len);
    }
    sender.flush()?;
    drop(sender);
    let elapsed = start.elapsed();

    let received = receiver
        .join()
        .map_err(|_| std::io::Error::other("receiver_thread_panic"))??;
    let transferred = received.min(total_bytes);
    let mib_s = if elapsed.as_secs_f64() > 0.0 {
        (transferred as f64 / (1024.0 * 1024.0)) / elapsed.as_secs_f64()
    } else {
        0.0
    };
    let score = throughput_score_milli(mib_s, bad_mib_s, good_mib_s);

    Ok(BenchmarkCaseResult {
        domain: "network",
        case: "network.loopback.throughput",
        status: score_to_status(score),
        score_milli: score,
        duration_ms: elapsed.as_millis() as u64,
        sample_count: 1,
        error_code: None,
        source_path_or_api: "std::net::TcpStream(loopback_burst)",
    })
}

fn failed_network_case(case: &'static str, err: std::io::Error) -> BenchmarkCaseResult {
    let (status, code) = if err.kind() == std::io::ErrorKind::PermissionDenied {
        (BenchmarkStatus::Blocked, "permission_denied".to_string())
    } else {
        (BenchmarkStatus::Fail, err.kind().to_string())
    };
    BenchmarkCaseResult {
        domain: "network",
        case,
        status,
        score_milli: 0,
        duration_ms: 0,
        sample_count: 1,
        error_code: Some(code),
        source_path_or_api: "std::net(loopback)",
    }
}

fn compute_effective_summary(
    report: &CapabilityReport,
    now_unix_s: u64,
    cases: &[BenchmarkCaseResult],
) -> BTreeMap<&'static str, DomainAvailabilitySummary> {
    let mut out = BTreeMap::new();
    for domain in ["cpu", "ram", "storage", "network"] {
        let detection_score = domain_detection_score(report, domain);
        let permission_score = domain_permission_score(report, domain);
        let performance_score = domain_perf_score(cases, domain);
        let freshness_score = domain_freshness_score(report, now_unix_s);

        let mut blockers = Vec::new();
        if permission_score == 0 {
            blockers.push("permission_blocked".to_string());
        }
        if performance_score == 0 {
            blockers.push("benchmark_fail_or_missing".to_string());
        }

        let weighted = weighted_score(
            detection_score,
            permission_score,
            performance_score,
            freshness_score,
        );
        let confidence = domain_confidence(report, domain);

        out.insert(
            domain,
            DomainAvailabilitySummary {
                effective_availability_milli: weighted,
                confidence,
                blockers,
            },
        );
    }

    out
}

fn domain_detection_score(report: &CapabilityReport, domain: &str) -> u16 {
    match domain_signal_detected(report, domain).value {
        CapabilityValue::Supported => 1000,
        CapabilityValue::Unsupported => 0,
        CapabilityValue::Unknown => 300,
    }
}

fn domain_permission_score(report: &CapabilityReport, domain: &str) -> u16 {
    match domain_signal_available(report, domain).value {
        CapabilityValue::Supported => 1000,
        CapabilityValue::Unsupported => 0,
        CapabilityValue::Unknown => 300,
    }
}

fn domain_perf_score(cases: &[BenchmarkCaseResult], domain: &str) -> u16 {
    let mut sum = 0u32;
    let mut count = 0u32;
    for case in cases.iter().filter(|c| c.domain == domain) {
        sum = sum.saturating_add(u32::from(case.score_milli));
        count = count.saturating_add(1);
    }
    if count == 0 {
        0
    } else {
        (sum / count) as u16
    }
}

fn domain_freshness_score(report: &CapabilityReport, now_unix_s: u64) -> u16 {
    let Some(collected) = report.metadata.collected_unix_s else {
        return 500;
    };
    let Some(ttl) = report.metadata.ttl_s else {
        return 500;
    };
    let age = now_unix_s.saturating_sub(collected);
    if age <= u64::from(ttl) {
        1000
    } else {
        0
    }
}

fn weighted_score(detection: u16, permission: u16, performance: u16, freshness: u16) -> u16 {
    let score = (u32::from(detection) * 20)
        + (u32::from(permission) * 30)
        + (u32::from(performance) * 40)
        + (u32::from(freshness) * 10);
    (score / 100) as u16
}

fn domain_confidence(report: &CapabilityReport, domain: &str) -> &'static str {
    let diag = match domain {
        "cpu" => report.diagnostics.cpu,
        "ram" => report.diagnostics.ram,
        "storage" => report.diagnostics.storage,
        "network" => report.diagnostics.network,
        "gpu" => report.diagnostics.gpu,
        _ => return "unknown",
    };

    match diag.available.confidence {
        ProbeConfidence::High => "high",
        ProbeConfidence::Medium => "medium",
        ProbeConfidence::Low => "low",
        ProbeConfidence::Unknown => "unknown",
    }
}

fn domain_signal_detected(
    report: &CapabilityReport,
    domain: &str,
) -> edgerun_device_cap_core::CapabilitySignal {
    match domain {
        "cpu" => report.domains.cpu.detected,
        "ram" => report.domains.ram.detected,
        "storage" => report.domains.storage.detected,
        "network" => report.domains.network.detected,
        "gpu" => report.domains.gpu.detected,
        _ => report.domains.cpu.detected,
    }
}

fn domain_signal_available(
    report: &CapabilityReport,
    domain: &str,
) -> edgerun_device_cap_core::CapabilitySignal {
    match domain {
        "cpu" => report.domains.cpu.available,
        "ram" => report.domains.ram.available,
        "storage" => report.domains.storage.available,
        "network" => report.domains.network.available,
        "gpu" => report.domains.gpu.available,
        _ => report.domains.cpu.available,
    }
}

fn throughput_score_milli(value: f64, bad: f64, good: f64) -> u16 {
    if !value.is_finite() || !bad.is_finite() || !good.is_finite() || good <= bad {
        return 0;
    }
    if value <= bad {
        return 0;
    }
    if value >= good {
        return 1000;
    }
    (((value - bad) * 1000.0) / (good - bad)).round() as u16
}

fn latency_score_milli(value: f64, good: f64, bad: f64) -> u16 {
    if !value.is_finite() || !bad.is_finite() || !good.is_finite() || bad <= good {
        return 0;
    }
    if value <= good {
        return 1000;
    }
    if value >= bad {
        return 0;
    }
    (((bad - value) * 1000.0) / (bad - good)).round() as u16
}

fn score_to_status(score: u16) -> BenchmarkStatus {
    if score >= 800 {
        BenchmarkStatus::Pass
    } else if score >= 600 {
        BenchmarkStatus::Degraded
    } else if score > 0 {
        BenchmarkStatus::Fail
    } else {
        BenchmarkStatus::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_score_balances_inputs() {
        assert_eq!(weighted_score(1000, 1000, 1000, 1000), 1000);
        assert_eq!(weighted_score(0, 0, 0, 0), 0);
    }

    #[test]
    fn throughput_score_uses_linear_band() {
        assert_eq!(throughput_score_milli(50.0, 100.0, 300.0), 0);
        assert_eq!(throughput_score_milli(300.0, 100.0, 300.0), 1000);
        assert_eq!(throughput_score_milli(200.0, 100.0, 300.0), 500);
    }

    #[test]
    fn latency_score_uses_inverse_linear_band() {
        assert_eq!(latency_score_milli(50.0, 100.0, 500.0), 1000);
        assert_eq!(latency_score_milli(500.0, 100.0, 500.0), 0);
        assert_eq!(latency_score_milli(300.0, 100.0, 500.0), 500);
    }
}
