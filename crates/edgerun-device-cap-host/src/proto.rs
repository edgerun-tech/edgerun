// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use edgerun_device_cap_core::{
    CapabilityReport, CapabilitySignal, CapabilityValue, CpuCapabilities, DeviceDomainDiagnostic,
    DeviceDomainDiagnostics, DeviceDomainStatus, DeviceDomainStatuses, DomainFieldDiagnostic,
    HostEnvironmentCapabilities, PlatformCapabilities, ProbeConfidence, ProbeErrorCode,
    ProbeSource, QuantitativeResources, ReportMetadata, TargetInfo,
};
use prost::Message;

use crate::benchmark::{
    BenchmarkCaseResult, BenchmarkReport, BenchmarkStatus, DomainAvailabilitySummary,
};
use crate::{CapabilityReportWithDetails, DomainResolvedPaths, ResolvedSourcePaths};

pub mod v1 {
    include!(concat!(env!("OUT_DIR"), "/edgerun.devicecap.v1.rs"));
}

pub fn encode_capability_report(report: &CapabilityReport) -> Result<Vec<u8>, prost::EncodeError> {
    let msg = capability_report_to_proto(report);
    let mut buf = Vec::with_capacity(msg.encoded_len());
    msg.encode(&mut buf)?;
    Ok(buf)
}

pub fn encode_capability_report_with_details(
    report: &CapabilityReportWithDetails,
) -> Result<Vec<u8>, prost::EncodeError> {
    let msg = capability_report_with_details_to_proto(report);
    let mut buf = Vec::with_capacity(msg.encoded_len());
    msg.encode(&mut buf)?;
    Ok(buf)
}

pub fn encode_benchmark_report(report: &BenchmarkReport) -> Result<Vec<u8>, prost::EncodeError> {
    let msg = benchmark_report_to_proto(report);
    let mut buf = Vec::with_capacity(msg.encoded_len());
    msg.encode(&mut buf)?;
    Ok(buf)
}

fn capability_report_to_proto(report: &CapabilityReport) -> v1::CapabilityReportV1 {
    v1::CapabilityReportV1 {
        target: Some(target_to_proto(report.target)),
        platform: Some(platform_to_proto(report.platform)),
        cpu: Some(cpu_to_proto(report.cpu)),
        domains: Some(domain_statuses_to_proto(report.domains)),
        diagnostics: Some(domain_diagnostics_to_proto(report.diagnostics)),
        metrics: Some(metrics_to_proto(report.metrics)),
        metadata: Some(metadata_to_proto(report.metadata)),
        host: Some(host_to_proto(report.host)),
    }
}

fn capability_report_with_details_to_proto(
    report: &CapabilityReportWithDetails,
) -> v1::CapabilityReportWithDetailsV1 {
    v1::CapabilityReportWithDetailsV1 {
        report: Some(capability_report_to_proto(&report.report)),
        resolved_sources: Some(resolved_sources_to_proto(&report.resolved_sources)),
    }
}

fn benchmark_report_to_proto(report: &BenchmarkReport) -> v1::BenchmarkReportV1 {
    let effective = report
        .effective
        .iter()
        .map(|(k, v)| ((*k).to_string(), domain_availability_to_proto(v)))
        .collect::<HashMap<_, _>>();

    v1::BenchmarkReportV1 {
        profile: report.profile.to_string(),
        collected_unix_s: report.collected_unix_s,
        ttl_s: report.ttl_s,
        cases: report.cases.iter().map(benchmark_case_to_proto).collect(),
        effective,
    }
}

fn benchmark_case_to_proto(case: &BenchmarkCaseResult) -> v1::BenchmarkCaseResultV1 {
    v1::BenchmarkCaseResultV1 {
        domain: case.domain.to_string(),
        case_id: case.case.to_string(),
        status: benchmark_status_to_proto(case.status) as i32,
        score_milli: u32::from(case.score_milli),
        duration_ms: case.duration_ms,
        sample_count: case.sample_count,
        error_code: case.error_code.clone(),
        source_path_or_api: case.source_path_or_api.to_string(),
    }
}

