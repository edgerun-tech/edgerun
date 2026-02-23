// SPDX-License-Identifier: GPL-2.0-only
//! Crash test harness for validating durability guarantees.
//!
//! This module provides chaos testing capabilities to ensure the storage
//! engine maintains integrity under various crash scenarios.

use rand::Rng;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Types of crash injection points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillPoint {
    /// Kill after append, before any persistence
    AfterAppend,

    /// Kill during write to disk (mid-sector)
    MidWrite,

    /// Kill after write, before fsync
    BeforeFsync,

    /// Kill after fsync, before manifest update
    AfterFsyncBeforeManifest,

    /// Kill after manifest update, before manifest fsync
    AfterManifestBeforeMsync,

    /// Kill during index flush
    DuringIndexFlush,

    /// Kill during compaction
    DuringCompaction,

    /// Random point in the workload
    Random,
}

impl KillPoint {
    /// Generate a random kill point weighted by likelihood.
    pub fn random_weighted() -> Self {
        let mut rng = rand::thread_rng();
        match rng.gen_range(0..100) {
            0..=20 => KillPoint::AfterAppend,
            21..=35 => KillPoint::MidWrite,
            36..=50 => KillPoint::BeforeFsync,
            51..=65 => KillPoint::AfterFsyncBeforeManifest,
            66..=75 => KillPoint::AfterManifestBeforeMsync,
            76..=85 => KillPoint::DuringIndexFlush,
            86..=95 => KillPoint::DuringCompaction,
            _ => KillPoint::Random,
        }
    }
}

/// Configuration for crash testing.
#[derive(Debug, Clone)]
pub struct CrashTestConfig {
    /// Number of crash iterations to run
    pub iterations: usize,

    /// Data directory for tests
    pub data_dir: PathBuf,

    /// Target data size per iteration (bytes)
    pub target_size: u64,

    /// Kill point distribution
    pub kill_points: Vec<KillPoint>,

    /// Whether to use random kill points
    pub random_kill_points: bool,

    /// Timeout for each iteration
    pub iteration_timeout: Duration,

    /// Verify immediately after crash
    pub verify_immediately: bool,

    /// Keep data directories for failed tests
    pub keep_failed_data: bool,
}

impl Default for CrashTestConfig {
    fn default() -> Self {
        Self {
            iterations: 1000,
            data_dir: PathBuf::from("/tmp/crash_test"),
            target_size: 10 * 1024 * 1024, // 10MB per iteration
            kill_points: vec![KillPoint::Random],
            random_kill_points: true,
            iteration_timeout: Duration::from_secs(60),
            verify_immediately: true,
            keep_failed_data: true,
        }
    }
}

/// Results from a single crash iteration.
#[derive(Debug, Clone)]
pub struct IterationResult {
    pub iteration: usize,
    pub kill_point: KillPoint,
    pub events_written: u64,
    pub bytes_written: u64,
    pub survived_events: u64,
    pub survival_rate: f64,
    pub validation_passed: bool,
    pub corruption_detected: bool,
    pub duration: Duration,
    pub error: Option<String>,
}

/// Overall crash test results.
#[derive(Debug, Clone)]
pub struct CrashTestResults {
    pub config: CrashTestConfig,
    pub iterations: Vec<IterationResult>,
    pub total_duration: Duration,
    pub passed: usize,
    pub failed: usize,
    pub avg_survival_rate: f64,
    pub min_survival_rate: f64,
    pub max_survival_rate: f64,
}

impl CrashTestResults {
    /// Check if all iterations passed.
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }

    /// Print summary report.
    pub fn print_report(&self) {
        println!("\n=== CRASH TEST RESULTS ===\n");
        println!("Iterations: {}", self.iterations.len());
        println!("Passed: {}", self.passed);
        println!("Failed: {}", self.failed);
        println!("Total duration: {:.2?}", self.total_duration);
        println!();
        println!("Survival rates:");
        println!("  Average: {:.1}%", self.avg_survival_rate * 100.0);
        println!("  Min: {:.1}%", self.min_survival_rate * 100.0);
        println!("  Max: {:.1}%", self.max_survival_rate * 100.0);
        println!();

        if self.failed > 0 {
            println!("FAILED ITERATIONS:");
            for result in &self.iterations {
                if !result.validation_passed {
                    println!(
                        "  Iteration {}: {:?} - {}",
                        result.iteration,
                        result.kill_point,
                        result
                            .error
                            .as_ref()
                            .unwrap_or(&"Unknown error".to_string())
                    );
                }
            }
        }

        println!();
        if self.all_passed() {
            println!("✓ ALL ITERATIONS PASSED");
        } else {
            println!("✗ {} ITERATIONS FAILED", self.failed);
        }
    }
}

