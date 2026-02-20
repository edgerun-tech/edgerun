// SPDX-License-Identifier: GPL-2.0-only
use std::path::PathBuf;
use std::time::Instant;

use rand::RngCore;
use edgerun_storage::durability::DurabilityLevel;
use edgerun_storage::encryption::{verify_encrypted_segment_bytes, EncryptionMode};
use edgerun_storage::event::{ActorId, Event, StreamId};
use edgerun_storage::key_management::{
    derive_wrapping_key_from_material, EnvKeyProvider, KeyProvider, PassphraseKeyProvider,
    WrappedFileKeyProvider,
};
use edgerun_storage::manifest::ManifestManager;
use edgerun_storage::segment::SegmentReader;
use edgerun_storage::StorageEngine;

#[derive(Debug, Clone)]
enum Provider {
    Env {
        var_name: String,
    },
    Passphrase {
        passphrase: String,
    },
    WrappedFile {
        wrapped_path: PathBuf,
        wrapping_material: String,
    },
}

#[derive(Debug, Clone)]
struct Config {
    data_dir: PathBuf,
    segment_file: String,
    events: usize,
    payload_size: usize,
    key_epoch: u32,
    chunk_size: usize,
    verify_readback: bool,
    no_encryption: bool,
    durability: DurabilityLevel,
    provider: Provider,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("/tmp/storage_encrypted_demo"),
            segment_file: "encrypted.seg".to_string(),
            events: 1000,
            payload_size: 256,
            key_epoch: 1,
            chunk_size: 4096,
            verify_readback: true,
            no_encryption: false,
            durability: DurabilityLevel::AckLocal,
            provider: Provider::Passphrase {
                passphrase: "change-me".to_string(),
            },
        }
    }
}

fn parse_args() -> Config {
    let mut cfg = Config::default();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let mut i = 0usize;
    while i < args.len() {
        let arg = &args[i];
        let next = args.get(i + 1);
        match (arg.as_str(), next) {
            ("--data-dir", Some(v)) => {
                cfg.data_dir = PathBuf::from(v);
                i += 2;
            }
            ("--segment-file", Some(v)) => {
                cfg.segment_file = v.to_string();
                i += 2;
            }
            ("--events", Some(v)) => {
                cfg.events = v.parse().unwrap_or(cfg.events);
                i += 2;
            }
            ("--payload-size", Some(v)) => {
                cfg.payload_size = v.parse().unwrap_or(cfg.payload_size);
                i += 2;
            }
            ("--key-epoch", Some(v)) => {
                cfg.key_epoch = v.parse().unwrap_or(cfg.key_epoch);
                i += 2;
            }
            ("--chunk-size", Some(v)) => {
                cfg.chunk_size = v.parse().unwrap_or(cfg.chunk_size);
                i += 2;
            }
            ("--skip-verify", _) => {
                cfg.verify_readback = false;
                i += 1;
            }
            ("--no-encryption", _) => {
                cfg.no_encryption = true;
                i += 1;
            }
            ("--durability", Some(v)) => {
                cfg.durability = match v.as_str() {
                    "ack-local" => DurabilityLevel::AckLocal,
                    "ack-durable" => DurabilityLevel::AckDurable,
                    _ => cfg.durability,
                };
                i += 2;
            }
            ("--provider", Some(v)) => {
                match v.as_str() {
                    "env" => {
                        cfg.provider = Provider::Env {
                            var_name: "ERFS_STORE_KEY_HEX".to_string(),
                        };
                    }
                    "passphrase" => {
                        cfg.provider = Provider::Passphrase {
                            passphrase: "change-me".to_string(),
                        };
                    }
                    "wrapped-file" => {
                        cfg.provider = Provider::WrappedFile {
                            wrapped_path: PathBuf::from("/tmp/erfs_wrapped.key"),
                            wrapping_material: "change-me-wrap".to_string(),
                        };
                    }
                    _ => {}
                }
                i += 2;
            }
            ("--env-var", Some(v)) => {
                cfg.provider = Provider::Env {
                    var_name: v.to_string(),
                };
                i += 2;
            }
            ("--passphrase", Some(v)) => {
                cfg.provider = Provider::Passphrase {
                    passphrase: v.to_string(),
                };
                i += 2;
            }
            ("--wrapped-path", Some(v)) => {
                let wrapping_material = match &cfg.provider {
                    Provider::WrappedFile {
                        wrapping_material, ..
                    } => wrapping_material.clone(),
                    _ => "change-me-wrap".to_string(),
                };
                cfg.provider = Provider::WrappedFile {
                    wrapped_path: PathBuf::from(v),
                    wrapping_material,
                };
                i += 2;
            }
            ("--wrapping-material", Some(v)) => {
                let wrapped_path = match &cfg.provider {
                    Provider::WrappedFile { wrapped_path, .. } => wrapped_path.clone(),
                    _ => PathBuf::from("/tmp/erfs_wrapped.key"),
                };
                cfg.provider = Provider::WrappedFile {
                    wrapped_path,
                    wrapping_material: v.to_string(),
                };
                i += 2;
            }
            ("--help", _) | ("-h", _) => {
                print_help_and_exit();
            }
            _ => {
                i += 1;
            }
        }
    }
    cfg
}

