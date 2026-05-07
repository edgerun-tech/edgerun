//! ESP32-S3 Wi-Fi/AP driver boundary.
//!
//! The hardware-facing side is deliberately a tiny raw 802.11 radio trait.
//! That lets the first implementation use Espressif symbols as an oracle while
//! the AP/MAC and Ethernet bridge logic remains Edgerun-owned and testable.

use edgerun_protocols::ieee80211::{ApEvent, MacAddr, OpenAp, OpenApConfig, OpenApError};

const RAW_80211_MAX: usize = 2352;
const ETHERNET_MAX: usize = 1514;
const RX_ETH_QUEUE: usize = 4;

pub trait Esp32s3WifiRadio {
    fn start_open_ap(&mut self, config: &OpenApConfig) -> bool;
    fn send_raw_80211(&mut self, frame: &[u8]) -> bool;
    fn recv_raw_80211(&mut self, out: &mut [u8]) -> Option<usize>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Esp32s3WifiError {
    RadioStartFailed,
    RadioTxFailed,
    Ap(OpenApError),
}

impl From<OpenApError> for Esp32s3WifiError {
    fn from(value: OpenApError) -> Self {
        Self::Ap(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Esp32s3WifiStats {
    pub raw_rx: u32,
    pub raw_tx: u32,
    pub eth_rx: u32,
    pub eth_tx: u32,
    pub dropped_eth_rx: u32,
    pub last_event: Option<ApEvent>,
}

#[derive(Clone, Copy)]
struct EthFrame {
    used: bool,
    len: usize,
    bytes: [u8; ETHERNET_MAX],
}

impl EthFrame {
    const fn empty() -> Self {
        Self {
            used: false,
            len: 0,
            bytes: [0; ETHERNET_MAX],
        }
    }
}

pub struct Esp32s3WifiOpenAp<R: Esp32s3WifiRadio, const MAX_STATIONS: usize> {
    radio: R,
    ap: OpenAp<MAX_STATIONS>,
    rx_eth: [EthFrame; RX_ETH_QUEUE],
    rx_head: usize,
    rx_tail: usize,
    stats: Esp32s3WifiStats,
}

impl<R: Esp32s3WifiRadio, const MAX_STATIONS: usize> Esp32s3WifiOpenAp<R, MAX_STATIONS> {
    pub const fn new(radio: R, config: OpenApConfig) -> Self {
        Self {
            radio,
            ap: OpenAp::new(config),
            rx_eth: [EthFrame::empty(); RX_ETH_QUEUE],
            rx_head: 0,
            rx_tail: 0,
            stats: Esp32s3WifiStats {
                raw_rx: 0,
                raw_tx: 0,
                eth_rx: 0,
                eth_tx: 0,
                dropped_eth_rx: 0,
                last_event: None,
            },
        }
    }

    pub fn start(&mut self) -> Result<(), Esp32s3WifiError> {
        if !self.radio.start_open_ap(self.ap.config()) {
            return Err(Esp32s3WifiError::RadioStartFailed);
        }
        let mut beacon = [0; 256];
        let len = self.ap.build_beacon(&mut beacon)?;
        if !self.radio.send_raw_80211(&beacon[..len]) {
            return Err(Esp32s3WifiError::RadioTxFailed);
        }
        self.stats.raw_tx = self.stats.raw_tx.wrapping_add(1);
        Ok(())
    }

    pub fn poll(&mut self) {
        let mut raw = [0; RAW_80211_MAX];
        while let Some(len) = self.radio.recv_raw_80211(&mut raw) {
            self.stats.raw_rx = self.stats.raw_rx.wrapping_add(1);
            let mut raw_tx = [0; RAW_80211_MAX];
            let mut eth = [0; ETHERNET_MAX];
            match self.ap.handle_frame(&raw[..len], &mut raw_tx, &mut eth) {
                Ok(action) => {
                    self.stats.last_event = action.event;
                    if action.raw_tx_len != 0
                        && self.radio.send_raw_80211(&raw_tx[..action.raw_tx_len])
                    {
                        self.stats.raw_tx = self.stats.raw_tx.wrapping_add(1);
                    }
                    if let Some(ApEvent::Data { len, .. }) = action.event {
                        self.enqueue_eth(&eth[..len]);
                    }
                }
                Err(OpenApError::UnsupportedFrame) => {}
                Err(error) => {
                    self.stats.last_event = None;
                    let _ = error;
                }
            }
        }
    }

    pub fn send_ethernet(&mut self, frame: &[u8]) -> bool {
        if frame.len() > ETHERNET_MAX || frame.len() < 14 {
            return false;
        }
        let dst = MacAddr::new([frame[0], frame[1], frame[2], frame[3], frame[4], frame[5]]);
        let mut raw = [0; RAW_80211_MAX];
        let Ok(len) = self.ap.encapsulate_ethernet(frame, dst, &mut raw) else {
            return false;
        };
        if !self.radio.send_raw_80211(&raw[..len]) {
            return false;
        }
        self.stats.raw_tx = self.stats.raw_tx.wrapping_add(1);
        self.stats.eth_tx = self.stats.eth_tx.wrapping_add(1);
        true
    }

    pub fn recv_ethernet(&mut self, out: &mut [u8]) -> Option<usize> {
        if self.rx_head == self.rx_tail && !self.rx_eth[self.rx_tail].used {
            return None;
        }
        let frame = &mut self.rx_eth[self.rx_tail];
        if out.len() < frame.len {
            return None;
        }
        let len = frame.len;
        out[..len].copy_from_slice(&frame.bytes[..len]);
        frame.used = false;
        frame.len = 0;
        self.rx_tail = (self.rx_tail + 1) % RX_ETH_QUEUE;
        Some(len)
    }

    pub fn stats(&self) -> Esp32s3WifiStats {
        self.stats
    }

    pub fn radio_mut(&mut self) -> &mut R {
        &mut self.radio
    }

    fn enqueue_eth(&mut self, frame: &[u8]) {
        if frame.len() > ETHERNET_MAX || self.rx_eth[self.rx_head].used {
            self.stats.dropped_eth_rx = self.stats.dropped_eth_rx.wrapping_add(1);
            return;
        }
        let slot = &mut self.rx_eth[self.rx_head];
        slot.bytes[..frame.len()].copy_from_slice(frame);
        slot.len = frame.len();
        slot.used = true;
        self.rx_head = (self.rx_head + 1) % RX_ETH_QUEUE;
        self.stats.eth_rx = self.stats.eth_rx.wrapping_add(1);
    }
}

/// Placeholder radio object for the current no-blob image.
///
/// Replacing this with a symbol-backed or MMIO-backed implementation is the
/// next hardware step. The AP bridge above should not need to change.
pub struct NoRadio;

impl Esp32s3WifiRadio for NoRadio {
    fn start_open_ap(&mut self, _config: &OpenApConfig) -> bool {
        false
    }

    fn send_raw_80211(&mut self, _frame: &[u8]) -> bool {
        false
    }

    fn recv_raw_80211(&mut self, _out: &mut [u8]) -> Option<usize> {
        None
    }
}