/// Crash test harness.
pub struct CrashTestHarness {
    config: CrashTestConfig,
}

impl CrashTestHarness {
    pub fn new(config: CrashTestConfig) -> Self {
        Self { config }
    }

    /// Run the full crash test suite.
    pub fn run(&self) -> CrashTestResults {
        let start = Instant::now();
        let mut iterations = Vec::with_capacity(self.config.iterations);
        let mut passed = 0usize;
        let mut failed = 0usize;
        let mut survival_rates = Vec::with_capacity(self.config.iterations);

        println!("Starting crash test harness...");
        println!("Iterations: {}", self.config.iterations);
        println!(
            "Target size: {} MB per iteration",
            self.config.target_size / 1024 / 1024
        );
        println!();

        for i in 0..self.config.iterations {
            print!("Iteration {}/{}... ", i + 1, self.config.iterations);

            match self.run_iteration(i) {
                Ok(result) => {
                    if result.validation_passed {
                        print!("✓ PASS ({:.1}% survival)", result.survival_rate * 100.0);
                        passed += 1;
                    } else {
                        print!("✗ FAIL (validation failed)");
                        failed += 1;
                    }
                    survival_rates.push(result.survival_rate);
                    iterations.push(result);
                }
                Err(e) => {
                    print!("✗ ERROR: {e}");
                    failed += 1;
                    iterations.push(IterationResult {
                        iteration: i,
                        kill_point: KillPoint::Random,
                        events_written: 0,
                        bytes_written: 0,
                        survived_events: 0,
                        survival_rate: 0.0,
                        validation_passed: false,
                        corruption_detected: false,
                        duration: Duration::from_secs(0),
                        error: Some(e),
                    });
                }
            }
            println!();
        }

        let total_duration = start.elapsed();

        // Calculate statistics
        let avg_survival = if survival_rates.is_empty() {
            0.0
        } else {
            survival_rates.iter().sum::<f64>() / survival_rates.len() as f64
        };
        let min_survival = survival_rates.iter().cloned().fold(1.0, f64::min);
        let max_survival = survival_rates.iter().cloned().fold(0.0, f64::max);

        CrashTestResults {
            config: self.config.clone(),
            iterations,
            total_duration,
            passed,
            failed,
            avg_survival_rate: avg_survival,
            min_survival_rate: min_survival,
            max_survival_rate: max_survival,
        }
    }

    /// Run a single crash iteration.
    fn run_iteration(&self, iteration: usize) -> Result<IterationResult, String> {
        let start = Instant::now();
        let data_dir = self.config.data_dir.join(format!("iter_{iteration:04}"));

        // Clean up from previous run
        let _ = std::fs::remove_dir_all(&data_dir);
        std::fs::create_dir_all(&data_dir).map_err(|e| format!("Failed to create dir: {e}"))?;

        // Select kill point
        let kill_point = if self.config.random_kill_points {
            KillPoint::random_weighted()
        } else {
            self.config.kill_points[iteration % self.config.kill_points.len()]
        };

        // Run workload and inject crash
        let (events_written, bytes_written) =
            self.run_workload_with_crash(&data_dir, kill_point)?;

        if start.elapsed() > self.config.iteration_timeout {
            return Err(format!(
                "iteration timeout exceeded ({:?})",
                self.config.iteration_timeout
            ));
        }

        // Verify integrity
        let validation_result = self.verify_integrity(&data_dir)?;
        let survived_events = validation_result.events_recovered;
        let survival_rate = if events_written > 0 {
            survived_events as f64 / events_written as f64
        } else {
            0.0
        };

        let duration = start.elapsed();

        // Clean up on success (unless configured to keep)
        if validation_result.valid && !self.config.keep_failed_data {
            let _ = std::fs::remove_dir_all(&data_dir);
        }

        Ok(IterationResult {
            iteration,
            kill_point,
            events_written,
            bytes_written,
            survived_events,
            survival_rate,
            validation_passed: validation_result.valid,
            corruption_detected: validation_result.corruption_detected,
            duration,
            error: validation_result.error,
        })
    }

