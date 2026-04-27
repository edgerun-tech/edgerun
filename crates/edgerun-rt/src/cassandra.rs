//! Cassandra client

extern crate alloc;

use alloc::vec;
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

#[derive(Default)]
struct CassandraState {
    connected: bool,
    prepared: Vec<Vec<u8>>,
    executed: Vec<Vec<u8>>,
}

pub struct CassandraClient {
    state: RefCell<CassandraState>,
}

impl CassandraClient {
    pub fn new() -> Self {
        Self {
            state: RefCell::new(CassandraState::default()),
        }
    }

    pub fn connect(&self, hosts: &[&[u8]]) -> CqlConnectFuture {
        let result = if hosts.iter().any(|host| !host.is_empty()) {
            self.state.borrow_mut().connected = true;
            Ok(())
        } else {
            Err(())
        };
        CqlConnectFuture {
            result: Some(result),
        }
    }

    pub fn execute(&self, cql: &[u8], params: &[&[u8]]) -> CqlExecuteFuture {
        let result = if self.state.borrow().connected && !cql.is_empty() {
            let mut executed = cql.to_vec();
            for param in params {
                executed.extend_from_slice(param);
            }
            self.state.borrow_mut().executed.push(executed);
            Ok(())
        } else {
            Err(())
        };
        CqlExecuteFuture {
            result: Some(result),
        }
    }

    pub fn prepare(&self, cql: &[u8]) -> CqlPrepareFuture {
        let result = if self.state.borrow().connected && !cql.is_empty() {
            self.state.borrow_mut().prepared.push(cql.to_vec());
            Ok(())
        } else {
            Err(())
        };
        CqlPrepareFuture {
            result: Some(result),
        }
    }

    pub fn query(&self, cql: &[u8]) -> CqlQueryFuture {
        CqlQueryFuture {
            result: Some(if self.state.borrow().connected && !cql.is_empty() {
                Ok(vec![cql.to_vec()])
            } else {
                Err(())
            }),
        }
    }

    pub fn close(&self) -> CqlCloseFuture {
        self.state.borrow_mut().connected = false;
        CqlCloseFuture {
            result: Some(Ok(())),
        }
    }
}

impl Default for CassandraClient {
    fn default() -> Self {
        Self::new()
    }
}

ready_future!(CqlConnectFuture, Result<(), ()>);
ready_future!(CqlExecuteFuture, Result<(), ()>);
ready_future!(CqlPrepareFuture, Result<(), ()>);
ready_future!(CqlQueryFuture, Result<Vec<Vec<u8>>, ()>);
ready_future!(CqlCloseFuture, Result<(), ()>);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_on;

    #[test]
    fn cassandra_connect_prepare_execute_query_complete() {
        let client = CassandraClient::new();

        block_on(client.connect(&[b"127.0.0.1".as_slice()])).unwrap();
        block_on(client.prepare(b"select * from ks.tbl")).unwrap();
        block_on(client.execute(b"insert into ks.tbl (id) values (?)", &[b"1".as_slice()]))
            .unwrap();
        assert_eq!(
            block_on(client.query(b"select * from ks.tbl")).unwrap()[0],
            b"select * from ks.tbl"
        );
        block_on(client.close()).unwrap();
    }
}
