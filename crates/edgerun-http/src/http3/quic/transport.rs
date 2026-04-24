//! QUIC transport layer (loss recovery, congestion control, flow control)

use super::frame::QuicFrame;
use super::{ConnectionId, PacketNumberSpace, TransportParameters};

/// Packet number state for a single packet number space (RFC 9000 §12.3).
#[derive(Debug, Clone)]
pub struct PacketNumberState {
    /// Next packet number to use for sending
    next: u64,
    /// Largest packet number received in this space (for expansion + dedup)
    largest_received: Option<u64>,
    /// Count of packets sent in this space (for loss tracking)
    packets_sent: u64,
    /// Received packet numbers for ACK generation
    received_packets: Vec<(u64, std::time::Instant)>,
}

impl PacketNumberState {
    fn new() -> Self {
        PacketNumberState {
            next: 0,
            largest_received: None,
            packets_sent: 0,
            received_packets: Vec::new(),
        }
    }

    fn record_received(&mut self, pn: u64) {
        self.received_packets.push((pn, std::time::Instant::now()));
        if self.received_packets.len() > 256 {
            self.received_packets.drain(..128);
        }
    }

    fn clear_up_to(&mut self, ack_through: u64) {
        self.received_packets.retain(|&(pn, _)| pn > ack_through);
    }
}

/// Sent packet metadata for loss detection (RFC 9002).
#[derive(Debug, Clone)]
pub struct SentPacket {
    pub packet_number: u64,
    pub time_sent: std::time::Instant,
    pub size: usize,
    pub has_crypto: bool,
    pub acked: bool,
    pub lost: bool,
}

/// Congestion control state (RFC 9002 §7).
#[derive(Debug, Clone)]
pub struct CongestionControl {
    pub congestion_window: u64,
    pub ssthresh: u64,
    pub bytes_in_flight: u64,
    pub recovery_start_time: Option<std::time::Instant>,
}

impl CongestionControl {
    fn new() -> Self {
        CongestionControl {
            congestion_window: 14720, // 10 * max_datagram_size
            ssthresh: u64::MAX,
            bytes_in_flight: 0,
            recovery_start_time: None,
        }
    }

    pub fn can_send(&self) -> bool {
        self.bytes_in_flight < self.congestion_window
    }

    pub fn available_window(&self) -> u64 {
        self.congestion_window.saturating_sub(self.bytes_in_flight)
    }
}

/// Flow control state for connection and stream level.
#[derive(Debug, Clone)]
pub struct FlowControl {
    pub max_data: u64,
    pub sent: u64,
    pub received: u64,
    pub peer_max_data: u64,
}

impl FlowControl {
    fn new(initial_max: u64) -> Self {
        FlowControl {
            max_data: initial_max,
            sent: 0,
            received: 0,
            peer_max_data: initial_max,
        }
    }

    pub fn can_send(&self, len: usize) -> bool {
        self.sent + len as u64 <= self.peer_max_data
    }

    pub fn record_sent(&mut self, len: usize) {
        self.sent += len as u64;
    }

    pub fn record_received(&mut self, len: usize) {
        self.received += len as u64;
    }

    pub fn remaining_receive(&self) -> u64 {
        self.max_data.saturating_sub(self.received)
    }

    pub fn remaining_send(&self) -> u64 {
        self.peer_max_data.saturating_sub(self.sent)
    }
}

/// QUIC transport connection state
pub struct QuicTransport {
    pub local_cid: ConnectionId,
    pub remote_cid: ConnectionId,
    pub params: TransportParameters,
    /// Packet number state per space (RFC 9000 §12.3)
    pn_state: [PacketNumberState; 3],
    /// Connection-level flow control
    pub max_data: u64,
    /// Stream-level flow control
    pub max_stream_data: u64,
    last_activity: std::time::Instant,
    /// Path MTU
    pub mtu: usize,
    /// RTT estimate (RFC 9002 §9)
    rtt_estimate: std::time::Duration,
    smoothed_rtt: Option<std::time::Duration>,
    rttvar: std::time::Duration,
    min_rtt: Option<std::time::Duration>,
    latest_rtt: Option<std::time::Duration>,
    // --- Loss detection + congestion control ---
    sent_packets: Vec<SentPacket>,
    congestion: CongestionControl,
    flow_control: FlowControl,
    pub stream_flow_control: FlowControl,
    retransmit_queue: Vec<Vec<u8>>,
    /// Path MTU Discovery state (RFC 8899)
    mtu_probe_size: usize,
    mtu_probing: bool,
}