fn domain_availability_to_proto(
    summary: &DomainAvailabilitySummary,
) -> v1::DomainAvailabilitySummaryV1 {
    v1::DomainAvailabilitySummaryV1 {
        effective_availability_milli: u32::from(summary.effective_availability_milli),
        confidence: summary.confidence.to_string(),
        blockers: summary.blockers.clone(),
    }
}

fn target_to_proto(value: TargetInfo) -> v1::TargetInfoV1 {
    v1::TargetInfoV1 {
        arch: value.arch.to_string(),
        os: value.os.to_string(),
        env: value.env.to_string(),
        vendor: value.vendor.to_string(),
        endian: value.endian.to_string(),
        pointer_width: value.pointer_width.to_string(),
    }
}

fn cpu_to_proto(value: CpuCapabilities) -> v1::CpuCapabilitiesV1 {
    v1::CpuCapabilitiesV1 {
        simd128: Some(signal_to_proto(value.simd128)),
        simd256: Some(signal_to_proto(value.simd256)),
        simd512: Some(signal_to_proto(value.simd512)),
        aes: Some(signal_to_proto(value.aes)),
        sha2: Some(signal_to_proto(value.sha2)),
        crc32c: Some(signal_to_proto(value.crc32c)),
        random: Some(signal_to_proto(value.random)),
        virtualization: Some(signal_to_proto(value.virtualization)),
        atomics_64: Some(signal_to_proto(value.atomics_64)),
    }
}

fn platform_to_proto(value: PlatformCapabilities) -> v1::PlatformCapabilitiesV1 {
    v1::PlatformCapabilitiesV1 {
        family_unix: Some(signal_to_proto(value.family_unix)),
        family_windows: Some(signal_to_proto(value.family_windows)),
        family_wasm: Some(signal_to_proto(value.family_wasm)),
        atomic_8: Some(signal_to_proto(value.atomic_8)),
        atomic_16: Some(signal_to_proto(value.atomic_16)),
        atomic_32: Some(signal_to_proto(value.atomic_32)),
        atomic_64: Some(signal_to_proto(value.atomic_64)),
        atomic_ptr: Some(signal_to_proto(value.atomic_ptr)),
        panic_abort: Some(signal_to_proto(value.panic_abort)),
        panic_unwind: Some(signal_to_proto(value.panic_unwind)),
    }
}

fn host_to_proto(value: HostEnvironmentCapabilities) -> v1::HostEnvironmentCapabilitiesV1 {
    v1::HostEnvironmentCapabilitiesV1 {
        container_detection: Some(signal_to_proto(value.container_detection)),
        memory_limits: Some(signal_to_proto(value.memory_limits)),
        filesystem_inventory: Some(signal_to_proto(value.filesystem_inventory)),
        network_inventory: Some(signal_to_proto(value.network_inventory)),
        gpu_inventory: Some(signal_to_proto(value.gpu_inventory)),
        bluetooth_inventory: Some(signal_to_proto(value.bluetooth_inventory)),
        nfc_inventory: Some(signal_to_proto(value.nfc_inventory)),
        policy_is_root: Some(signal_to_proto(value.policy_is_root)),
        policy_cap_sys_admin: Some(signal_to_proto(value.policy_cap_sys_admin)),
        policy_cap_net_admin: Some(signal_to_proto(value.policy_cap_net_admin)),
        policy_cap_sys_rawio: Some(signal_to_proto(value.policy_cap_sys_rawio)),
    }
}

fn domain_status_to_proto(value: DeviceDomainStatus) -> v1::DeviceDomainStatusV1 {
    v1::DeviceDomainStatusV1 {
        detected: Some(signal_to_proto(value.detected)),
        available: Some(signal_to_proto(value.available)),
        in_use: Some(signal_to_proto(value.in_use)),
    }
}

fn domain_statuses_to_proto(value: DeviceDomainStatuses) -> v1::DeviceDomainStatusesV1 {
    v1::DeviceDomainStatusesV1 {
        cpu: Some(domain_status_to_proto(value.cpu)),
        storage: Some(domain_status_to_proto(value.storage)),
        gpu: Some(domain_status_to_proto(value.gpu)),
        ram: Some(domain_status_to_proto(value.ram)),
        usb: Some(domain_status_to_proto(value.usb)),
        network: Some(domain_status_to_proto(value.network)),
        input: Some(domain_status_to_proto(value.input)),
        output: Some(domain_status_to_proto(value.output)),
    }
}

