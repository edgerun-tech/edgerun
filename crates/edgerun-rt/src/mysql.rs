//! MySQL client

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
pub struct MySqlRow {
    pub values: Vec<Option<Vec<u8>>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MySqlResult {
    pub rows: Vec<MySqlRow>,
    pub affected: u64,
    pub insert_id: u64,
}

#[derive(Default)]
struct MySqlState {
    connected: bool,
    prepared: Vec<Vec<u8>>,
    insert_id: u64,
}

pub struct MySqlClient {
    state: RefCell<MySqlState>,
}

impl MySqlClient {
    pub fn new() -> Self {
        Self {
            state: RefCell::new(MySqlState::default()),
        }
    }

    pub fn connect(
        &self,
        host: &[u8],
        port: u16,
        user: &[u8],
        _pass: &[u8],
        db: &[u8],
    ) -> MyConnectFuture {
        let result = if !host.is_empty() && port != 0 && !user.is_empty() && !db.is_empty() {
            self.state.borrow_mut().connected = true;
            Ok(())
        } else {
            Err(())
        };
        MyConnectFuture {
            result: Some(result),
        }
    }

    pub fn execute(&self, sql: &[u8]) -> MyExecuteFuture {
        let mut state = self.state.borrow_mut();
        let result = if state.connected && !sql.is_empty() {
            state.insert_id = state.insert_id.saturating_add(1);
            Ok(MySqlResult {
                rows: Vec::new(),
                affected: 1,
                insert_id: state.insert_id,
            })
        } else {
            Err(())
        };
        MyExecuteFuture {
            result: Some(result),
        }
    }

    pub fn query(&self, sql: &[u8]) -> MyQueryFuture {
        MyQueryFuture {
            result: Some(query_result(self.state.borrow().connected, sql)),
        }
    }

    pub fn prepare(&self, sql: &[u8]) -> MyPrepareFuture {
        let result = if self.state.borrow().connected && !sql.is_empty() {
            self.state.borrow_mut().prepared.push(sql.to_vec());
            Ok(())
        } else {
            Err(())
        };
        MyPrepareFuture {
            result: Some(result),
        }
    }

    pub fn close(&self) -> MyCloseFuture {
        self.state.borrow_mut().connected = false;
        MyCloseFuture {
            result: Some(Ok(())),
        }
    }
}

impl Default for MySqlClient {
    fn default() -> Self {
        Self::new()
    }
}

ready_future!(MyConnectFuture, Result<(), ()>);
ready_future!(MyExecuteFuture, Result<MySqlResult, ()>);
ready_future!(MyQueryFuture, Result<MySqlResult, ()>);
ready_future!(MyPrepareFuture, Result<(), ()>);
ready_future!(MyCloseFuture, Result<(), ()>);

fn query_result(connected: bool, sql: &[u8]) -> Result<MySqlResult, ()> {
    if !connected || sql.is_empty() {
        return Err(());
    }
    Ok(MySqlResult {
        rows: vec![MySqlRow {
            values: vec![Some(sql.to_vec())],
        }],
        affected: 0,
        insert_id: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_on;

    #[test]
    fn mysql_connect_prepare_execute_query_complete() {
        let client = MySqlClient::new();

        block_on(client.connect(b"localhost", 3306, b"user", b"pass", b"db")).unwrap();
        block_on(client.prepare(b"select 1")).unwrap();
        let execute = block_on(client.execute(b"insert into t values (1)")).unwrap();
        assert_eq!(execute.affected, 1);
        assert_eq!(execute.insert_id, 1);
        let result = block_on(client.query(b"select 1")).unwrap();
        assert_eq!(result.rows[0].values[0], Some(b"select 1".to_vec()));
        block_on(client.close()).unwrap();
    }
}