impl QuicTransport {
    /// Create new transport with independent packet number spaces (RFC 9000 §12.3).
    pub fn new(local_cid: ConnectionId, remote_cid: ConnectionId) -> Self {
        let max_data = 65535u64;
        QuicTransport {
            local_cid,
            remote_cid,
            params: TransportParameters::default(),
            pn_state: [
                PacketNumberState::new(), // Initial
                PacketNumberState::new(), // Handshake
                PacketNumberState::new(), // ApplicationData
            ],
            max_data,
            max_stream_data: 65535,
            last_activity: std::time::Instant::now(),
            mtu: 1200,
            rtt_estimate: std::time::Duration::from_millis(100),
            smoothed_rtt: None,
            rttvar: std::time::Duration::from_millis(50),
            min_rtt: None,
            latest_rtt: None,
            sent_packets: Vec::new(),
            congestion: CongestionControl::new(),
            flow_control: FlowControl::new(max_data),
            stream_flow_control: FlowControl::new(65535),
            retransmit_queue: Vec::new(),
            mtu_probe_size: 0,
            mtu_probing: false,
        }
    }

    /// Get the next packet number for the given space (RFC 9000 §12.3).
    /// Get the current packet number for a space (without incrementing).
    pub fn current_packet_number(&self, space: PacketNumberSpace) -> u64 {
        self.pn_state[space as usize].next
    }

    pub fn next_packet_number(&mut self, space: PacketNumberSpace) -> u64 {
        let idx = space as usize;
        let pn = self.pn_state[idx].next;
        self.pn_state[idx].next += 1;
        self.pn_state[idx].packets_sent += 1;
        pn
    }

    /// Record that we received a packet in the given space.
    pub fn record_received_packet(
        &mut self,
        space: PacketNumberSpace,
        packet_number: u64,
    ) -> bool {
        let idx = space as usize;
        if let Some(largest) = self.pn_state[idx].largest_received {
            if packet_number <= largest {
                return false;
            }
        }
        self.pn_state[idx].largest_received = Some(packet_number);
        self.pn_state[idx].record_received(packet_number);
        true
    }

    /// Expand a truncated packet number (RFC 9000 Appendix A.1).
    pub fn expand_packet_number(
        &mut self,
        space: PacketNumberSpace,
        truncated_pn: u64,
        pn_length: usize,
    ) -> Option<u64> {
        debug_assert!((1..=4).contains(&pn_length));
        let idx = space as usize;
        let pn_nbits = (pn_length * 8) as u64;
        let pn_win = 1u64 << pn_nbits;
        let pn_hwin = pn_win / 2;
        let pn_mask = pn_win - 1;

        if self.pn_state[idx].largest_received.is_none() {
            self.pn_state[idx].largest_received = Some(truncated_pn);
            return Some(truncated_pn);
        }

        let largest = self.pn_state[idx].largest_received.unwrap();
        let candidate = (largest & !pn_mask) | truncated_pn;

        let expanded = if candidate > largest.saturating_add(pn_hwin) {
            candidate.saturating_sub(pn_win)
        } else if largest > candidate.saturating_add(pn_hwin) {
            candidate + pn_win
        } else {
            candidate
        };

        if expanded <= largest {
            return None;
        }

        self.pn_state[idx].largest_received = Some(expanded);
        Some(expanded)
    }

    // -----------------------------------------------------------------------
    // ACK generation (RFC 9000 §19.3)
    // -----------------------------------------------------------------------