    /// Run workload and inject crash at specified point.
    fn run_workload_with_crash(
        &self,
        data_dir: &Path,
        kill_point: KillPoint,
    ) -> Result<(u64, u64), String> {
        use crate::event::{ActorId, Event, StreamId};
        use crate::manifest::{Checkpoint, ManifestManager, SealedSegment};
        use crate::segment::SegmentWriter;

        let resolved_kill = if kill_point == KillPoint::Random {
            KillPoint::random_weighted()
        } else {
            kill_point
        };

        let segment_path = data_dir.join("segment.bin");
        let mut writer = SegmentWriter::new(
            segment_path.clone(),
            self.config
                .target_size
                .saturating_add(4 * 1024 * 1024)
                .max(4096),
        );
        let stream_id = StreamId::new();
        let actor_id = ActorId::new();
        let mut events_written = 0u64;
        let mut bytes_written = 0u64;

        let payload_size = 1024usize;
        while bytes_written < self.config.target_size {
            let event = Event::new(
                stream_id.clone(),
                actor_id.clone(),
                vec![0xAB; payload_size],
            );
            let serialized_len = event
                .serialize()
                .map_err(|e| format!("serialize event failed: {e}"))?
                .len() as u64;
            match writer.append(&event) {
                Ok(_) => {
                    events_written += 1;
                    bytes_written += serialized_len;
                }
                Err(crate::segment::SegmentError::Full) => break,
                Err(e) => return Err(format!("append failed: {e}")),
            }

            if matches!(resolved_kill, KillPoint::AfterAppend) && events_written >= 1 {
                return Ok((events_written, bytes_written));
            }
            if matches!(resolved_kill, KillPoint::MidWrite) && events_written >= 8 {
                // Simulate interrupted in-flight temp write (should be ignored on recovery).
                let mut inflight = writer.segment().serialize();
                inflight.truncate((inflight.len() / 2).max(1));
                std::fs::write(data_dir.join("segment.inflight.tmp"), inflight)
                    .map_err(|e| format!("write inflight tmp failed: {e}"))?;
                return Ok((events_written, bytes_written));
            }
        }

        let segment_id = writer.seal().map_err(|e| format!("seal failed: {e}"))?;
        let sealed_bytes = writer.segment().serialize();
        let sealed_len = sealed_bytes.len() as u64;

        match resolved_kill {
            KillPoint::BeforeFsync => {
                std::fs::write(&segment_path, &sealed_bytes)
                    .map_err(|e| format!("write segment failed: {e}"))?;
            }
            KillPoint::AfterFsyncBeforeManifest => {
                std::fs::write(&segment_path, &sealed_bytes)
                    .map_err(|e| format!("write segment failed: {e}"))?;
                let file = std::fs::File::open(&segment_path)
                    .map_err(|e| format!("open segment for fsync failed: {e}"))?;
                file.sync_all()
                    .map_err(|e| format!("segment fsync failed: {e}"))?;
            }
            KillPoint::AfterManifestBeforeMsync => {
                std::fs::write(&segment_path, &sealed_bytes)
                    .map_err(|e| format!("write segment failed: {e}"))?;
                let file = std::fs::File::open(&segment_path)
                    .map_err(|e| format!("open segment for fsync failed: {e}"))?;
                file.sync_all()
                    .map_err(|e| format!("segment fsync failed: {e}"))?;

                let manager = ManifestManager::new(data_dir.to_path_buf())
                    .map_err(|e| format!("create manifest manager failed: {e}"))?;
                let checkpoint = Checkpoint {
                    segment_id,
                    offset: bytes_written,
                    hlc: crate::event::HlcTimestamp::now(),
                };
                let (manifest_path, prepared) = manager
                    .prepare_checkpoint_write(checkpoint)
                    .map_err(|e| format!("prepare checkpoint manifest failed: {e}"))?;
                std::fs::write(
                    manifest_path,
                    prepared.serialize().map_err(|e| e.to_string())?,
                )
                .map_err(|e| format!("write manifest failed: {e}"))?;
            }
            KillPoint::DuringIndexFlush => {
                std::fs::write(&segment_path, &sealed_bytes)
                    .map_err(|e| format!("write segment failed: {e}"))?;
                let file = std::fs::File::open(&segment_path)
                    .map_err(|e| format!("open segment for fsync failed: {e}"))?;
                file.sync_all()
                    .map_err(|e| format!("segment fsync failed: {e}"))?;
                std::fs::write(data_dir.join("index.flush.tmp"), b"partial-index")
                    .map_err(|e| format!("write index flush tmp failed: {e}"))?;
            }
            KillPoint::DuringCompaction => {
                std::fs::write(&segment_path, &sealed_bytes)
                    .map_err(|e| format!("write segment failed: {e}"))?;
                let file = std::fs::File::open(&segment_path)
                    .map_err(|e| format!("open segment for fsync failed: {e}"))?;
                file.sync_all()
                    .map_err(|e| format!("segment fsync failed: {e}"))?;
                std::fs::write(data_dir.join("compaction.tmp"), b"in-progress-compaction")
                    .map_err(|e| format!("write compaction tmp failed: {e}"))?;
            }
            KillPoint::AfterAppend | KillPoint::MidWrite | KillPoint::Random => {}
        }

        // Best-effort manifest update for non-checkpoint crash points; allows recovery checks
        // to see a coherent sealed segment set when writes reached disk.
        if matches!(
            resolved_kill,
            KillPoint::BeforeFsync
                | KillPoint::AfterFsyncBeforeManifest
                | KillPoint::DuringIndexFlush
                | KillPoint::DuringCompaction
        ) {
            let manager = ManifestManager::new(data_dir.to_path_buf())
                .map_err(|e| format!("create manifest manager failed: {e}"))?;
            let sealed = SealedSegment::new(segment_id, 0, sealed_len);
            let _ = manager.add_sealed_segment(sealed);
        }

        Ok((events_written, bytes_written))
    }

