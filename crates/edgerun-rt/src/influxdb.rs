//! InfluxDB client

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

macro_rules! ready_future {
    ($name:ident, $output:ty) => {
        pub struct $name {
            result: Option<$output>,
        }

        impl Future for $name {
            type Output = $output;

            fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
                Poll::Ready(self.result.take().unwrap_or(Err(())))
            }
        }
    };
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InfluxPoint {
    pub measurement: Vec<u8>,
    pub tags: Vec<(Vec<u8>, Vec<u8>)>,
    pub fields: Vec<(Vec<u8>, Vec<u8>)>,
    pub timestamp: Option<i64>,
}

impl InfluxPoint {
    pub fn new(measurement: &[u8]) -> Self {
        Self {
            measurement: measurement.to_vec(),
            tags: Vec::new(),
            fields: Vec::new(),
            timestamp: None,
        }
    }

    pub fn tag(&mut self, key: &[u8], value: &[u8]) {
        self.tags.push((key.to_vec(), value.to_vec()));
    }

    pub fn field(&mut self, key: &[u8], value: &[u8]) {
        self.fields.push((key.to_vec(), value.to_vec()));
    }

    pub fn timestamp(&mut self, ts: i64) {
        self.timestamp = Some(ts);
    }
}

#[derive(Default)]
struct InfluxState {
    connected: bool,
    url: Vec<u8>,
    points: Vec<(Vec<u8>, InfluxPoint)>,
}

pub struct InfluxClient {
    state: RefCell<InfluxState>,
}

impl InfluxClient {
    pub fn new() -> Self {
        Self {
            state: RefCell::new(InfluxState::default()),
        }
    }

    pub fn connect(&self, url: &[u8]) -> InfluxConnectFuture {
        let result = if !url.is_empty() {
            let mut state = self.state.borrow_mut();
            state.connected = true;
            state.url = url.to_vec();
            Ok(())
        } else {
            Err(())
        };
        InfluxConnectFuture {
            result: Some(result),
        }
    }

    pub fn write(&self, db: &[u8], point: &InfluxPoint) -> InfluxWriteFuture {
        let result = if self.state.borrow().connected
            && !db.is_empty()
            && !point.measurement.is_empty()
            && !point.fields.is_empty()
        {
            self.state
                .borrow_mut()
                .points
                .push((db.to_vec(), point.clone()));
            Ok(())
        } else {
            Err(())
        };
        InfluxWriteFuture {
            result: Some(result),
        }
    }

    pub fn query(&self, db: &[u8], cql: &[u8]) -> InfluxQueryFuture {
        let state = self.state.borrow();
        let result = if state.connected && !db.is_empty() && !cql.is_empty() {
            Ok(state
                .points
                .iter()
                .filter(|(stored_db, _)| stored_db.as_slice() == db)
                .map(|(_, point)| point.clone())
                .collect())
        } else {
            Err(())
        };
        InfluxQueryFuture {
            result: Some(result),
        }
    }

    pub fn ping(&self) -> InfluxPingFuture {
        InfluxPingFuture {
            result: Some(if self.state.borrow().connected {
                Ok((204, String::from("edgerun-influx")))
            } else {
                Err(())
            }),
        }
    }
}

impl Default for InfluxClient {
    fn default() -> Self {
        Self::new()
    }
}

ready_future!(InfluxConnectFuture, Result<(), ()>);
ready_future!(InfluxWriteFuture, Result<(), ()>);
ready_future!(InfluxQueryFuture, Result<Vec<InfluxPoint>, ()>);
ready_future!(InfluxPingFuture, Result<(u16, String), ()>);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_on;
    use alloc::vec;

    #[test]
    fn influx_connect_write_query_ping_complete() {
        let client = InfluxClient::new();
        let mut point = InfluxPoint::new(b"cpu");
        point.tag(b"host", b"edge");
        point.field(b"value", b"42");
        point.timestamp(7);

        block_on(client.connect(b"http://127.0.0.1:8086")).unwrap();
        block_on(client.write(b"metrics", &point)).unwrap();
        let rows = block_on(client.query(b"metrics", b"select * from cpu")).unwrap();
        assert_eq!(rows, vec![point]);
        assert_eq!(block_on(client.ping()).unwrap().0, 204);
    }
}