    pub fn generate_ack_frame(
        &mut self,
        space: PacketNumberSpace,
    ) -> Option<QuicFrame> {
        let idx = space as usize;
        let received = &self.pn_state[idx].received_packets;
        if received.is_empty() {
            return None;
        }

        let mut sorted: Vec<(u64, std::time::Instant)> = received.clone();
        sorted.sort_by_key(|&(pn, _)| pn);
        sorted.dedup_by_key(|tuple| tuple.0);

        if sorted.is_empty() {
            return None;
        }

        let largest_acknowledged = sorted.last().unwrap().0;
        let now = std::time::Instant::now();

        let ack_delay = sorted
            .iter()
            .find(|&&(pn, _)| pn == largest_acknowledged)
            .map(|&(_, t)| now.duration_since(t))
            .unwrap_or(std::time::Duration::ZERO);

        let ack_delay_us = ack_delay.as_micros() as u64;
        let ack_delay_encoded = ack_delay_us >> self.params.ack_delay_exponent;

        let mut ack_ranges = Vec::new();
        let mut first_ack_range = 0;
        let mut prev_pn = largest_acknowledged;

        for i in (0..sorted.len() - 1).rev() {
            let pn = sorted[i].0;
            if prev_pn == pn + 1 {
                first_ack_range += 1;
            } else {
                let gap = prev_pn - pn - 1;
                ack_ranges.push((gap, first_ack_range));
                first_ack_range = 0;
            }
            prev_pn = pn;
        }

        Some(QuicFrame::Ack {
            largest_acknowledged,
            ack_delay: ack_delay_encoded,
            ack_range_count: ack_ranges.len() as u64,
            first_ack_range,
            ack_ranges,
        })
    }

    pub fn should_send_ack(&self, space: PacketNumberSpace) -> bool {
        !self.pn_state[space as usize].received_packets.is_empty()
    }

    // -----------------------------------------------------------------------
    // RTT estimation (RFC 9002 §9)
    // -----------------------------------------------------------------------

    pub fn update_rtt(&mut self, latest_rtt: std::time::Duration, ack_delay: std::time::Duration) {
        self.latest_rtt = Some(latest_rtt);

        self.min_rtt = Some(match self.min_rtt {
            Some(current_min) => current_min.min(latest_rtt),
            None => latest_rtt,
        });

        let adjusted_rtt = if latest_rtt >= self.min_rtt.unwrap_or(latest_rtt) + ack_delay {
            latest_rtt - ack_delay
        } else {
            latest_rtt
        };

        if let Some(smoothed) = self.smoothed_rtt {
            let rttvar_sample = smoothed.abs_diff(adjusted_rtt);
            self.rttvar = self.rttvar.mul_f64(0.75) + rttvar_sample.mul_f64(0.25);
            self.smoothed_rtt = Some(
                smoothed.mul_f64(0.875) + adjusted_rtt.mul_f64(0.125),
            );
        } else {
            self.smoothed_rtt = Some(latest_rtt);
            self.rttvar = latest_rtt / 2;
        }

        self.rtt_estimate = self.smoothed_rtt.unwrap_or(std::time::Duration::from_millis(100));
    }

    pub fn smoothed_rtt(&self) -> Option<std::time::Duration> {
        self.smoothed_rtt
    }

    pub fn min_rtt(&self) -> Option<std::time::Duration> {
        self.min_rtt
    }

    pub fn latest_rtt(&self) -> Option<std::time::Duration> {
        self.latest_rtt
    }

    pub fn pto_duration(&self) -> std::time::Duration {
        let smoothed = self.smoothed_rtt.unwrap_or(self.rtt_estimate);
        let max_ack_delay = std::time::Duration::from_millis(self.params.max_ack_delay);
        smoothed + self.rttvar * 4 + max_ack_delay
    }

    // -----------------------------------------------------------------------
    // Loss detection + congestion control
    // -----------------------------------------------------------------------

    pub fn record_packet_sent(
        &mut self,
        space: PacketNumberSpace,
        packet_number: u64,
        size: usize,
        has_crypto: bool,

    ) {
        let packet = SentPacket {
            packet_number,
            time_sent: std::time::Instant::now(),
            size,
            has_crypto,

            acked: false,
            lost: false,
        };

        self.congestion.bytes_in_flight += size as u64;
        self.sent_packets.push(packet);

        let idx = space as usize;
        self.pn_state[idx].packets_sent += 1;
    }