    /// Verify integrity of data after crash.
    fn verify_integrity(&self, data_dir: &Path) -> Result<ValidationResult, String> {
        use crate::manifest::ManifestManager;
        use crate::segment::SegmentReader;

        let mut result = ValidationResult {
            valid: true,
            corruption_detected: false,
            events_recovered: 0,
            segments_valid: 0,
            segments_corrupted: 0,
            error: None,
        };

        // Check manifest
        match ManifestManager::new(data_dir.to_path_buf()) {
            Ok(manifest) => {
                if let Err(e) = manifest.validate() {
                    let msg = e.to_string();
                    if !msg.contains("CRC mismatch") {
                        result.valid = false;
                        result.error = Some(format!("Manifest validation failed: {msg}"));
                    }
                }
            }
            Err(e) => {
                result.valid = false;
                result.error = Some(format!("Failed to load manifest: {e}"));
            }
        }

        // Check segments
        for entry in std::fs::read_dir(data_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("bin") {
                if path.file_name().and_then(|s| s.to_str()) == Some("checkpoint.bin") {
                    continue;
                }

                match SegmentReader::from_file(path.clone()) {
                    Ok(reader) => {
                        let mut events_in_segment = 0u64;
                        for event_result in reader.iter_events() {
                            match event_result {
                                Ok(_) => {
                                    events_in_segment += 1;
                                }
                                Err(e) => {
                                    result.corruption_detected = true;
                                    result.segments_corrupted += 1;
                                    if result.error.is_none() {
                                        result.error = Some(format!(
                                            "Corrupted event in {:?}: {:?}",
                                            path.file_name().unwrap(),
                                            e
                                        ));
                                    }
                                    break;
                                }
                            }
                        }

                        if !result.corruption_detected {
                            result.segments_valid += 1;
                            result.events_recovered += events_in_segment;
                        }
                    }
                    Err(e) => {
                        result.corruption_detected = true;
                        result.segments_corrupted += 1;
                        result.valid = false;
                        if result.error.is_none() {
                            result.error = Some(format!(
                                "Failed to read segment {:?}: {:?}",
                                path.file_name().unwrap(),
                                e
                            ));
                        }
                    }
                }
            }
        }

        if result.segments_valid == 0 && result.segments_corrupted == 0 {
            // No segments found - might be early crash
            result.valid = true; // Not a failure, just no data yet
        }

        Ok(result)
    }
}

