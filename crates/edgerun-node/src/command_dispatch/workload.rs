use edgerun_log;
use edgerun_core::command::{command_hash, validate_command, CommandValidationContext};
use edgerun_core::protocol::{canonical_bytes, ProtocolRecord, EventEnvelope, Digest};
use edgerun_core::result::Verdict;
use edgerun_hardware_signing::MeshSigner;
use edgerun_storage::NodeStore;
use edgerun_proto::edgerun::v0::stream::{CommandDecision, CommandEnvelope, CommandResultPayload as ProtoCommandResultPayload, CommandType, EventType};
use edgerun_proto::edgerun::v0::trust::{DelegationRecord as ProtoDelegationRecord, RevocationRecord as ProtoRevocationRecord};
use edgerun_proto::edgerun::v0::common::{CommandRef, DelegationRef};
use edgerun_crypto::rand_core::RngCore;
use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashVerifier;
use prost::Message;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

    let workload_spec = match &command.payload {
        Some(edgerun_proto::edgerun::v0::stream::command_envelope::Payload::InlinePayload(bytes)) => bytes.clone(),
        _ => {
            return record_and_respond(command, store, stream_id, signer, controllers,
                false, "missing_workload_spec", Vec::new(), None);
        }
    };

    // Parse image ref and resource hints from spec
    let spec_str = String::from_utf8_lossy(&workload_spec);
    let (image_str, allocated_cores, allocated_memory_bytes) = parse_workload_spec(&workload_spec);

    // === WORKLOAD POLICY CHECK: reject disallowed images ===
    if let Err(reason) = workload_policy.validate(&image_str) {
        edgerun_log::warn!("workload policy rejected: {} — {}", image_str, reason);
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, &format!("policy_violation: {}", reason), Vec::new(), None);
    }

    let image_ref: edgerun_oci_registry::ImageRef = match image_str.parse() {
        Ok(img) => img,
        Err(e) => {
            return record_and_respond(command, store, stream_id, signer, controllers,
                false, &format!("invalid_image_ref: {}", e), Vec::new(), None);
        }
    };

    // === RATE LIMIT CHECK: prevent workload spam ===
    let requester_id = command.issuer.as_ref().map(|i| i.identity_id.clone()).unwrap_or_default();
    if !rate_limiter.check_and_record(&requester_id) {
        edgerun_log::warn!("rate limit exceeded for requester: {}", edgerun_core::util::bytes_to_hex(&requester_id));
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, "rate_limit_exceeded", Vec::new(), None);
    }

    // Compute deterministic work ID (normalized spec)
    let normalized_spec = normalize_workload_spec(&workload_spec);
    let work_id = compute_work_id(&normalized_spec);

    // Extract requester identity
    let requester_id_bytes = match &command.issuer {
        Some(issuer) => {
            let mut id = [0u8; 64];
            if let Some(ref hint) = issuer.key_hint {
                if hint.len() == 64 {
                    id.copy_from_slice(hint);
                }
            }
            id
        }
        None => {
            return record_and_respond(command, store, stream_id, signer, controllers,
                false, "missing_requester_identity", Vec::new(), None);
        }
    };

    let delegation_hash = if let Some(first) = command.delegation_chain.first() {
        let mut signable = first.clone();
        signable.signature = None;
        let mut canonical = Vec::new();
        if prost::Message::encode(&signable, &mut canonical).is_ok() {
            let d = edgerun_core::crypto::sha256(&canonical);
            let mut h = [0u8; 32];
            h.copy_from_slice(&d);
            h
        } else {
            [0u8; 32]
        }
    } else {
        [0u8; 32]
    };

    let provider_id = signer.node_id().0;
    let mut cert = match load_cached_cert(store) {
        Some(c) => c,
        None => {
            let c = edgerun_core::benchmark::run_full_benchmark(provider_id);
            cache_cert(store, &c, signer);
            c
        }
    };

    // Sign the certificate if it's not already signed (legacy certs)
    if cert.signature == [0u8; 64] {
        let mut digest_32 = [0u8; 32];
        digest_32.copy_from_slice(&cert.digest);
        if let Ok(sig) = signer.sign_digest(&digest_32) {
            cert.signature = sig;
            cache_cert(store, &cert, signer); // Re-cache with signature
        }
    }

    // === CERTIFICATE SIGNATURE VERIFICATION ===
    // The certificate must be self-signed by the node's key to prevent fabrication.
    if !cert.verify() {
        edgerun_log::error!("performance certificate signature verification failed — possible fabrication");
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, "invalid_certificate_signature", Vec::new(), None);
    }

    // Also verify the certificate's node_id matches our node identity
    if cert.node_id != provider_id {
        edgerun_log::error!("performance certificate node_id does not match local node identity");
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, "certificate_identity_mismatch", Vec::new(), None);
    }

    // === CERTIFICATE FRESHNESS CHECK: reject stale certs ===
    if let Err(reason) = check_cert_freshness_impl(&cert) {
        edgerun_log::warn!("certificate freshness check failed: {}", reason);
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, &format!("stale_certificate: {}", reason), Vec::new(), None);
    }

    // === CAPACITY CHECK: do we have enough resources? ===
    if !capacity_tracker.try_allocate(allocated_cores, allocated_memory_bytes) {
        edgerun_log::warn!("insufficient capacity: need {} cores, {} bytes (have {} cores, {} bytes)",
            allocated_cores, allocated_memory_bytes,
            capacity_tracker.available_cores(),
            capacity_tracker.available_memory());
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, "insufficient_capacity", Vec::new(), None);
    }

    // Determine workload class from image name and resource profile
    // Uses the parsed image_ref (not raw spec string) to avoid false substring matches.
    let workload_class = {
        let img_name = image_ref.repository.to_lowercase();
        if img_name.contains("inference") || img_name.contains("llm") || img_name.contains("model") {
            WorkloadClass::Inference
        } else if img_name.contains("compil") || img_name.contains("build") {
            WorkloadClass::Compilation
        } else if image_str.starts_with("container:") || img_name.contains("container") {
            WorkloadClass::Container
        } else {
            WorkloadClass::General
        }
    };

    // === PHASE 1: Pull image ===
    let paths = WorkloadPaths::new(&work_id);
    let bundle_dir = &paths.bundle_dir;
    let store_dir = &paths.store_dir;

    // Start metering BEFORE pull so download time is billed
    let meter = WorkMeter::new(
        work_id, requester_id_bytes, provider_id, delegation_hash,
        allocated_cores, allocated_memory_bytes,
        workload_class, WorkPriority::Standard, &cert,
    );

    edgerun_log::info!("pulling: {}", image_str);
    if let Err(e) = pull_with_metering(&image_ref, bundle_dir, store_dir, &meter) {
        edgerun_log::warn!("pull failed: {}", e);
        capacity_tracker.release(allocated_cores, allocated_memory_bytes);
        let acc = meter.finalize(WorkStatus::Failed, None);
        let _ = store.record_work_accounting(&acc);
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, &format!("pull_failed: {}", e), Vec::new(), None);
    }

    // === PHASE 2: Apply resource limits ===
    let cfg_path = bundle_dir.join("config.json");
    if let Ok(txt) = std::fs::read_to_string(&cfg_path) {
        if let Ok(mut spec) = edgerun_oci_runtime::json::parse_oci_spec(txt.as_bytes()) {
            let shares = (allocated_cores as u64).saturating_mul(1024);
            if let Some(linux) = spec.linux.as_mut() {
                linux.resources = Some(edgerun_oci_runtime::OciLinuxResources {
                    memory: Some(edgerun_oci_runtime::OciLinuxMemory {
                        limit: Some(allocated_memory_bytes as i64),
                        reservation: None,
                        swap: None,
                        kernel: None,
                        kernel_tcp: None,
                        check_before_update: None,
                    }),
                    cpu: Some(edgerun_oci_runtime::OciLinuxCpu {
                        shares: Some(shares),
                        quota: None,
                        period: None,
                        realtime_runtime: None,
                        realtime_period: None,
                        cpus: None,
                        mems: None,
                        idle: None,
                        burst: None,
                    }),
                    pids: Some(edgerun_oci_runtime::OciLinuxPids { limit: 256 }),
                    block_io: None,
                    devices: None,
                    hugepage_limits: None,
                    network: None,
                });
            }
            let json = spec.to_json_string_pretty();
            if let Err(e) = std::fs::write(&cfg_path, json) {
                edgerun_log::warn!("failed to update config.json: {}", e);
                // ABORT: container must not start without resource limits.
                // Running unconstrained would violate the capacity guarantee.
                capacity_tracker.release(allocated_cores, allocated_memory_bytes);
                let _ = std::fs::remove_dir_all(&paths.tmp_base);
                let acc = meter.finalize(WorkStatus::Failed, None);
                let _ = store.record_work_accounting(&acc);
                return record_and_respond(command, store, stream_id, signer, controllers,
                    false, "resource_limit_write_failed", Vec::new(), None);
            }
        }
    }

    // === PHASE 3: Run container (non-blocking) ===
    edgerun_log::info!("running: {}", image_str);
    let container = match edgerun_oci_runtime::start_bundle(&bundle_dir) {
        Ok(c) => c,
        Err(e) => {
            edgerun_log::warn!("run failed: {}", e);
            capacity_tracker.release(allocated_cores, allocated_memory_bytes);
            let acc = meter.finalize(WorkStatus::Failed, None);
            let _ = store.record_work_accounting(&acc);
            let _ = std::fs::remove_dir_all(&paths.tmp_base);
            return record_and_respond(command, store, stream_id, signer, controllers,
                false, &format!("run_failed: {}", e), Vec::new(), None);
        }
    };

    let container_pid = container.pid();

    // Register in running workloads so it can be preempted
    let workload_info = super::running_workloads::RunningWorkloadInfo {
        work_id,
        pid: container_pid,
        allocated_cores,
        allocated_memory_bytes,
        bundle_path: paths.tmp_base.clone(),
        cgroup_path: paths.cgroup_path.clone(),
    };
    match running_workloads.register(workload_info) {
        Ok(()) => {}
        Err(_existing) => {
            edgerun_log::warn!("work {} already running (duplicate work_id)",
                edgerun_core::util::bytes_to_hex(&work_id[..8]));
            let _ = container.kill();
            capacity_tracker.release(allocated_cores, allocated_memory_bytes);
            let _ = std::fs::remove_dir_all(&paths.tmp_base);
            return record_and_respond(command, store, stream_id, signer, controllers,
                false, "duplicate_work_id", Vec::new(), None);
        }
    }

    // === Background thread: owns the container handle, does ALL cleanup ===
    //
    // Concurrency design:
    // - This thread exclusively owns `container`, `capacity_tracker` release,
    //   and bundle directory cleanup. No other path touches them.
    // - `terminate()` only signals kill (via AtomicBool + kill syscall).
    //   It does NOT release resources or remove the registry entry.
    // - After the container exits, this thread releases resources, records
    //   accounting, cleans up, and unregisters from the registry.
    // - This guarantees exactly-once cleanup regardless of exit path.
    let running_workloads_bg = Arc::clone(running_workloads);
    let capacity_tracker_bg = Arc::clone(capacity_tracker);
    let provider_id_bg = provider_id;
    let requester_id_bg = requester_id_bytes;
    let delegation_hash_bg = delegation_hash;
    let cpu_multiplier_bg = cert.cpu_core_multiplier();
    let memory_multiplier_bg = cert.memory_multiplier();
    let storage_multiplier_bg = cert.storage_multiplier();
    let cert_digest_bg = cert.digest;

    // Clone values needed for panic-safe cleanup (moved into catch_unwind below)
    let cleanup_capacity = Arc::clone(&capacity_tracker_bg);
    let cleanup_paths = paths.clone();
    let cleanup_registry = Arc::clone(&running_workloads_bg);
    let cleanup_allocated_cores = allocated_cores;
    let cleanup_allocated_memory = allocated_memory_bytes;
    let cleanup_work_id = work_id;
    let image_str_for_response = image_str.clone();

    std::thread::Builder::new()
        .name(paths.thread_name.clone())
        .spawn(move || {
            // Catch panics so they are logged rather than silently killing the thread
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                workload_thread_body(
                    work_id, container, container_pid,
                    running_workloads_bg, capacity_tracker_bg,
                    provider_id_bg, requester_id_bg, delegation_hash_bg,
                    cpu_multiplier_bg, memory_multiplier_bg, storage_multiplier_bg,
                    cert_digest_bg, allocated_cores, allocated_memory_bytes,
                    image_str, paths, workload_class,
                )
            }));
            if let Err(panic) = result {
                let msg = if let Some(s) = panic.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown panic".to_string()
                };
                edgerun_log::error!("work {} thread panicked: {}",
                    edgerun_core::util::bytes_to_hex(&cleanup_work_id[..8]), msg);
                // Best-effort cleanup even on panic
                cleanup_capacity.release(cleanup_allocated_cores, cleanup_allocated_memory);
                let _ = std::fs::remove_dir_all(&cleanup_paths.tmp_base);
                cleanup_registry.unregister(&cleanup_work_id);
            }
        })
        .expect("failed to spawn workload thread");

    edgerun_log::info!("work {} started (pid {}, {} running)",
        edgerun_core::util::bytes_to_hex(&work_id[..8]),
        container_pid,
        running_workloads.count());

    // === IMMEDIATE RESPONSE: workload accepted, running asynchronously ===
    let response = format!("accepted {} (pid {}, work {})",
        image_str_for_response,
        container_pid,
        edgerun_core::util::bytes_to_hex(&work_id[..8])).into_bytes();

    record_and_respond(command, store, stream_id, signer, controllers,
        true, "", response, None)
}

