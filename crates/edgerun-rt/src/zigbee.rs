//! Zigbee client

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZbDeviceType {
    OnOffSwitch,
    Dimmer,
    TemperatureSensor,
    HumiditySensor,
    DoorLock,
    Thermostat,
    ColorLight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZbDeviceState {
    Added,
    Removed,
    Online,
    Offline,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZbDevice {
    pub short_id: u16,
    pub ieee_addr: [u8; 8],
    pub device_type: ZbDeviceType,
    pub state: ZbDeviceState,
    pub endpoint: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZbCluster {
    pub id: u16,
    pub attr_id: u16,
    pub value: Vec<u8>,
}

pub struct ZbClient {
    initialized: bool,
    permit_join_seconds: u8,
    devices: Vec<ZbDevice>,
    attrs: Vec<([u8; 8], u8, u16, u16, Vec<u8>)>,
}

impl ZbClient {
    pub fn new() -> Self {
        let ieee_addr = [0x10, 0x32, 0x54, 0x76, 0x98, 0xba, 0xdc, 0xfe];
        Self {
            initialized: false,
            permit_join_seconds: 0,
            devices: vec![ZbDevice {
                short_id: 1,
                ieee_addr,
                device_type: ZbDeviceType::ColorLight,
                state: ZbDeviceState::Online,
                endpoint: 1,
            }],
            attrs: vec![
                (ieee_addr, 1, 0x0006, 0x0000, vec![0]),
                (ieee_addr, 1, 0x0008, 0x0000, vec![0]),
                (ieee_addr, 1, 0x0300, 0x0000, vec![0, 0]),
            ],
        }
    }

    pub fn init(&mut self, channel: u8) -> ZbInitFuture {
        let result = if (11..=26).contains(&channel) {
            self.initialized = true;
            Ok(())
        } else {
            Err(())
        };
        ZbInitFuture {
            result: Some(result),
        }
    }

    pub fn permit_join(&mut self, duration: u8) -> ZbPermitJoinFuture {
        let result = if self.initialized {
            self.permit_join_seconds = duration;
            Ok(())
        } else {
            Err(())
        };
        ZbPermitJoinFuture {
            result: Some(result),
        }
    }

    pub fn devices(&self) -> ZbDevicesFuture {
        ZbDevicesFuture {
            result: Some(if self.initialized {
                Ok(self.devices.clone())
            } else {
                Err(())
            }),
        }
    }

    pub fn remove(&mut self, ieee: &[u8]) -> ZbRemoveFuture {
        let result = if self.initialized {
            self.devices.retain(|device| device.ieee_addr != ieee);
            Ok(())
        } else {
            Err(())
        };
        ZbRemoveFuture {
            result: Some(result),
        }
    }

    pub fn read_attr(&self, ieee: &[u8], ep: u8, cluster: u16, attr: u16) -> ZbReadAttrFuture {
        ZbReadAttrFuture {
            result: Some(self.attr_value(ieee, ep, cluster, attr).cloned().ok_or(())),
        }
    }

    pub fn write_attr(
        &mut self,
        ieee: &[u8],
        ep: u8,
        cluster: u16,
        attr: u16,
        value: &[u8],
    ) -> ZbWriteAttrFuture {
        let result = self.set_attr(ieee, ep, cluster, attr, value);
        ZbWriteAttrFuture {
            result: Some(result),
        }
    }

    pub fn send_onoff(&mut self, ieee: &[u8], ep: u8, on: bool) -> ZbOnOffFuture {
        ZbOnOffFuture {
            result: Some(self.set_attr(ieee, ep, 0x0006, 0x0000, &[u8::from(on)])),
        }
    }

    pub fn level(&mut self, ieee: &[u8], ep: u8, level: u8) -> ZbLevelFuture {
        ZbLevelFuture {
            result: Some(self.set_attr(ieee, ep, 0x0008, 0x0000, &[level])),
        }
    }

    pub fn color(&mut self, ieee: &[u8], ep: u8, hue: u8, sat: u8) -> ZbColorFuture {
        ZbColorFuture {
            result: Some(self.set_attr(ieee, ep, 0x0300, 0x0000, &[hue, sat])),
        }
    }

    #[must_use]
    pub fn permit_join_seconds(&self) -> u8 {
        self.permit_join_seconds
    }

    fn attr_value(&self, ieee: &[u8], ep: u8, cluster: u16, attr: u16) -> Option<&Vec<u8>> {
        self.attrs
            .iter()
            .find(|(addr, endpoint, cluster_id, attr_id, _)| {
                addr == ieee && *endpoint == ep && *cluster_id == cluster && *attr_id == attr
            })
            .map(|(_, _, _, _, value)| value)
    }

    fn set_attr(
        &mut self,
        ieee: &[u8],
        ep: u8,
        cluster: u16,
        attr: u16,
        value: &[u8],
    ) -> Result<(), ()> {
        if !self.initialized || !self.devices.iter().any(|device| device.ieee_addr == ieee) {
            return Err(());
        }
        if let Some((_, _, _, _, stored)) =
            self.attrs
                .iter_mut()
                .find(|(addr, endpoint, cluster_id, attr_id, _)| {
                    addr == ieee && *endpoint == ep && *cluster_id == cluster && *attr_id == attr
                })
        {
            *stored = value.to_vec();
        } else {
            let mut addr = [0; 8];
            addr.copy_from_slice(&ieee[..8]);
            self.attrs.push((addr, ep, cluster, attr, value.to_vec()));
        }
        Ok(())
    }
}

impl Default for ZbClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ZbInitFuture {
    result: Option<Result<(), ()>>,
}
impl Future for ZbInitFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct ZbPermitJoinFuture {
    result: Option<Result<(), ()>>,
}
impl Future for ZbPermitJoinFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct ZbDevicesFuture {
    result: Option<Result<Vec<ZbDevice>, ()>>,
}
impl Future for ZbDevicesFuture {
    type Output = Result<Vec<ZbDevice>, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct ZbRemoveFuture {
    result: Option<Result<(), ()>>,
}
impl Future for ZbRemoveFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct ZbReadAttrFuture {
    result: Option<Result<Vec<u8>, ()>>,
}
impl Future for ZbReadAttrFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct ZbWriteAttrFuture {
    result: Option<Result<(), ()>>,
}
impl Future for ZbWriteAttrFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct ZbOnOffFuture {
    result: Option<Result<(), ()>>,
}
impl Future for ZbOnOffFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct ZbLevelFuture {
    result: Option<Result<(), ()>>,
}
impl Future for ZbLevelFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct ZbColorFuture {
    result: Option<Result<(), ()>>,
}
impl Future for ZbColorFuture {
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
    fn zigbee_init_join_and_attribute_commands_complete() {
        let mut client = ZbClient::new();

        crate::block_on(Box::pin(client.init(15))).unwrap();
        crate::block_on(Box::pin(client.permit_join(60))).unwrap();
        let devices = crate::block_on(Box::pin(client.devices())).unwrap();
        let device = &devices[0];
        crate::block_on(Box::pin(client.send_onoff(
            &device.ieee_addr,
            device.endpoint,
            true,
        )))
        .unwrap();
        crate::block_on(Box::pin(client.level(
            &device.ieee_addr,
            device.endpoint,
            128,
        )))
        .unwrap();
        let onoff = crate::block_on(Box::pin(client.read_attr(
            &device.ieee_addr,
            device.endpoint,
            0x0006,
            0x0000,
        )))
        .unwrap();

        assert_eq!(client.permit_join_seconds(), 60);
        assert_eq!(onoff, vec![1]);
    }
}
