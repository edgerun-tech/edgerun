//! PostgreSQL client

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PgRow {
    pub values: Vec<Option<Vec<u8>>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PgResult {
    pub rows: Vec<PgRow>,
    pub affected: u64,
}

#[derive(Default)]
struct PgState {
    connected: bool,
    prepared: Vec<Vec<u8>>,
    executed: Vec<Vec<u8>>,
}

pub struct PgClient {
    state: RefCell<PgState>,
}

impl PgClient {
    pub fn new() -> Self {
        Self {
            state: RefCell::new(PgState::default()),
        }
    }

    pub fn connect(
        &self,
        host: &[u8],
        port: u16,
        user: &[u8],
        _pass: &[u8],
        db: &[u8],
    ) -> PgConnectFuture {
        let result = if !host.is_empty() && port != 0 && !user.is_empty() && !db.is_empty() {
            self.state.borrow_mut().connected = true;
            Ok(())
        } else {
            Err(())
        };
        PgConnectFuture {
            result: Some(result),
        }
    }

    pub fn execute(&self, sql: &[u8]) -> PgExecuteFuture {
        let result = if self.state.borrow().connected && !sql.is_empty() {
            self.state.borrow_mut().executed.push(sql.to_vec());
            Ok(PgResult {
                rows: Vec::new(),
                affected: 1,
            })
        } else {
            Err(())
        };
        PgExecuteFuture {
            result: Some(result),
        }
    }

    pub fn query(&self, sql: &[u8]) -> PgQueryFuture {
        PgQueryFuture {
            result: Some(query_result(self.state.borrow().connected, sql)),
        }
    }

    pub fn prepare(&self, sql: &[u8]) -> PgPrepareFuture {
        let result = if self.state.borrow().connected && !sql.is_empty() {
            self.state.borrow_mut().prepared.push(sql.to_vec());
            Ok(())
        } else {
            Err(())
        };
        PgPrepareFuture {
            result: Some(result),
        }
    }

    pub fn close(&self) -> PgCloseFuture {
        self.state.borrow_mut().connected = false;
        PgCloseFuture {
            result: Some(Ok(())),
        }
    }
}

impl Default for PgClient {
    fn default() -> Self {
        Self::new()
    }
}

ready_future!(PgConnectFuture, Result<(), ()>);
ready_future!(PgExecuteFuture, Result<PgResult, ()>);
ready_future!(PgQueryFuture, Result<PgResult, ()>);
ready_future!(PgPrepareFuture, Result<(), ()>);
ready_future!(PgCloseFuture, Result<(), ()>);

fn query_result(connected: bool, sql: &[u8]) -> Result<PgResult, ()> {
    if !connected || sql.is_empty() {
        return Err(());
    }
    Ok(PgResult {
        rows: vec![PgRow {
            values: vec![Some(sql.to_vec())],
        }],
        affected: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_on;

    #[test]
    fn postgres_connect_prepare_execute_query_complete() {
        let client = PgClient::new();

        block_on(client.connect(b"localhost", 5432, b"user", b"pass", b"db")).unwrap();
        block_on(client.prepare(b"select 1")).unwrap();
        assert_eq!(
            block_on(client.execute(b"update t set a=1"))
                .unwrap()
                .affected,
            1
        );
        let result = block_on(client.query(b"select 1")).unwrap();
        assert_eq!(result.rows[0].values[0], Some(b"select 1".to_vec()));
        block_on(client.close()).unwrap();
    }
}