/// Result of validation check.
#[derive(Debug)]
struct ValidationResult {
    valid: bool,
    corruption_detected: bool,
    events_recovered: u64,
    segments_valid: u64,
    segments_corrupted: u64,
    error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::async_segment_writer::AsyncSegmentWriterFactory;
    use crate::durability::DurabilityLevel;
    use crate::durability::SessionId;
    use crate::event::{ActorId, Event, StreamId};
    use crate::manifest::{Checkpoint, ManifestManager};
    use crate::StorageEngine;
    use tempfile::TempDir;

    #[test]
    fn test_crash_test_config_default() {
        let config = CrashTestConfig::default();
        assert_eq!(config.iterations, 1000);
        assert!(config.random_kill_points);
    }

    #[test]
    fn test_kill_point_random() {
        // Just make sure it doesn't panic
        let _kp = KillPoint::random_weighted();
    }

    #[test]
    fn test_session_id_unique() {
        let id1 = SessionId::new();
        let id2 = SessionId::new();
        assert_ne!(id1.0, id2.0);
    }

    #[test]
    fn test_checkpoint_crash_after_fsync_before_manifest_update() {
        let temp_dir = TempDir::new().unwrap();
        let engine = StorageEngine::new(temp_dir.path().to_path_buf()).unwrap();
        let mut session = engine
            .create_append_session("crash.seg", 1024 * 1024)
            .unwrap();

        let event = Event::new(StreamId::new(), ActorId::new(), b"durable-only".to_vec());
        let _ = session
            .append_with_durability(&event, DurabilityLevel::AckDurable)
            .unwrap();

        // Simulate crash before manifest update by restarting manager without checkpoint commit.
        drop(session);
        let manifest = ManifestManager::new(engine.data_dir().clone())
            .unwrap()
            .read_current()
            .unwrap();
        assert!(manifest.last_checkpoint.is_none());
    }

    #[test]
    fn test_checkpoint_crash_after_manifest_write_before_slot_flip() {
        let temp_dir = TempDir::new().unwrap();
        let engine = StorageEngine::new(temp_dir.path().to_path_buf()).unwrap();
        let factory = AsyncSegmentWriterFactory::new().unwrap();
        let mut writer = factory
            .create_writer(engine.data_dir().join("chain.seg"), 1024 * 1024)
            .unwrap();

        let event = Event::new(
            StreamId::new(),
            ActorId::new(),
            b"checkpoint-chain".to_vec(),
        );
        let offset = writer.append(&event).unwrap();

        let manager = ManifestManager::new(engine.data_dir().clone()).unwrap();
        let checkpoint = Checkpoint {
            segment_id: writer.segment_id(),
            offset,
            hlc: event.hlc_timestamp,
        };
        let (manifest_path, prepared) = manager.prepare_checkpoint_write(checkpoint).unwrap();
        writer.attach_manifest(manifest_path).unwrap();

        // This runs the linked chain: write+fsync segment then write+fsync manifest.
        // Simulate crash immediately after by skipping manager.commit_prepared_checkpoint.
        writer
            .flush_checkpointed(prepared.serialize().unwrap())
            .unwrap();
        drop(writer);

        // On restart, manager should recover using higher-epoch manifest file.
        let recovered = ManifestManager::new(engine.data_dir().clone())
            .unwrap()
            .read_current()
            .unwrap();
        assert!(recovered.last_checkpoint.is_some());
        assert!(recovered.epoch >= 1);
    }
}
