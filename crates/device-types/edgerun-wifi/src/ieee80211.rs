//! Minimal IEEE 802.11 open-AP frame core.
//!
//! This module is intentionally hardware-independent. ESP32-S3 radio code can
//! feed raw 802.11 frames in here and send the produced raw 802.11 frames back
//! out, while the rest of Edgerun continues to see Ethernet frames through the
//! runtime network boundary.

pub const MAX_SSID_LEN: usize = 32;
pub const MAC_LEN: usize = 6;
pub const MANAGEMENT_HEADER_LEN: usize = 24;
pub const DATA_HEADER_LEN: usize = 24;
pub const LLC_SNAP_LEN: usize = 8;

pub const FRAME_TYPE_MANAGEMENT: u16 = 0;
pub const FRAME_TYPE_DATA: u16 = 2;

pub const SUBTYPE_ASSOC_REQ: u16 = 0;
pub const SUBTYPE_ASSOC_RESP: u16 = 1;
pub const SUBTYPE_PROBE_REQ: u16 = 4;
pub const SUBTYPE_PROBE_RESP: u16 = 5;
pub const SUBTYPE_BEACON: u16 = 8;
pub const SUBTYPE_AUTH: u16 = 11;
pub const SUBTYPE_DATA: u16 = 0;

const CAP_ESS: u16 = 1 << 0;
const CAP_SHORT_PREAMBLE: u16 = 1 << 5;
const CAP_SHORT_SLOT_TIME: u16 = 1 << 10;
const AUTH_ALGO_OPEN_SYSTEM: u16 = 0;
const AUTH_SEQUENCE_RESPONSE: u16 = 2;
const STATUS_SUCCESS: u16 = 0;
const AID_FIRST: u16 = 1;

