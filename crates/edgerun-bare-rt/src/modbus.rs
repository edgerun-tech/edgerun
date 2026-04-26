//! Modbus client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub const MODBUS_MAX_REGISTERS: usize = 125;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModbusFunction {
    ReadCoils,
    ReadDiscreteInputs,
    ReadHoldingRegisters,
    ReadInputRegisters,
    WriteSingleCoil,
    WriteSingleRegister,
    WriteMultipleCoils,
    WriteMultipleRegisters,
}

pub struct ModbusRequest {
    pub unit_id: u8,
    pub function: ModbusFunction,
    pub address: u16,
    pub quantity: u16,
    pub data: Vec<u8>,
}

impl ModbusRequest {
    pub fn read_holding(unit_id: u8, address: u16, quantity: u16) -> Self {
        Self { unit_id, function: ModbusFunction::ReadHoldingRegisters, address, quantity, data: Vec::new() }
    }

    pub fn write_registers(unit_id: u8, address: u16, data: &[u8]) -> Self {
        Self { unit_id, function: ModbusFunction::WriteMultipleRegisters, address, quantity: 0, data: data.to_vec() }
    }
}

pub struct ModbusResponse {
    pub data: Vec<u8>,
    pub exception_code: u8,
}

impl ModbusResponse {
    pub fn new() -> Self {
        Self { data: Vec::new(), exception_code: 0 }
    }

    pub fn to_u16(&self) -> Vec<u16> {
        let mut result = Vec::new();
        for chunk in self.data.chunks(2) {
            if chunk.len() == 2 {
                let val = ((chunk[0] as u16) << 8) | (chunk[1] as u16);
                result.push(val);
            }
        }
        result
    }
}

impl Default for ModbusResponse {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ModbusClient;

impl ModbusClient {
    pub fn new() -> Self {
        Self
    }

    pub fn connect(&self, _host: &[u8], _port: u16) -> ModbusConnectFuture {
        ModbusConnectFuture { done: false }
    }

    pub fn request(&self, _req: &ModbusRequest) -> ModbusRequestFuture {
        ModbusRequestFuture { done: false }
    }
}

impl Default for ModbusClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ModbusConnectFuture {
    done: bool,
}

impl Future for ModbusConnectFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct ModbusRequestFuture {
    done: bool,
}

impl Future for ModbusRequestFuture {
    type Output = Result<ModbusResponse, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}