fn print_help_and_exit() -> ! {
    println!(
        "Usage: encrypted_append_demo [--data-dir PATH] [--segment-file NAME] [--events N] [--payload-size N] [--key-epoch N] [--chunk-size N] [--durability ack-local|ack-durable] [--provider env|passphrase|wrapped-file] [--env-var NAME] [--passphrase VALUE] [--wrapped-path PATH] [--wrapping-material TEXT] [--skip-verify] [--no-encryption]"
    );
    std::process::exit(0);
}

fn main() {
    let cfg = parse_args();
    let _ = std::fs::remove_dir_all(&cfg.data_dir);
    std::fs::create_dir_all(&cfg.data_dir).expect("create data dir");

    let engine = StorageEngine::new(cfg.data_dir.clone()).expect("create engine");
    let manifest = ManifestManager::new(cfg.data_dir.clone()).expect("create manifest manager");
    let mut store_uuid = [0u8; 16];
    store_uuid.copy_from_slice(manifest.store_uuid().expect("read store uuid").as_bytes());

    let store_key = match &cfg.provider {
        Provider::Env { var_name } => EnvKeyProvider::new(var_name.clone())
            .load_store_key(store_uuid)
            .expect("load store key from env"),
        Provider::Passphrase { passphrase } => PassphraseKeyProvider::new(passphrase.clone())
            .load_store_key(store_uuid)
            .expect("derive store key from passphrase"),
        Provider::WrappedFile {
            wrapped_path,
            wrapping_material,
        } => {
            let wrap_key = derive_wrapping_key_from_material(
                wrapping_material.as_bytes(),
                b"erfs/demo/wrapping",
            );
            if !wrapped_path.exists() {
                // bootstrap wrapped key file for demo usage
                let mut key = [0u8; 32];
                rand::rngs::OsRng.fill_bytes(&mut key);
                WrappedFileKeyProvider::write_wrapped_key(wrapped_path, key, wrap_key)
                    .expect("write wrapped key");
            }
            WrappedFileKeyProvider::new(wrapped_path.clone(), wrap_key)
                .load_store_key(store_uuid)
                .expect("load wrapped store key")
        }
    };

    let mut session = engine
        .create_append_session(&cfg.segment_file, 128 * 1024 * 1024)
        .expect("create session");
    if !cfg.no_encryption {
        session
            .enable_encryption(
                store_key,
                cfg.key_epoch,
                EncryptionMode::PayloadOnly,
                cfg.chunk_size,
            )
            .expect("enable encryption");
    }

    let stream = StreamId::new();
    let actor = ActorId::new();
    let start = Instant::now();
    for i in 0..cfg.events {
        let payload = vec![(i % 251) as u8; cfg.payload_size];
        let event = Event::new(stream.clone(), actor.clone(), payload);
        session
            .append_with_durability(&event, cfg.durability)
            .expect("append");
    }
    session
        .flush(DurabilityLevel::AckDurable)
        .expect("final flush");
    let elapsed = start.elapsed();

    if cfg.verify_readback {
        let segment_path = cfg.data_dir.join(&cfg.segment_file);
        let reader = if cfg.no_encryption {
            SegmentReader::from_file(segment_path).expect("open segment")
        } else {
            let encrypted = std::fs::read(&segment_path).expect("read segment");
            verify_encrypted_segment_bytes(&encrypted).expect("verify encrypted segment");
            SegmentReader::from_encrypted_file(segment_path, store_key, store_uuid)
                .expect("decrypt+open segment")
        };
        let mut event_count = 0usize;
        for event in reader.iter_events() {
            event.expect("decode event");
            event_count += 1;
        }
        assert_eq!(
            event_count, cfg.events,
            "decrypted event count should match writes"
        );
    }

    println!("Encrypted append demo complete.");
    println!("data_dir={}", cfg.data_dir.display());
    println!("segment_file={}", cfg.segment_file);
    println!("events={}", cfg.events);
    println!("payload_size={}", cfg.payload_size);
    println!("encryption_enabled={}", !cfg.no_encryption);
    println!("durability={:?}", cfg.durability);
    println!("verify_readback={}", cfg.verify_readback);
    println!("elapsed_s={:.6}", elapsed.as_secs_f64());
    let total_mb = (cfg.events * cfg.payload_size) as f64 / (1024.0 * 1024.0);
    println!("throughput_mb_s={:.2}", total_mb / elapsed.as_secs_f64());
    println!(
        "events_per_s={:.0}",
        cfg.events as f64 / elapsed.as_secs_f64()
    );
}
