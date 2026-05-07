use edgerun_alsa_speaker::discover_speakers;
use edgerun_devices::speaker::{AudioPlaybackRequest, SpeakerDevice, SpeakerSampleFormat};

fn usage() {
    eprintln!(
        "usage: alsa-speaker-tool list | level <card> <device> [percent] | tone <card> <device> <duration_ms> [rate] [channels] [hz] [software_gain_percent] [target_output_level_percent]"
    );
}

fn synth_tone(duration_ms: u32, sample_rate_hz: u32, channels: u16, hz: f32) -> Vec<u8> {
    let frames = (u64::from(duration_ms) * u64::from(sample_rate_hz) / 1000) as usize;
    let mut out = Vec::with_capacity(frames * usize::from(channels) * 2);
    for i in 0..frames {
        let t = i as f32 / sample_rate_hz as f32;
        let sample = (t * hz * std::f32::consts::TAU).sin();
        let value = (sample * 0.2 * i16::MAX as f32) as i16;
        for _ in 0..channels {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
    out
}

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("list") => match discover_speakers() {
            Ok(backends) => {
                for backend in backends {
                    let info = backend.speaker_info().unwrap();
                    println!(
                        "card={} device={} name={} rate={} channels={} level_control={}",
                        backend.card_index,
                        backend.device_index,
                        info.display_name,
                        info.default_sample_rate_hz,
                        info.channels,
                        info.supports_output_level_control
                    );
                }
            }
            Err(err) => {
                eprintln!("list error: {err}");
                std::process::exit(1);
            }
        },
        Some("level") => {
            let Some(card_index) = args.next().and_then(|s| s.parse::<u32>().ok()) else {
                eprintln!("missing card");
                std::process::exit(2);
            };
            let Some(device_index) = args.next().and_then(|s| s.parse::<u32>().ok()) else {
                eprintln!("missing device");
                std::process::exit(2);
            };
            let target_percent = args.next().and_then(|s| s.parse::<u8>().ok());
            let backend = discover_speakers().ok().and_then(|devices| {
                devices
                    .into_iter()
                    .find(|d| d.card_index == card_index && d.device_index == device_index)
            });
            let Some(backend) = backend else {
                eprintln!("speaker backend not found");
                std::process::exit(1);
            };
            let result = if let Some(percent) = target_percent {
                backend.set_output_level(percent)
            } else {
                backend.output_level()
            };
            match result {
                Ok(Some(level)) => println!(
                    "level={} min_raw={} max_raw={} muted={}",
                    level.current_percent,
                    level.min_raw_value,
                    level.max_raw_value,
                    level.muted.unwrap_or(false)
                ),
                Ok(None) => {
                    eprintln!("level control unavailable");
                    std::process::exit(1);
                }
                Err(err) => {
                    eprintln!("level error: {err}");
                    std::process::exit(1);
                }
            }
        }
        Some("tone") => {
            let Some(card_index) = args.next().and_then(|s| s.parse::<u32>().ok()) else {
                eprintln!("missing card");
                std::process::exit(2);
            };
            let Some(device_index) = args.next().and_then(|s| s.parse::<u32>().ok()) else {
                eprintln!("missing device");
                std::process::exit(2);
            };
            let Some(duration_ms) = args.next().and_then(|s| s.parse::<u32>().ok()) else {
                eprintln!("missing duration_ms");
                std::process::exit(2);
            };
            let sample_rate_hz = args
                .next()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(48_000);
            let channels = args.next().and_then(|s| s.parse::<u16>().ok()).unwrap_or(2);
            let hz = args
                .next()
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(440.0);
            let software_gain_percent = args.next().and_then(|s| s.parse::<u16>().ok());
            let target_output_level_percent = args.next().and_then(|s| s.parse::<u8>().ok());
            let backend = discover_speakers().ok().and_then(|devices| {
                devices
                    .into_iter()
                    .find(|d| d.card_index == card_index && d.device_index == device_index)
            });
            let Some(backend) = backend else {
                eprintln!("speaker backend not found");
                std::process::exit(1);
            };
            let audio_bytes = synth_tone(duration_ms, sample_rate_hz, channels, hz);
            let request = AudioPlaybackRequest {
                duration_ms,
                sample_rate_hz,
                channels,
                format: SpeakerSampleFormat::PcmS16Le,
                audio_bytes,
                software_gain_percent,
                target_output_level_percent,
            };
            match backend.play_audio(&request) {
                Ok(result) => println!(
                    "played bytes={} rate={} channels={} finished={}",
                    result.bytes_written, result.sample_rate_hz, result.channels, result.finished
                ),
                Err(err) => {
                    eprintln!("tone error: {err}");
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
