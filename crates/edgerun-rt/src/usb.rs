//! USB driver stubs

extern crate alloc;

use alloc::vec::Vec;

#[derive(Clone, Copy, Debug)]
pub enum UsbSpeed {
    Low,
    Full,
    High,
    Super,
    SuperPlus,
}

pub struct UsbDevice {
    pub vendor_id: u16,
    pub product_id: u16,
    pub class: u8,
    pub subclass: u8,
    pub speed: UsbSpeed,
    pub max_packet_size: u8,
}

pub struct UsbEndpoint {
    pub address: u8,
    pub direction: UsbDirection,
    pub transfer_type: UsbTransferType,
    pub max_packet_size: u16,
}

#[derive(Clone, Copy, Debug)]
pub enum UsbDirection {
    Out,
    In,
}

#[derive(Clone, Copy, Debug)]
pub enum UsbTransferType {
    Control,
    Isochronous,
    Bulk,
    Interrupt,
}

pub struct UsbController {
    pub base: usize,
    pub devices: Vec<UsbDevice>,
    pub running: bool,
}

impl UsbController {
    pub fn new(base: usize) -> Self {
        Self { base, devices: Vec::new(), running: false }
    }

    pub fn init(&mut self) -> bool {
        self.running = true;
        true
    }

    pub fn scan(&mut self) -> &[UsbDevice] {
        &self.devices
    }

    pub fn reset_port(&mut self, _port: u8) -> bool {
        true
    }

    pub fn control_transfer(&mut self, _dev: &UsbDevice, _request: u8, _value: u16, _index: u16, _data: &mut [u8]) -> Result<usize, UsbError> {
        Ok(0)
    }

    pub fn bulk_transfer(&mut self, _ep: &UsbEndpoint, _data: &mut [u8]) -> Result<usize, UsbError> {
        Ok(0)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum UsbError {
    NotFound,
    Stall,
    Timeout,
    Protocol,
    NoDevice,
}

#[derive(Clone, Copy, Debug)]
pub enum XhciReg {
    UsbCmd,
    UsbSts,
    PgOffset,
    Crce,
    DnCtrl,
   Dcbaap,
    DcbaapHi,
    Config,
    PortScBase,
}

pub struct XhciController {
    pub base: usize,
    pub ports: u8,
}

impl XhciController {
    pub fn new(base: usize) -> Self {
        Self { base, ports: 0 }
    }

    pub fn init(&mut self) -> bool {
        true
    }

    pub fn run(&mut self) -> bool {
        true
    }

    pub fn stop(&mut self) -> bool {
        true
    }

    pub fn port_count(&self) -> u8 {
        self.ports
    }
}