fn field_diagnostic_to_proto(value: DomainFieldDiagnostic) -> v1::DomainFieldDiagnosticV1 {
    v1::DomainFieldDiagnosticV1 {
        confidence: probe_confidence_to_proto(value.confidence) as i32,
        error: probe_error_to_proto(value.error) as i32,
        source_path: value.source_path.map(str::to_string),
    }
}

fn domain_diagnostic_to_proto(value: DeviceDomainDiagnostic) -> v1::DeviceDomainDiagnosticV1 {
    v1::DeviceDomainDiagnosticV1 {
        detected: Some(field_diagnostic_to_proto(value.detected)),
        available: Some(field_diagnostic_to_proto(value.available)),
        in_use: Some(field_diagnostic_to_proto(value.in_use)),
    }
}

fn domain_diagnostics_to_proto(value: DeviceDomainDiagnostics) -> v1::DeviceDomainDiagnosticsV1 {
    v1::DeviceDomainDiagnosticsV1 {
        cpu: Some(domain_diagnostic_to_proto(value.cpu)),
        storage: Some(domain_diagnostic_to_proto(value.storage)),
        gpu: Some(domain_diagnostic_to_proto(value.gpu)),
        ram: Some(domain_diagnostic_to_proto(value.ram)),
        usb: Some(domain_diagnostic_to_proto(value.usb)),
        network: Some(domain_diagnostic_to_proto(value.network)),
        input: Some(domain_diagnostic_to_proto(value.input)),
        output: Some(domain_diagnostic_to_proto(value.output)),
    }
}

fn metrics_to_proto(value: QuantitativeResources) -> v1::QuantitativeResourcesV1 {
    v1::QuantitativeResourcesV1 {
        cpu_logical_cores_total: value.cpu_logical_cores_total.map(u32::from),
        cpu_logical_cores_allocatable: value.cpu_logical_cores_allocatable.map(u32::from),
        ram_bytes_total: value.ram_bytes_total,
        ram_bytes_available: value.ram_bytes_available,
        storage_bytes_total: value.storage_bytes_total,
        storage_bytes_available: value.storage_bytes_available,
        gpu_count_total: value.gpu_count_total.map(u32::from),
        gpu_count_available: value.gpu_count_available.map(u32::from),
        network_interface_count: value.network_interface_count.map(u32::from),
        usb_device_count: value.usb_device_count.map(u32::from),
        input_device_count: value.input_device_count.map(u32::from),
        output_device_count: value.output_device_count.map(u32::from),
        bluetooth_adapter_count: value.bluetooth_adapter_count.map(u32::from),
        nfc_adapter_count: value.nfc_adapter_count.map(u32::from),
        cpu_load_ratio_per_core_milli: value.cpu_load_ratio_per_core_milli.map(u32::from),
        ram_bytes_used: value.ram_bytes_used,
        storage_bytes_used: value.storage_bytes_used,
        network_links_up_count: value.network_links_up_count.map(u32::from),
        gpu_busy_percent: value.gpu_busy_percent.map(u32::from),
    }
}

fn metadata_to_proto(value: ReportMetadata) -> v1::ReportMetadataV1 {
    v1::ReportMetadataV1 {
        collected_unix_s: value.collected_unix_s,
        ttl_s: value.ttl_s,
    }
}

fn domain_paths_to_proto(value: &DomainResolvedPaths) -> v1::DomainResolvedPathsV1 {
    v1::DomainResolvedPathsV1 {
        detected: value.detected.clone(),
        available: value.available.clone(),
        in_use: value.in_use.clone(),
    }
}

fn resolved_sources_to_proto(value: &ResolvedSourcePaths) -> v1::ResolvedSourcePathsV1 {
    v1::ResolvedSourcePathsV1 {
        cpu: Some(domain_paths_to_proto(&value.cpu)),
        storage: Some(domain_paths_to_proto(&value.storage)),
        gpu: Some(domain_paths_to_proto(&value.gpu)),
        ram: Some(domain_paths_to_proto(&value.ram)),
        usb: Some(domain_paths_to_proto(&value.usb)),
        network: Some(domain_paths_to_proto(&value.network)),
        input: Some(domain_paths_to_proto(&value.input)),
        output: Some(domain_paths_to_proto(&value.output)),
        bluetooth: Some(domain_paths_to_proto(&value.bluetooth)),
        nfc: Some(domain_paths_to_proto(&value.nfc)),
    }
}

