use lifegraph_audio_calibration::{run_speaker_mic_sweep, AudioSweepConfig};

fn usage() {
    eprintln!("usage: audio-calibration-tool sweep <speaker_card> <speaker_device> <mic_card> <mic_device> <start_level> <end_level> <step> <duration_ms> [rate] [channels] [hz] [software_gain_percent] [lead_in_ms]");
}

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("sweep") => {
            let Some(speaker_card) = args.next().and_then(|v| v.parse::<u32>().ok()) else {
                usage();
                std::process::exit(2);
            };
            let Some(speaker_device) = args.next().and_then(|v| v.parse::<u32>().ok()) else {
                usage();
                std::process::exit(2);
            };
            let Some(microphone_card) = args.next().and_then(|v| v.parse::<u32>().ok()) else {
                usage();
                std::process::exit(2);
            };
            let Some(microphone_device) = args.next().and_then(|v| v.parse::<u32>().ok()) else {
                usage();
                std::process::exit(2);
            };
            let Some(start_level_percent) = args.next().and_then(|v| v.parse::<u8>().ok()) else {
                usage();
                std::process::exit(2);
            };
            let Some(end_level_percent) = args.next().and_then(|v| v.parse::<u8>().ok()) else {
                usage();
                std::process::exit(2);
            };
            let Some(step_percent) = args.next().and_then(|v| v.parse::<u8>().ok()) else {
                usage();
                std::process::exit(2);
            };
            let Some(duration_ms) = args.next().and_then(|v| v.parse::<u32>().ok()) else {
                usage();
                std::process::exit(2);
            };
            let sample_rate_hz = args
                .next()
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(48_000);
            let channels = args.next().and_then(|v| v.parse::<u16>().ok()).unwrap_or(2);
            let tone_hz = args
                .next()
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(660.0);
            let software_gain_percent = args
                .next()
                .and_then(|v| v.parse::<u16>().ok())
                .unwrap_or(35);
            let lead_in_ms = args
                .next()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(25);
            let config = AudioSweepConfig {
                speaker_card,
                speaker_device,
                microphone_card,
                microphone_device,
                start_level_percent,
                end_level_percent,
                step_percent,
                duration_ms,
                sample_rate_hz,
                channels,
                tone_hz,
                software_gain_percent,
                lead_in_ms,
            };
            match run_speaker_mic_sweep(&config) {
                Ok(results) => {
                    for step in results {
                        let clipped_percent = if step.total_samples > 0 {
                            (step.clipped_samples as f32 * 100.0) / step.total_samples as f32
                        } else {
                            0.0
                        };
                        println!(
                            "requested={} applied={} rms_dbfs={:.2} peak_dbfs={:.2} clipped_samples={} clipped_percent={:.3}",
                            step.requested_level_percent,
                            step.applied_level_percent.map(|v| v.to_string()).unwrap_or_else(|| "n/a".into()),
                            step.rms_dbfs,
                            step.peak_dbfs,
                            step.clipped_samples,
                            clipped_percent,
                        );
                    }
                }
                Err(err) => {
                    eprintln!("sweep error: {err}");
                    std::process::exit(1);
                }
            }
        }
        _ => {
            usage();
            std::process::exit(2);
        }
    }
}