const LLC_SNAP_RFC1042: [u8; LLC_SNAP_LEN] = [0xaa, 0xaa, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MacAddr(pub [u8; MAC_LEN]);

impl MacAddr {
    pub const BROADCAST: Self = Self([0xff; MAC_LEN]);

    pub const fn new(bytes: [u8; MAC_LEN]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; MAC_LEN] {
        self.0
    }

    pub fn is_broadcast(self) -> bool {
        self.0 == Self::BROADCAST.0
    }

    pub fn is_zero(self) -> bool {
        self.0 == [0; MAC_LEN]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenApError {
    SsidTooLong,
    EmptySsid,
    BufferTooSmall,
    MalformedFrame,
    UnsupportedFrame,
    NoStationSlot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenApConfig {
    pub bssid: MacAddr,
    pub ssid: [u8; MAX_SSID_LEN],
    pub ssid_len: usize,
    pub channel: u8,
    pub beacon_interval_tu: u16,
}

impl OpenApConfig {
    pub fn new(bssid: MacAddr, ssid: &[u8], channel: u8) -> Result<Self, OpenApError> {
        if ssid.is_empty() {
            return Err(OpenApError::EmptySsid);
        }
        if ssid.len() > MAX_SSID_LEN {
            return Err(OpenApError::SsidTooLong);
        }
        let mut stored = [0; MAX_SSID_LEN];
        stored[..ssid.len()].copy_from_slice(ssid);
        Ok(Self {
            bssid,
            ssid: stored,
            ssid_len: ssid.len(),
            channel,
            beacon_interval_tu: 100,
        })
    }

    pub fn ssid(&self) -> &[u8] {
        &self.ssid[..self.ssid_len]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApEvent {
    ProbeRequest { station: MacAddr },
    Authenticated { station: MacAddr },
    Associated { station: MacAddr, aid: u16 },
    Data { station: MacAddr, len: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Station {
    pub mac: MacAddr,
    pub authenticated: bool,
    pub associated: bool,
    pub aid: u16,
}

impl Station {
    pub const fn empty() -> Self {
        Self {
            mac: MacAddr([0; MAC_LEN]),
            authenticated: false,
            associated: false,
            aid: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameAction {
    pub raw_tx_len: usize,
    pub event: Option<ApEvent>,
}

pub struct OpenAp<const MAX_STATIONS: usize> {
    config: OpenApConfig,
    stations: [Station; MAX_STATIONS],
    seq: u16,
}

impl<const MAX_STATIONS: usize> OpenAp<MAX_STATIONS> {
    pub const fn new(config: OpenApConfig) -> Self {
        Self {
            config,
            stations: [Station::empty(); MAX_STATIONS],
            seq: 0,
        }
    }

    pub fn config(&self) -> &OpenApConfig {
        &self.config
    }

    pub fn stations(&self) -> &[Station; MAX_STATIONS] {
        &self.stations
    }

    pub fn build_beacon(&mut self, out: &mut [u8]) -> Result<usize, OpenApError> {
        self.write_beacon_like(SUBTYPE_BEACON, MacAddr::BROADCAST, out)
    }

    pub fn handle_frame(
        &mut self,
        frame: &[u8],
        raw_tx: &mut [u8],
        ethernet_out: &mut [u8],
    ) -> Result<FrameAction, OpenApError> {
        let control = FrameControl::parse(frame)?;
        match (control.frame_type, control.subtype) {
            (FRAME_TYPE_MANAGEMENT, SUBTYPE_PROBE_REQ) => {
                let header = MgmtHeader::parse(frame)?;
                if !self.probe_matches(frame) {
                    return Err(OpenApError::UnsupportedFrame);
                }
                let len = self.write_beacon_like(SUBTYPE_PROBE_RESP, header.addr2, raw_tx)?;
                Ok(FrameAction {
                    raw_tx_len: len,
                    event: Some(ApEvent::ProbeRequest {
                        station: header.addr2,
                    }),
                })
            }
            (FRAME_TYPE_MANAGEMENT, SUBTYPE_AUTH) => {
                let header = MgmtHeader::parse(frame)?;
                let station = self.ensure_station(header.addr2)?;
                station.authenticated = true;
                let len = self.write_auth_response(header.addr2, raw_tx)?;
                Ok(FrameAction {
                    raw_tx_len: len,
                    event: Some(ApEvent::Authenticated {
                        station: header.addr2,
                    }),
                })
            }
            (FRAME_TYPE_MANAGEMENT, SUBTYPE_ASSOC_REQ) => {
                let header = MgmtHeader::parse(frame)?;
                let aid = {
                    let station = self.ensure_station(header.addr2)?;
                    if !station.authenticated {
                        station.authenticated = true;
                    }
                    station.associated = true;
                    if station.aid == 0 {
                        station.aid = AID_FIRST;
                    }
                    station.aid
                };
                let len = self.write_assoc_response(header.addr2, aid, raw_tx)?;
                Ok(FrameAction {
                    raw_tx_len: len,
                    event: Some(ApEvent::Associated {
                        station: header.addr2,
                        aid,
                    }),
                })
            }
            (FRAME_TYPE_DATA, SUBTYPE_DATA) => {
                let header = DataHeader::parse(frame)?;
                if header.addr1 != self.config.bssid {
                    return Err(OpenApError::UnsupportedFrame);
                }
                let len = decapsulate_data_to_ethernet(frame, ethernet_out)?;
                Ok(FrameAction {
                    raw_tx_len: 0,
                    event: Some(ApEvent::Data {
                        station: header.addr2,
                        len,
                    }),
                })
            }
            _ => Err(OpenApError::UnsupportedFrame),
        }
    }

    pub fn encapsulate_ethernet(
        &mut self,
        ethernet: &[u8],
        station: MacAddr,
        out: &mut [u8],
    ) -> Result<usize, OpenApError> {
        encapsulate_ethernet_to_data(ethernet, self.config.bssid, station, self.next_seq(), out)
    }

    fn next_seq(&mut self) -> u16 {
        let seq = self.seq;
        self.seq = self.seq.wrapping_add(1) & 0x0fff;
        seq
    }

    fn ensure_station(&mut self, mac: MacAddr) -> Result<&mut Station, OpenApError> {
        if mac.is_zero() || mac.is_broadcast() {
            return Err(OpenApError::MalformedFrame);
        }
        if let Some(index) = self.stations.iter().position(|station| station.mac == mac) {
            return Ok(&mut self.stations[index]);
        }
        if let Some(index) = self
            .stations
            .iter()
            .position(|station| station.mac.is_zero())
        {
            let station = &mut self.stations[index];
            station.mac = mac;
            station.aid = AID_FIRST + index as u16;
            return Ok(station);
        }
        Err(OpenApError::NoStationSlot)
    }

    fn write_beacon_like(
        &mut self,
        subtype: u16,
        dst: MacAddr,
        out: &mut [u8],
    ) -> Result<usize, OpenApError> {
        let fixed_len = 12;
        let ies_len = 2 + self.config.ssid_len + 2 + 8 + 3;
        let len = MANAGEMENT_HEADER_LEN + fixed_len + ies_len;
        if out.len() < len {
            return Err(OpenApError::BufferTooSmall);
        }

        write_mgmt_header(
            out,
            subtype,
            dst,
            self.config.bssid,
            self.config.bssid,
            self.next_seq(),
        );
        let mut offset = MANAGEMENT_HEADER_LEN;
        out[offset..offset + 8].fill(0);
        offset += 8;
        write_le_u16(out, offset, self.config.beacon_interval_tu);
        offset += 2;
        write_le_u16(
            out,
            offset,
            CAP_ESS | CAP_SHORT_PREAMBLE | CAP_SHORT_SLOT_TIME,
        );
        offset += 2;
        offset = write_ie(out, offset, 0, self.config.ssid())?;
        offset = write_ie(
            out,
            offset,
            1,
            &[0x82, 0x84, 0x8b, 0x96, 0x0c, 0x12, 0x18, 0x24],
        )?;
        offset = write_ie(out, offset, 3, &[self.config.channel])?;
        Ok(offset)
    }

    fn write_auth_response(&mut self, dst: MacAddr, out: &mut [u8]) -> Result<usize, OpenApError> {
        let len = MANAGEMENT_HEADER_LEN + 6;
        if out.len() < len {
            return Err(OpenApError::BufferTooSmall);
        }
        write_mgmt_header(
            out,
            SUBTYPE_AUTH,
            dst,
            self.config.bssid,
            self.config.bssid,
            self.next_seq(),
        );
        write_le_u16(out, MANAGEMENT_HEADER_LEN, AUTH_ALGO_OPEN_SYSTEM);
        write_le_u16(out, MANAGEMENT_HEADER_LEN + 2, AUTH_SEQUENCE_RESPONSE);
        write_le_u16(out, MANAGEMENT_HEADER_LEN + 4, STATUS_SUCCESS);
        Ok(len)
    }

    fn write_assoc_response(
        &mut self,
        dst: MacAddr,
        aid: u16,
        out: &mut [u8],
    ) -> Result<usize, OpenApError> {
        let rates = [0x82, 0x84, 0x8b, 0x96, 0x0c, 0x12, 0x18, 0x24];
        let len = MANAGEMENT_HEADER_LEN + 6 + 2 + rates.len();
        if out.len() < len {
            return Err(OpenApError::BufferTooSmall);
        }
        write_mgmt_header(
            out,
            SUBTYPE_ASSOC_RESP,
            dst,
            self.config.bssid,
            self.config.bssid,
            self.next_seq(),
        );
        write_le_u16(
            out,
            MANAGEMENT_HEADER_LEN,
            CAP_ESS | CAP_SHORT_PREAMBLE | CAP_SHORT_SLOT_TIME,
        );
        write_le_u16(out, MANAGEMENT_HEADER_LEN + 2, STATUS_SUCCESS);
        write_le_u16(out, MANAGEMENT_HEADER_LEN + 4, aid | 0xc000);
        write_ie(out, MANAGEMENT_HEADER_LEN + 6, 1, &rates)
    }

    fn probe_matches(&self, frame: &[u8]) -> bool {
        let mut offset = MANAGEMENT_HEADER_LEN;
        while offset + 2 <= frame.len() {
            let id = frame[offset];
            let len = frame[offset + 1] as usize;
            offset += 2;
            if offset + len > frame.len() {
                return false;
            }
            if id == 0 {
                let ssid = &frame[offset..offset + len];
                return ssid.is_empty() || ssid == self.config.ssid();
            }
            offset += len;
        }
        true
    }
}

#[derive(Clone, Copy)]
struct FrameControl {
    frame_type: u16,
    subtype: u16,
    to_ds: bool,
    from_ds: bool,
}

impl FrameControl {
    fn parse(frame: &[u8]) -> Result<Self, OpenApError> {
        if frame.len() < 2 {
            return Err(OpenApError::MalformedFrame);
        }
        let raw = u16::from_le_bytes([frame[0], frame[1]]);
        Ok(Self {
            frame_type: (raw >> 2) & 0x3,
            subtype: (raw >> 4) & 0xf,
            to_ds: (raw & (1 << 8)) != 0,
            from_ds: (raw & (1 << 9)) != 0,
        })
    }
}

#[derive(Clone, Copy)]
struct MgmtHeader {
    addr2: MacAddr,
}

impl MgmtHeader {
    fn parse(frame: &[u8]) -> Result<Self, OpenApError> {
        if frame.len() < MANAGEMENT_HEADER_LEN {
            return Err(OpenApError::MalformedFrame);
        }
        Ok(Self {
            addr2: read_mac(frame, 10),
        })
    }
}

#[derive(Clone, Copy)]
struct DataHeader {
    addr1: MacAddr,
    addr2: MacAddr,
    addr3: MacAddr,
    to_ds: bool,
    from_ds: bool,
}

impl DataHeader {
    fn parse(frame: &[u8]) -> Result<Self, OpenApError> {
        if frame.len() < DATA_HEADER_LEN {
            return Err(OpenApError::MalformedFrame);
        }
        let control = FrameControl::parse(frame)?;
        Ok(Self {
            addr1: read_mac(frame, 4),
            addr2: read_mac(frame, 10),
            addr3: read_mac(frame, 16),
            to_ds: control.to_ds,
            from_ds: control.from_ds,
        })
    }
}

pub fn encapsulate_ethernet_to_data(
    ethernet: &[u8],
    bssid: MacAddr,
    station: MacAddr,
    seq: u16,
    out: &mut [u8],
) -> Result<usize, OpenApError> {
    if ethernet.len() < 14 {
        return Err(OpenApError::MalformedFrame);
    }
    let payload_len = ethernet.len() - 14;
    let len = DATA_HEADER_LEN + LLC_SNAP_LEN + payload_len;
    if out.len() < len {
        return Err(OpenApError::BufferTooSmall);
    }

    write_data_header_from_ds(out, station, bssid, read_mac(ethernet, 6), seq);
    out[DATA_HEADER_LEN..DATA_HEADER_LEN + LLC_SNAP_LEN].copy_from_slice(&LLC_SNAP_RFC1042);
    out[DATA_HEADER_LEN + 6..DATA_HEADER_LEN + 8].copy_from_slice(&ethernet[12..14]);
    out[DATA_HEADER_LEN + LLC_SNAP_LEN..len].copy_from_slice(&ethernet[14..]);
    Ok(len)
}

pub fn decapsulate_data_to_ethernet(frame: &[u8], out: &mut [u8]) -> Result<usize, OpenApError> {
    let header = DataHeader::parse(frame)?;
    if !header.to_ds || header.from_ds {
        return Err(OpenApError::UnsupportedFrame);
    }
    if frame.len() < DATA_HEADER_LEN + LLC_SNAP_LEN {
        return Err(OpenApError::MalformedFrame);
    }
    let snap = &frame[DATA_HEADER_LEN..DATA_HEADER_LEN + LLC_SNAP_LEN];
    if snap[..6] != LLC_SNAP_RFC1042[..6] {
        return Err(OpenApError::UnsupportedFrame);
    }
    let payload = &frame[DATA_HEADER_LEN + LLC_SNAP_LEN..];
    let len = 14 + payload.len();
    if out.len() < len {
        return Err(OpenApError::BufferTooSmall);
    }
    out[..6].copy_from_slice(&header.addr3.0);
    out[6..12].copy_from_slice(&header.addr2.0);
    out[12..14].copy_from_slice(&snap[6..8]);
    out[14..len].copy_from_slice(payload);
    Ok(len)
}

fn write_mgmt_header(
    out: &mut [u8],
    subtype: u16,
    dst: MacAddr,
    src: MacAddr,
    bssid: MacAddr,
    seq: u16,
) {
    let fc = ((subtype & 0xf) << 4) | (FRAME_TYPE_MANAGEMENT << 2);
    write_le_u16(out, 0, fc);
    write_le_u16(out, 2, 0);
    out[4..10].copy_from_slice(&dst.0);
    out[10..16].copy_from_slice(&src.0);
    out[16..22].copy_from_slice(&bssid.0);
    write_le_u16(out, 22, seq << 4);
}

fn write_data_header_from_ds(out: &mut [u8], dst: MacAddr, bssid: MacAddr, src: MacAddr, seq: u16) {
    let fc = (SUBTYPE_DATA << 4) | (FRAME_TYPE_DATA << 2) | (1 << 9);
    write_le_u16(out, 0, fc);
    write_le_u16(out, 2, 0);
    out[4..10].copy_from_slice(&dst.0);
    out[10..16].copy_from_slice(&bssid.0);
    out[16..22].copy_from_slice(&src.0);
    write_le_u16(out, 22, seq << 4);
}

fn write_ie(out: &mut [u8], offset: usize, id: u8, payload: &[u8]) -> Result<usize, OpenApError> {
    if payload.len() > u8::MAX as usize || out.len() < offset + 2 + payload.len() {
        return Err(OpenApError::BufferTooSmall);
    }
    out[offset] = id;
    out[offset + 1] = payload.len() as u8;
    out[offset + 2..offset + 2 + payload.len()].copy_from_slice(payload);
    Ok(offset + 2 + payload.len())
}

fn read_mac(data: &[u8], offset: usize) -> MacAddr {
    let mut mac = [0; MAC_LEN];
    mac.copy_from_slice(&data[offset..offset + MAC_LEN]);
    MacAddr(mac)
}

fn write_le_u16(out: &mut [u8], offset: usize, value: u16) {
    out[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    const BSSID: MacAddr = MacAddr([0x02, 0xed, 0x67, 0x75, 0x6e, 0x01]);
    const STA: MacAddr = MacAddr([0x10, 0x20, 0x30, 0x40, 0x50, 0x60]);
    const DST: MacAddr = MacAddr([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);

    fn ap() -> OpenAp<4> {
        OpenAp::new(OpenApConfig::new(BSSID, b"edgerun-ac", 6).unwrap())
    }

    fn write_probe_req(out: &mut [u8], ssid: &[u8]) -> usize {
        write_mgmt_header(
            out,
            SUBTYPE_PROBE_REQ,
            MacAddr::BROADCAST,
            STA,
            MacAddr::BROADCAST,
            0,
        );
        write_ie(out, MANAGEMENT_HEADER_LEN, 0, ssid).unwrap()
    }

    fn write_auth_req(out: &mut [u8]) -> usize {
        write_mgmt_header(out, SUBTYPE_AUTH, BSSID, STA, BSSID, 0);
        write_le_u16(out, MANAGEMENT_HEADER_LEN, AUTH_ALGO_OPEN_SYSTEM);
        write_le_u16(out, MANAGEMENT_HEADER_LEN + 2, 1);
        write_le_u16(out, MANAGEMENT_HEADER_LEN + 4, STATUS_SUCCESS);
        MANAGEMENT_HEADER_LEN + 6
    }

    fn write_assoc_req(out: &mut [u8]) -> usize {
        write_mgmt_header(out, SUBTYPE_ASSOC_REQ, BSSID, STA, BSSID, 0);
        write_le_u16(out, MANAGEMENT_HEADER_LEN, CAP_ESS);
        write_le_u16(out, MANAGEMENT_HEADER_LEN + 2, 10);
        write_ie(out, MANAGEMENT_HEADER_LEN + 4, 0, b"edgerun-ac").unwrap()
    }

    #[test]
    fn beacon_contains_ssid_and_channel() {
        let mut ap = ap();
        let mut out = [0; 256];
        let len = ap.build_beacon(&mut out).unwrap();

        assert_eq!(
            (u16::from_le_bytes([out[0], out[1]]) >> 4) & 0xf,
            SUBTYPE_BEACON
        );
        assert!(out[..len]
            .windows(b"edgerun-ac".len())
            .any(|w| w == b"edgerun-ac"));
        assert!(out[..len].windows(3).any(|w| w == [3, 1, 6]));
    }

    #[test]
    fn probe_request_gets_probe_response() {
        let mut ap = ap();
        let mut frame = [0; 128];
        let len = write_probe_req(&mut frame, b"edgerun-ac");
        let mut raw_tx = [0; 256];
        let mut eth = [0; 1514];

        let action = ap
            .handle_frame(&frame[..len], &mut raw_tx, &mut eth)
            .unwrap();

        assert!(action.raw_tx_len > MANAGEMENT_HEADER_LEN);
        assert_eq!(action.event, Some(ApEvent::ProbeRequest { station: STA }));
        assert_eq!(
            (u16::from_le_bytes([raw_tx[0], raw_tx[1]]) >> 4) & 0xf,
            SUBTYPE_PROBE_RESP
        );
    }

    #[test]
    fn auth_and_assoc_track_station() {
        let mut ap = ap();
        let mut frame = [0; 128];
        let mut raw_tx = [0; 256];
        let mut eth = [0; 1514];

        let auth_len = write_auth_req(&mut frame);
        let auth = ap
            .handle_frame(&frame[..auth_len], &mut raw_tx, &mut eth)
            .unwrap();
        assert_eq!(auth.event, Some(ApEvent::Authenticated { station: STA }));

        let assoc_len = write_assoc_req(&mut frame);
        let assoc = ap
            .handle_frame(&frame[..assoc_len], &mut raw_tx, &mut eth)
            .unwrap();
        assert_eq!(
            assoc.event,
            Some(ApEvent::Associated {
                station: STA,
                aid: 1
            })
        );
        assert!(ap.stations()[0].associated);
    }

    #[test]
    fn ethernet_round_trips_through_data_frame() {
        let mut ap = ap();
        let mut downlink = [0; 64];
        downlink[..6].copy_from_slice(&STA.0);
        downlink[6..12].copy_from_slice(&DST.0);
        downlink[12..14].copy_from_slice(&0x0800u16.to_be_bytes());
        downlink[14..18].copy_from_slice(&[1, 2, 3, 4]);

        let mut wifi = [0; 256];
        let wifi_len = ap
            .encapsulate_ethernet(&downlink[..18], STA, &mut wifi)
            .unwrap();
        let mut out = [0; 64];
        let eth_len = decapsulate_data_to_ethernet(&wifi[..wifi_len], &mut out);

        assert_eq!(eth_len, Err(OpenApError::UnsupportedFrame));

        let mut uplink = [0; 64];
        uplink[..6].copy_from_slice(&DST.0);
        uplink[6..12].copy_from_slice(&STA.0);
        uplink[12..14].copy_from_slice(&0x0800u16.to_be_bytes());
        uplink[14..18].copy_from_slice(&[1, 2, 3, 4]);

        let to_ds_fc = (SUBTYPE_DATA << 4) | (FRAME_TYPE_DATA << 2) | (1 << 8);
        write_le_u16(&mut wifi, 0, to_ds_fc);
        wifi[4..10].copy_from_slice(&BSSID.0);
        wifi[10..16].copy_from_slice(&STA.0);
        wifi[16..22].copy_from_slice(&DST.0);
        let eth_len = decapsulate_data_to_ethernet(&wifi[..wifi_len], &mut out).unwrap();
        assert_eq!(&out[..eth_len], &uplink[..18]);
    }
}
