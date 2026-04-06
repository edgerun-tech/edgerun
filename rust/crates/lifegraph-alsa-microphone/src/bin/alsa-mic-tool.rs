use lifegraph_alsa_microphone::{discover_alsa_pcms, AlsaMicrophoneBackend};
use lifegraph_microphone::{AudioCaptureRequest, MicrophoneDevice, MicrophoneSampleFormat};

fn usage() {
    eprintln!("usage: alsa-mic-tool list | capture <card> <device> <duration_ms> [rate] [channels]");
}

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("list") => match discover_alsa_pcms() {
            Ok(pcms) => {
                for pcm in pcms {
                    println!(
                        "card={} device={} capture={} playback={} name={}",
                        pcm.card_index, pcm.device_index, pcm.capture, pcm.playback, pcm.name
                    );
                }
            }
            Err(err) => {
                eprintln!("error: {err}");
                std::process::exit(1);
            }
        },
        Some("capture") => {
            let Some(card) = args.next().and_then(|v| v.parse::<u32>().ok()) else {
                usage();
                std::process::exit(2);
            };
            let Some(device) = args.next().and_then(|v| v.parse::<u32>().ok()) else {
                usage();
                std::process::exit(2);
            };
            let Some(duration_ms) = args.next().and_then(|v| v.parse::<u32>().ok()) else {
                usage();
                std::process::exit(2);
            };
            let rate = args.next().and_then(|v| v.parse::<u32>().ok()).unwrap_or(48_000);
            let channels = args.next().and_then(|v| v.parse::<u16>().ok()).unwrap_or(2);
            let Some(pcm) = discover_alsa_pcms().ok().and_then(|v| v.into_iter().find(|p| p.card_index == card && p.device_index == device)) else {
                eprintln!("PCM hw:{},{} not found", card, device);
                std::process::exit(1);
            };
            let device_path = format!("/dev/snd/pcmC{}D{}c", pcm.card_index, pcm.device_index);
            let mut backend = AlsaMicrophoneBackend { pcm, device_path };
            match backend.capture_audio(&AudioCaptureRequest {
                duration_ms,
                sample_rate_hz: rate,
                channels,
                format: MicrophoneSampleFormat::PcmS16Le,
            }) {
                Ok(capture) => println!(
                    "captured bytes={} rate={} channels={} started_at={}",
                    capture.bytes.len(), capture.sample_rate_hz, capture.channels, capture.started_at_unix_ms
                ),
                Err(err) => {
                    eprintln!("capture error: {err}");
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