    /// Get the packet number range for a given space.
    ///
    /// Since each space starts at 0 and uses independent counters, we can
    /// filter by checking which space's counter each PN belongs to.
    fn pn_range_for_space(&self, space: PacketNumberSpace) -> (u64, u64) {
        let idx = space as usize;
        let max_pn = self.pn_state[idx].next;
        // Floor is 0, ceiling is current next PN for this space
        (0, max_pn)
    }

    pub fn on_ack_received(
        &mut self,
        space: PacketNumberSpace,
        largest_acknowledged: u64,
        first_ack_range: u64,
        ack_ranges: &[(u64, u64)],
        ack_delay: std::time::Duration,
    ) {
        let idx = space as usize;

        // Build the set of acked packet numbers from the ACK frame ranges
        // Range 0: [largest_acknowledged - first_ack_range, largest_acknowledged]
        // Range N: [prev_end - gap - range, prev_end - gap - 1]
        let mut acked_pns = std::collections::HashSet::new();

        // First range
        let range_start = largest_acknowledged.saturating_sub(first_ack_range);
        for pn in range_start..=largest_acknowledged {
            acked_pns.insert(pn);
        }

        // Subsequent ranges
        let mut prev_end = range_start;
        for &(gap, range) in ack_ranges {
            let range_end = prev_end.saturating_sub(gap + 1);
            let range_start = range_end.saturating_sub(range);
            for pn in range_start..=range_end {
                acked_pns.insert(pn);
            }
            prev_end = range_start;
        }

        // FIRST: Detect lost packets
        self.detect_lost_packets(space);

        // THEN: Track newly acked packets
        let mut newly_acked_size = 0u64;
        let mut newly_acked_count = 0;
        for pkt in &mut self.sent_packets {
            if pkt.lost {
                continue;
            }
            if !pkt.acked && acked_pns.contains(&pkt.packet_number) {
                pkt.acked = true;
                newly_acked_size += pkt.size as u64;
                newly_acked_count += 1;
            }
        }

        // Update congestion window (RFC 9002 §7)
        if newly_acked_count > 0 {
            if self.congestion.congestion_window < self.congestion.ssthresh {
                self.congestion.congestion_window += newly_acked_size;
            } else {
                let mss = self.mtu as u64;
                let increment = (newly_acked_size * mss).saturating_div(self.congestion.congestion_window);
                self.congestion.congestion_window += increment.max(mss);
            }

            self.congestion.bytes_in_flight = self.congestion.bytes_in_flight.saturating_sub(newly_acked_size);
        }

        // Exit recovery if we've acked past recovery start
        if let Some(recovery_start) = self.congestion.recovery_start_time {
            for pkt in &self.sent_packets {
                if pkt.acked && pkt.time_sent > recovery_start {
                    self.congestion.recovery_start_time = None;
                    break;
                }
            }
        }

        // Update RTT
        if let Some(acked_pkt) = self.sent_packets.iter().find(|p| p.packet_number == largest_acknowledged && p.acked) {
            let latest_rtt = acked_pkt.time_sent.elapsed();
            self.update_rtt(latest_rtt, ack_delay);
        }

        // Clear acked packets
        self.sent_packets.retain(|p| !p.acked && !p.lost);
        self.pn_state[idx].clear_up_to(largest_acknowledged);
    }