// ---------------------------------------------------------------------------
// Workload background thread body (extracted for panic safety)
// ---------------------------------------------------------------------------

/// The body of the background workload thread. Runs after the container is started.
/// Waits for container exit, records accounting, and performs cleanup.
/// This is extracted into a function so it can be wrapped in `catch_unwind`.
#[cfg(feature = "oci")]
fn workload_thread_body(
    work_id: [u8; 32],
    container: edgerun_oci_runtime::RunningContainer,
    container_pid: u32,
    running_workloads_bg: Arc<super::running_workloads::RunningWorkloads>,
    capacity_tracker_bg: Arc<super::capacity::ResourceTracker>,
    provider_id_bg: [u8; 64],
    requester_id_bg: [u8; 64],
    delegation_hash_bg: [u8; 32],
    cpu_multiplier_bg: edgerun_core::fixed_point::FixedPoint16,
    memory_multiplier_bg: edgerun_core::fixed_point::FixedPoint16,
    storage_multiplier_bg: edgerun_core::fixed_point::FixedPoint16,
    cert_digest_bg: [u8; 32],
    allocated_cores: u32,
    allocated_memory_bytes: u64,
    _image_str: String,
    paths: WorkloadPaths,
    workload_class: edgerun_core::accounting::WorkloadClass,
) {
    use edgerun_core::accounting::{WorkPriority, WorkStatus};

    let start_monotonic = std::time::Instant::now();
    let start_time_us = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(std::time::Duration::ZERO)
        .as_micros() as u64;

    // Await container exit (blocking). The container will be killed
    // if terminate() was called — the RunningContainer::kill() path
    // in the registry sets the kill flag and sends SIGTERM/SIGKILL,
    // but this thread still does the wait() and cleanup.
    let run_result = container.wait();

    let elapsed_us = start_monotonic.elapsed().as_micros() as u64;
    let completed_at_us = start_time_us + elapsed_us;

    let exit_code_val = match &run_result {
        Ok(st) => st.code(),
        Err(_) => None,
    };
    let status = match &run_result {
        Ok(st) if st.success() => WorkStatus::Completed,
        _ => WorkStatus::Failed,
    };

    // Physical resource calculations
    let physical_core_us = (allocated_cores as u64) * elapsed_us;
    let physical_memory_gb = (allocated_memory_bytes / (1024 * 1024 * 1024)) as u32;
    let memory_duration_seconds = (elapsed_us / 1_000_000) as u32;

    // Billable RC-µs
    let billable_compute_rc_us = cpu_multiplier_bg.mul_u64(physical_core_us);

    // Build the accounting record
    let _accounting = edgerun_core::accounting::WorkAccounting {
        work_id,
        requester_id: requester_id_bg,
        provider_id: provider_id_bg,
        delegation_hash: delegation_hash_bg,
        started_at_us: start_time_us,
        completed_at_us,
        physical_core_us,
        physical_memory_gb,
        memory_duration_seconds,
        gpu_core_us: 0,
        npu_core_us: 0,
        storage_read_bytes: 0,
        storage_written_bytes: 0,
        storage_read_ops: 0,
        storage_write_ops: 0,
        network_sent_bytes: 0,
        network_received_bytes: 0,
        workload_class,
        priority: WorkPriority::Standard,
        status,
        exit_code: exit_code_val,
        provider_cert_digest: cert_digest_bg,
        cpu_multiplier: cpu_multiplier_bg,
        memory_multiplier: memory_multiplier_bg,
        storage_multiplier: storage_multiplier_bg,
        billable_compute_rc_us,
    };

    // === ALL cleanup happens here — exactly once ===

    // 1. Log the accounting record
    let status_str = status.as_str();
    let elapsed_s = elapsed_us / 1_000_000;
    edgerun_log::info!("work {} {} ({} RC-µs, {}s, pid {})",
        edgerun_core::util::bytes_to_hex(&work_id[..8]),
        status_str,
        billable_compute_rc_us,
        elapsed_s,
        container_pid);

    // 2. Release reserved resources (exactly once — terminate() never does this)
    capacity_tracker_bg.release(allocated_cores, allocated_memory_bytes);

    // 3. Cleanup bundle directory
    let _ = std::fs::remove_dir_all(&paths.tmp_base);

    // 4. Unregister from the running workloads registry
    // This must happen last — until this point, terminate() could
    // have been called and waited for the kill to take effect.
    running_workloads_bg.unregister(&work_id);
}