fn signal_to_proto(value: CapabilitySignal) -> v1::CapabilitySignalV1 {
    v1::CapabilitySignalV1 {
        value: capability_value_to_proto(value.value) as i32,
        source: probe_source_to_proto(value.source) as i32,
    }
}

fn capability_value_to_proto(value: CapabilityValue) -> v1::CapabilityValueV1 {
    match value {
        CapabilityValue::Supported => v1::CapabilityValueV1::Supported,
        CapabilityValue::Unsupported => v1::CapabilityValueV1::Unsupported,
        CapabilityValue::Unknown => v1::CapabilityValueV1::Unknown,
    }
}

fn probe_source_to_proto(value: ProbeSource) -> v1::ProbeSourceV1 {
    match value {
        ProbeSource::Runtime => v1::ProbeSourceV1::Runtime,
        ProbeSource::CompileTime => v1::ProbeSourceV1::CompileTime,
        ProbeSource::NotAvailable => v1::ProbeSourceV1::NotAvailable,
    }
}

fn probe_confidence_to_proto(value: ProbeConfidence) -> v1::ProbeConfidenceV1 {
    match value {
        ProbeConfidence::High => v1::ProbeConfidenceV1::High,
        ProbeConfidence::Medium => v1::ProbeConfidenceV1::Medium,
        ProbeConfidence::Low => v1::ProbeConfidenceV1::Low,
        ProbeConfidence::Unknown => v1::ProbeConfidenceV1::Unknown,
    }
}

fn probe_error_to_proto(value: ProbeErrorCode) -> v1::ProbeErrorCodeV1 {
    match value {
        ProbeErrorCode::None => v1::ProbeErrorCodeV1::None,
        ProbeErrorCode::PermissionDenied => v1::ProbeErrorCodeV1::PermissionDenied,
        ProbeErrorCode::NotFound => v1::ProbeErrorCodeV1::NotFound,
        ProbeErrorCode::ParseFailed => v1::ProbeErrorCodeV1::ParseFailed,
        ProbeErrorCode::IoFailure => v1::ProbeErrorCodeV1::IoFailure,
        ProbeErrorCode::NotSupported => v1::ProbeErrorCodeV1::NotSupported,
        ProbeErrorCode::Unknown => v1::ProbeErrorCodeV1::Unknown,
    }
}

fn benchmark_status_to_proto(value: BenchmarkStatus) -> v1::BenchmarkStatusV1 {
    match value {
        BenchmarkStatus::Pass => v1::BenchmarkStatusV1::Pass,
        BenchmarkStatus::Degraded => v1::BenchmarkStatusV1::Degraded,
        BenchmarkStatus::Fail => v1::BenchmarkStatusV1::Fail,
        BenchmarkStatus::Blocked => v1::BenchmarkStatusV1::Blocked,
        BenchmarkStatus::Unknown => v1::BenchmarkStatusV1::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_report_encodes_to_protobuf() {
        let report = crate::probe_capabilities_with_host();
        let payload = encode_capability_report(&report).expect("encode report");
        assert!(!payload.is_empty());
        let decoded = v1::CapabilityReportV1::decode(payload.as_slice()).expect("decode report");
        assert!(decoded.target.is_some());
    }

    #[test]
    fn benchmark_report_encodes_to_protobuf() {
        let report = BenchmarkReport {
            profile: "edge-standard",
            collected_unix_s: 1,
            ttl_s: 30,
            cases: Vec::new(),
            effective: Default::default(),
        };
        let payload = encode_benchmark_report(&report).expect("encode benchmark report");
        assert!(!payload.is_empty());
        let decoded = v1::BenchmarkReportV1::decode(payload.as_slice()).expect("decode benchmark");
        assert_eq!(decoded.profile, "edge-standard");
    }
}