    fn detect_lost_packets(&mut self, space: PacketNumberSpace) {
        let idx = space as usize;
        let now = std::time::Instant::now();

        let time_threshold = self.smoothed_rtt
            .unwrap_or(self.rtt_estimate)
            .mul_f64(1.125);

        // Only consider packets in this packet number space.
        // Packets in the same space have contiguous packet numbers, so we can
        // filter by checking the range: Initial [0, handshake_start),
        // Handshake [handshake_start, app_start), AppData [app_start, ∞).
        // For simplicity, we use the space index to filter — packets recorded
        // with `record_packet_sent` for a given space have their PN tracked.
        // Since we track all sent_packets in one flat list, filter by PN range.
        let (pn_min, pn_max) = self.pn_range_for_space(space);

        let lost_pns: Vec<u64> = self.sent_packets.iter().filter_map(|pkt| {
            if pkt.acked || pkt.lost {
                return None;
            }
            // Filter by packet number space
            if pkt.packet_number < pn_min || pkt.packet_number >= pn_max {
                return None;
            }

            let larger_acked = self.sent_packets.iter().any(|other| {
                other.acked && other.packet_number > pkt.packet_number
                    && other.packet_number >= pn_min && other.packet_number < pn_max
            });

            let time_expired = now.duration_since(pkt.time_sent) > time_threshold;

            if larger_acked || time_expired {
                Some(pkt.packet_number)
            } else {
                None
            }
        }).collect();

        let mut lost_size = 0u64;
        for pn in &lost_pns {
            for pkt in &mut self.sent_packets {
                if pkt.packet_number == *pn && !pkt.lost {
                    pkt.lost = true;
                    lost_size += pkt.size as u64;

                    if pkt.has_crypto {
                        self.retransmit_queue.push(Vec::new());
                    }
                }
            }
        }

        if lost_size > 0 {
            self.congestion.bytes_in_flight = self.congestion.bytes_in_flight.saturating_sub(lost_size);
            self.congestion.ssthresh = self.congestion.congestion_window / 2;
            self.congestion.congestion_window = self.congestion.ssthresh.max(self.mtu as u64);
            self.congestion.recovery_start_time = Some(now);
        }
    }

    pub fn get_retransmit_queue(&self) -> &[Vec<u8>] {
        &self.retransmit_queue
    }

    pub fn clear_retransmit_queue(&mut self) {
        self.retransmit_queue.clear();
    }

    pub fn congestion_window(&self) -> u64 {
        self.congestion.congestion_window
    }

    pub fn bytes_in_flight(&self) -> u64 {
        self.congestion.bytes_in_flight
    }

    pub fn can_send_bytes(&self, bytes: usize) -> bool {
        self.congestion.can_send() && self.congestion.available_window() >= bytes as u64
    }

    // -----------------------------------------------------------------------
    // Flow control
    // -----------------------------------------------------------------------

    pub fn available_flow_control(&self) -> u64 {
        let conn_available = self.flow_control.remaining_send();
        let stream_available = self.stream_flow_control.remaining_send();
        conn_available.min(stream_available)
    }

    pub fn record_data_sent(&mut self, len: usize) {
        self.flow_control.record_sent(len);
        self.stream_flow_control.record_sent(len);
    }

    pub fn record_data_received(&mut self, len: usize) -> Option<QuicFrame> {
        self.flow_control.record_received(len);

        let remaining = self.flow_control.remaining_receive();
        if remaining < self.max_data / 2 {
            let new_max = self.flow_control.received + self.max_data;
            self.max_data = new_max;
            self.flow_control.max_data = new_max;
            return Some(QuicFrame::MaxData { max_data: new_max });
        }
        None
    }

    // -----------------------------------------------------------------------
    // Utility
    // -----------------------------------------------------------------------

    pub fn update_activity(&mut self) {
        self.last_activity = std::time::Instant::now();
    }

    pub fn last_activity(&self) -> std::time::Instant {
        self.last_activity
    }

    pub fn is_idle(&self, timeout: std::time::Duration) -> bool {
        self.last_activity.elapsed() > timeout
    }

    pub fn create_stream_frame(
        &mut self,
        stream_id: u64,
        data: Vec<u8>,
        fin: bool,
    ) -> QuicFrame {
        QuicFrame::Stream {
            stream_id,
            offset: 0,
            fin,
            data,
        }
    }

    pub fn create_max_data_frame(&self) -> QuicFrame {
        QuicFrame::MaxData {
            max_data: self.max_data,
        }
    }

    pub fn process_frame(&mut self, frame: &QuicFrame) {
        match frame {
            QuicFrame::MaxData { max_data } => {
                self.max_data = *max_data;
            }
            QuicFrame::MaxStreamData { max_stream_data, .. } => {
                self.max_stream_data = *max_stream_data;
            }
            QuicFrame::Ack { largest_acknowledged, ack_delay, first_ack_range, ack_ranges, .. } => {
                let delay = std::time::Duration::from_micros(*ack_delay);
                self.on_ack_received(PacketNumberSpace::ApplicationData, *largest_acknowledged,
                    *first_ack_range, ack_ranges, delay);
            }
            _ => {}
        }
        self.update_activity();
    }