// ---------------------------------------------------------------------------
// TerminateWorkload — preempt/terminate a running container
// ---------------------------------------------------------------------------

/// Dispatches a TerminateWorkload command: looks up the running workload
/// by work_id (from the command payload) and kills it via the registry.
fn dispatch_terminate_workload(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
    capacity_tracker: &std::sync::Arc<crate::capacity::ResourceTracker>,
    running_workloads: &std::sync::Arc<super::running_workloads::RunningWorkloads>,
) -> CommandDispatchResult {
    // Extract work_id from command payload
    let workload_spec = match &command.payload {
        Some(edgerun_proto::edgerun::v0::stream::command_envelope::Payload::InlinePayload(bytes)) => bytes.clone(),
        _ => {
            return record_and_respond(command, store, stream_id, signer, controllers,
                false, "missing_workload_spec", Vec::new(), None);
        }
    };

    // Parse work_id from payload: either raw 32 bytes or hex string
    let work_id = if workload_spec.len() == 32 {
        let mut id = [0u8; 32];
        id.copy_from_slice(&workload_spec);
        id
    } else {
        // Try hex decode
        let hex_str = String::from_utf8_lossy(&workload_spec);
        match edgerun_core::util::hex_to_bytes(&hex_str) {
            Ok(bytes) if bytes.len() == 32 => {
                let mut id = [0u8; 32];
                id.copy_from_slice(&bytes);
                id
            }
            _ => {
                return record_and_respond(command, store, stream_id, signer, controllers,
                    false, "invalid_work_id", Vec::new(), None);
            }
        }
    };

    // Look up and terminate the workload
    match running_workloads.terminate(&work_id) {
        Some(info) => {
            // DO NOT release capacity here — the background thread owns
            // resource cleanup. Releasing here would cause double-free
            // when the background thread also releases on exit.
            // terminate() only signals kill; the background thread handles
            // capacity release, bundle cleanup, and unregister.

            edgerun_log::info!("work {} terminated (pid {}, {} cores, {} bytes — resources released by background thread)",
                edgerun_core::util::bytes_to_hex(&work_id[..8]),
                info.pid,
                info.allocated_cores,
                info.allocated_memory_bytes);

            let response = format!("terminated work {} (pid {})",
                edgerun_core::util::bytes_to_hex(&work_id[..8]),
                info.pid).into_bytes();

            record_and_respond(command, store, stream_id, signer, controllers,
                true, "", response, None)
        }
        None => {
            edgerun_log::warn!("work {} not found in running workloads",
                edgerun_core::util::bytes_to_hex(&work_id[..8]));
            record_and_respond(command, store, stream_id, signer, controllers,
                false, "work_not_found", Vec::new(), None)
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[cfg(feature = "oci")]
fn parse_workload_spec(spec: &[u8]) -> (String, u32, u64) {
    let s = String::from_utf8_lossy(spec);
    let mut image = s.to_string();
    let mut cores: u32 = 1;
    let mut memory_gb: u64 = 1;

    if let Some(pos) = s.find(":cores=") {
        let rest = &s[pos + 7..];
        let end = rest.find(':').unwrap_or(rest.len());
        if let Ok(c) = rest[..end].parse::<u32>() {
            cores = c.max(1).min(256);
        }
        image = s[..pos].to_string();
    }

    if let Some(pos) = s.find(":memory=") {
        let rest = &s[pos + 8..];
        let end = rest.find(':').unwrap_or(rest.len());
        let mem_str = &rest[..end];
        if let Some(pi) = mem_str.find(char::is_alphabetic) {
            let (num_s, unit) = mem_str.split_at(pi);
            if let Ok(n) = num_s.parse::<u64>() {
                let u = unit.to_uppercase();
                memory_gb = if u.starts_with('T') { n * 1024 }
                    else if u.starts_with('G') { n }
                    else if u.starts_with('M') { n.max(512) / 1024 }
                    else { n };
                if memory_gb == 0 { memory_gb = 1; }
            }
        } else if let Ok(n) = mem_str.parse::<u64>() {
            memory_gb = n;
        }
        if image.len() > pos {
            image = s[..pos].to_string();
        }
    }

    if image.starts_with("container:") {
        image = image["container:".len()..].to_string();
    }

    (image, cores, memory_gb * 1024 * 1024 * 1024)
}

#[cfg(feature = "oci")]
fn pull_with_metering(
    image: &edgerun_oci_registry::ImageRef,
    bundle_dir: &std::path::Path,
    store_dir: &std::path::Path,
    meter: &super::metering::WorkMeter,
) -> Result<(), String> {
    let mut client = edgerun_oci_registry::RegistryClient::new();

    // Resolve manifest to know expected layer sizes
    let manifest = client.resolve_manifest(image).map_err(|e| e.to_string())?;
    let total_layer_bytes = match &manifest {
        edgerun_oci_registry::ImageManifest::Single(m) => {
            m.layers.iter().map(|l| l.size).sum::<u64>()
        }
        edgerun_oci_registry::ImageManifest::Index(idx) => {
            idx.manifests.first().map(|m| m.size).unwrap_or(0)
        }
    };

    // Pull the image — download_blob() now tracks real bytes in the client
    client.pull(image, bundle_dir, store_dir).map_err(|e| e.to_string())?;

    // Report actual downloaded bytes (may differ from manifest due to retries, compression)
    let real_bytes = client.bytes_downloaded();
    if real_bytes > 0 {
        meter.add_network_received(real_bytes);
    } else {
        // Fallback to manifest estimate if tracking didn't work
        meter.add_network_received(total_layer_bytes);
    }

    // Report the extracted rootfs size as storage write
    if let Ok(sz) = dir_size(&bundle_dir.join("rootfs")) {
        meter.add_storage_write(sz);
    }
    // Report cached layer blobs as storage reads
    if store_dir.exists() {
        if let Ok(sz) = dir_size(store_dir) {
            meter.add_storage_read(sz);
        }
    }
    Ok(())
}

#[cfg(feature = "oci")]
fn dir_size(path: &std::path::Path) -> std::io::Result<u64> {
    let mut total = 0u64;
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let e = entry?;
            let md = e.metadata()?;
            if md.is_file() { total += md.len(); }
            else if md.is_dir() { total += dir_size(&e.path())?; }
        }
    }
    Ok(total)
}

#[cfg(feature = "oci")]
fn load_cached_cert(store: &NodeStore) -> Option<edgerun_core::accounting::PerformanceCertificate> {
    let p = store.data_root().join("perf_cert.bin");
    if let Ok(data) = std::fs::read(&p) {
        edgerun_core::accounting::PerformanceCertificate::from_bytes(&data)
    } else {
        None
    }
}

#[cfg(feature = "oci")]
fn cache_cert(store: &mut NodeStore, cert: &edgerun_core::accounting::PerformanceCertificate, signer: &dyn MeshSigner) {
    // Sign the certificate if it's not already signed
    let mut cert = cert.clone();
    if cert.signature == [0u8; 64] {
        let mut digest_32 = [0u8; 32];
        digest_32.copy_from_slice(&cert.digest);
        if let Ok(sig) = signer.sign_digest(&digest_32) {
            cert.signature = sig;
        } else {
            edgerun_log::warn!("failed to sign performance certificate");
        }
    }

    // Persist to disk for fast loading
    let p = store.data_root().join("perf_cert.bin");
    if std::fs::write(&p, cert.to_bytes()).is_ok() {
        edgerun_log::info!("perf cert cached: cpu {:.2}x ref",
            cert.cpu_core_multiplier().to_raw() as f64 / 65536.0);
    }

    // Also store as an object so it's recoverable from the event stream
    // via the command that triggered the re-benchmark.
    let cert_bytes = cert.to_bytes();
    if let Err(e) = store.put_object(&cert_bytes, 0, &[]) {
        edgerun_log::warn!("failed to store cert as object: {}", e);
    }
}

// ===========================================================================
// Workload spec normalization for deterministic work IDs
// ===========================================================================

/// Normalize a workload spec string so that logically equivalent specs
/// produce the same work ID regardless of key ordering.
///
/// E.g., `container:alpine:cores=4:memory=8G` and
///       `container:alpine:memory=8G:cores=4` → same normalized form
///
/// Strategy: extract image, cores, memory → sort key-value pairs → reassemble.
#[cfg(feature = "oci")]
fn normalize_workload_spec(spec: &[u8]) -> Vec<u8> {
    let (image, cores, memory_bytes) = parse_workload_spec(spec);

    // Normalize memory back to human form for the key
    let memory_gb = memory_bytes / (1024 * 1024 * 1024);
    let mut parts = vec![image];

    // Build sortable key-value pairs
    let mut kvs: Vec<(String, String)> = Vec::new();
    kvs.push(("cores".to_string(), cores.to_string()));
    kvs.push(("memory".to_string(), format!("{}G", memory_gb)));
    kvs.sort_by_key(|(k, _)| k.clone());

    for (k, v) in kvs {
        parts.push(format!("{}={}", k, v));
    }

    parts.join(":").into_bytes()
}

// ===========================================================================
// Certificate freshness check
// ===========================================================================

/// Maximum age for a performance certificate before re-benchmarking is required.
#[cfg(feature = "oci")]
const MAX_CERT_AGE_US: u64 = 24 * 60 * 60 * 1_000_000; // 24 hours

/// Check if a performance certificate is still fresh.
#[cfg(feature = "oci")]
fn check_cert_freshness_impl(cert: &edgerun_core::accounting::PerformanceCertificate) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(std::time::Duration::ZERO)
        .as_micros() as u64;
    let age = now.saturating_sub(cert.benchmark_completed_us);
    if age > MAX_CERT_AGE_US {
        return Err(format!("cert is {} hours old (max 24h)", age / 3_600_000_000));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Custom command dispatch (delegation, revocation)
// ---------------------------------------------------------------------------

/// Handles custom commands: delegation records and revocation records.
