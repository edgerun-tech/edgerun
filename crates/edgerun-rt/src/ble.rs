//! BLE client

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BleState {
    PoweredOff,
    PoweredOn,
    Advertising,
    Connected,
    Disconnected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BleService {
    pub uuid: [u8; 16],
    pub handle: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BleCharacteristic {
    pub uuid: [u8; 16],
    pub handle: u16,
    pub properties: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BleDevice {
    pub addr: [u8; 6],
    pub rssi: i8,
    pub name: Vec<u8>,
}

pub struct BleClient {
    state: BleState,
    next_connection: u16,
    active_connection: Option<u16>,
    devices: Vec<BleDevice>,
    services: Vec<BleService>,
    characteristics: Vec<BleCharacteristic>,
    values: Vec<(u16, Vec<u8>)>,
    notifications: Vec<u16>,
}

impl BleClient {
    pub fn new() -> Self {
        Self {
            state: BleState::PoweredOff,
            next_connection: 1,
            active_connection: None,
            devices: vec![BleDevice {
                addr: [1, 2, 3, 4, 5, 6],
                rssi: -42,
                name: b"edgerun-ble".to_vec(),
            }],
            services: vec![BleService {
                uuid: [0x18, 0x0f, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                handle: 1,
            }],
            characteristics: vec![BleCharacteristic {
                uuid: [0x2a, 0x19, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                handle: 2,
                properties: 0x12,
            }],
            values: vec![(2, b"100".to_vec())],
            notifications: Vec::new(),
        }
    }

    pub fn init(&mut self) -> BleInitFuture {
        self.state = BleState::PoweredOn;
        BleInitFuture {
            result: Some(Ok(())),
        }
    }

    pub fn scan(&self, duration: u16) -> BleScanFuture {
        let result = if self.state == BleState::PoweredOn && duration != 0 {
            Ok(self.devices.clone())
        } else {
            Err(())
        };
        BleScanFuture {
            result: Some(result),
        }
    }

    pub fn connect(&mut self, addr: &[u8]) -> BleConnectFuture {
        let result = if self.state == BleState::PoweredOn
            && self.devices.iter().any(|device| device.addr == addr)
        {
            let handle = self.next_connection;
            self.next_connection = self.next_connection.saturating_add(1);
            self.active_connection = Some(handle);
            self.state = BleState::Connected;
            Ok(handle)
        } else {
            Err(())
        };
        BleConnectFuture {
            result: Some(result),
        }
    }

    pub fn disconnect(&mut self, handle: u16) -> BleDisconnectFuture {
        let result = if self.active_connection == Some(handle) {
            self.active_connection = None;
            self.state = BleState::Disconnected;
            Ok(())
        } else {
            Err(())
        };
        BleDisconnectFuture {
            result: Some(result),
        }
    }

    pub fn discover_services(&self, handle: u16) -> BleDiscoverSvcsFuture {
        BleDiscoverSvcsFuture {
            result: Some(if self.active_connection == Some(handle) {
                Ok(self.services.clone())
            } else {
                Err(())
            }),
        }
    }

    pub fn discover_chars(&self, handle: u16, svc: u16) -> BleDiscoverCharsFuture {
        BleDiscoverCharsFuture {
            result: Some(
                if self.active_connection == Some(handle)
                    && self.services.iter().any(|service| service.handle == svc)
                {
                    Ok(self.characteristics.clone())
                } else {
                    Err(())
                },
            ),
        }
    }

    pub fn read(&self, handle: u16) -> BleReadFuture {
        BleReadFuture {
            result: Some(
                self.values
                    .iter()
                    .find(|(value_handle, _)| *value_handle == handle)
                    .map(|(_, data)| data.clone())
                    .ok_or(()),
            ),
        }
    }

    pub fn write(&mut self, handle: u16, data: &[u8]) -> BleWriteFuture {
        let result = if let Some((_, value)) = self
            .values
            .iter_mut()
            .find(|(value_handle, _)| *value_handle == handle)
        {
            *value = data.to_vec();
            Ok(())
        } else {
            self.values.push((handle, data.to_vec()));
            Ok(())
        };
        BleWriteFuture {
            result: Some(result),
        }
    }

    pub fn notify(&mut self, handle: u16, enable: bool) -> BleNotifyFuture {
        let result = if self
            .values
            .iter()
            .any(|(value_handle, _)| *value_handle == handle)
        {
            if enable && !self.notifications.contains(&handle) {
                self.notifications.push(handle);
            } else if !enable {
                self.notifications.retain(|h| *h != handle);
            }
            Ok(())
        } else {
            Err(())
        };
        BleNotifyFuture {
            result: Some(result),
        }
    }

    #[must_use]
    pub fn state(&self) -> BleState {
        self.state
    }
}

impl Default for BleClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BleInitFuture {
    result: Option<Result<(), ()>>,
}
impl Future for BleInitFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct BleScanFuture {
    result: Option<Result<Vec<BleDevice>, ()>>,
}
impl Future for BleScanFuture {
    type Output = Result<Vec<BleDevice>, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct BleConnectFuture {
    result: Option<Result<u16, ()>>,
}
impl Future for BleConnectFuture {
    type Output = Result<u16, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct BleDisconnectFuture {
    result: Option<Result<(), ()>>,
}
impl Future for BleDisconnectFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct BleDiscoverSvcsFuture {
    result: Option<Result<Vec<BleService>, ()>>,
}
impl Future for BleDiscoverSvcsFuture {
    type Output = Result<Vec<BleService>, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct BleDiscoverCharsFuture {
    result: Option<Result<Vec<BleCharacteristic>, ()>>,
}
impl Future for BleDiscoverCharsFuture {
    type Output = Result<Vec<BleCharacteristic>, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct BleReadFuture {
    result: Option<Result<Vec<u8>, ()>>,
}
impl Future for BleReadFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct BleWriteFuture {
    result: Option<Result<(), ()>>,
}
impl Future for BleWriteFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct BleNotifyFuture {
    result: Option<Result<(), ()>>,
}
impl Future for BleNotifyFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;

    #[test]
    fn ble_scan_connect_read_write_complete() {
        let mut client = BleClient::new();

        crate::block_on(Box::pin(client.init())).unwrap();
        let devices = crate::block_on(Box::pin(client.scan(10))).unwrap();
        let conn = crate::block_on(Box::pin(client.connect(&devices[0].addr))).unwrap();
        let services = crate::block_on(Box::pin(client.discover_services(conn))).unwrap();
        let chars =
            crate::block_on(Box::pin(client.discover_chars(conn, services[0].handle))).unwrap();
        crate::block_on(Box::pin(client.write(chars[0].handle, b"42"))).unwrap();
        let value = crate::block_on(Box::pin(client.read(chars[0].handle))).unwrap();

        assert_eq!(client.state(), BleState::Connected);
        assert_eq!(value, b"42");
    }
}