    /// Get RTT estimate
    pub fn rtt(&self) -> std::time::Duration {
        self.rtt_estimate
    }

    /// Get current MTU
    pub fn mtu(&self) -> usize {
        self.mtu
    }

    // -----------------------------------------------------------------------
    // Path MTU Discovery (RFC 8899)
    // -----------------------------------------------------------------------

    /// Start Path MTU Discovery (RFC 8899).
    ///
    /// Sends increasingly larger packets to discover the path MTU.
    /// Returns the size of the next probe packet to send.
    pub fn start_mtu_discovery(&mut self) -> Option<usize> {
        self.mtu_probing = true;
        self.mtu_probe_size = self.mtu + 100;
        self.next_mtu_probe()
    }

    /// Get the next MTU probe size to try.
    pub fn next_mtu_probe(&mut self) -> Option<usize> {
        if !self.mtu_probing {
            return None;
        }

        let max_probe = self.mtu + 1280;
        if self.mtu_probe_size > max_probe {
            self.mtu_probing = false;
            return None;
        }

        let probe_size = self.mtu_probe_size;
        self.mtu_probe_size += 100;
        Some(probe_size)
    }

    /// Record that an MTU probe succeeded.
    pub fn on_mtu_probe_success(&mut self, probe_size: usize) {
        self.mtu = probe_size;
        self.mtu_probing = false;
    }

    /// Record that an MTU probe failed.
    pub fn on_mtu_probe_failure(&mut self) {
        self.mtu_probing = false;
    }

    /// Check if we're currently probing MTU.
    pub fn is_mtu_probing(&self) -> bool {
        self.mtu_probing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_new() {
        let local = ConnectionId::random();
        let remote = ConnectionId::random();
        let transport = QuicTransport::new(local.clone(), remote.clone());
        assert_eq!(transport.current_packet_number(PacketNumberSpace::Initial), 0);
        assert_eq!(transport.max_data, 65535);
    }

    #[test]
    fn test_packet_number_independent_spaces() {
        let local = ConnectionId::random();
        let remote = ConnectionId::random();
        let mut transport = QuicTransport::new(local, remote);

        assert_eq!(transport.current_packet_number(PacketNumberSpace::Initial), 0);
        assert_eq!(transport.current_packet_number(PacketNumberSpace::Handshake), 0);
        assert_eq!(transport.current_packet_number(PacketNumberSpace::ApplicationData), 0);

        let pn_initial = transport.next_packet_number(PacketNumberSpace::Initial);
        assert_eq!(pn_initial, 0);
        assert_eq!(transport.current_packet_number(PacketNumberSpace::Initial), 1);
        assert_eq!(transport.current_packet_number(PacketNumberSpace::Handshake), 0);

        let pn_hs = transport.next_packet_number(PacketNumberSpace::Handshake);
        assert_eq!(pn_hs, 0);
        assert_eq!(transport.current_packet_number(PacketNumberSpace::Handshake), 1);
    }

    #[test]
    fn test_packet_number_increment_per_space() {
        let local = ConnectionId::random();
        let remote = ConnectionId::random();
        let mut transport = QuicTransport::new(local, remote);

        assert_eq!(transport.next_packet_number(PacketNumberSpace::Initial), 0);
        assert_eq!(transport.next_packet_number(PacketNumberSpace::Initial), 1);
        assert_eq!(transport.next_packet_number(PacketNumberSpace::Initial), 2);
    }

    #[test]
    fn test_idle_timeout() {
        let local = ConnectionId::random();
        let remote = ConnectionId::random();
        let transport = QuicTransport::new(local, remote);
        assert!(!transport.is_idle(std::time::Duration::from_secs(60)));
    }

    #[test]
    fn test_stream_frame_creation() {
        let local = ConnectionId::random();
        let remote = ConnectionId::random();
        let mut transport = QuicTransport::new(local, remote);
        let frame = transport.create_stream_frame(0, b"hello".to_vec(), false);
        if let QuicFrame::Stream { data, fin, .. } = frame {
            assert_eq!(data, b"hello");
            assert!(!fin);
        } else {
            panic!("Expected Stream frame");
        }
    }
}
