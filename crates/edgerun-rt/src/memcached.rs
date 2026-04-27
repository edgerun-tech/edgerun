//! Memcached client

extern crate alloc;

use alloc::string::ToString;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    key: Vec<u8>,
    value: Vec<u8>,
    flags: u32,
    cas: u64,
}

pub struct MemcachedClient {
    connected: bool,
    next_cas: u64,
    entries: Vec<Entry>,
}

impl MemcachedClient {
    pub fn new() -> Self {
        Self {
            connected: false,
            next_cas: 1,
            entries: Vec::new(),
        }
    }

    pub fn connect(&mut self, host: &[u8], port: u16) -> McConnectFuture {
        let result = if !host.is_empty() && port != 0 {
            self.connected = true;
            Ok(())
        } else {
            Err(())
        };
        McConnectFuture {
            result: Some(result),
        }
    }

    pub fn get(&self, key: &[u8]) -> McGetFuture {
        McGetFuture {
            result: Some(if self.connected {
                Ok(self
                    .entries
                    .iter()
                    .find(|entry| entry.key == key)
                    .map(|entry| (entry.value.clone(), entry.flags)))
            } else {
                Err(())
            }),
        }
    }

    pub fn set(&mut self, key: &[u8], value: &[u8], flags: u32, _ttl: u32) -> McSetFuture {
        McSetFuture {
            result: Some(self.store(key, value, flags, StoreMode::Set)),
        }
    }

    pub fn add(&mut self, key: &[u8], value: &[u8], flags: u32, _ttl: u32) -> McSetFuture {
        McSetFuture {
            result: Some(self.store(key, value, flags, StoreMode::Add)),
        }
    }

    pub fn replace(&mut self, key: &[u8], value: &[u8], flags: u32, _ttl: u32) -> McSetFuture {
        McSetFuture {
            result: Some(self.store(key, value, flags, StoreMode::Replace)),
        }
    }

    pub fn append(&mut self, key: &[u8], value: &[u8]) -> McAppendFuture {
        let result = self.append_value(key, value, false);
        McAppendFuture {
            result: Some(result),
        }
    }

    pub fn prepend(&mut self, key: &[u8], value: &[u8]) -> McAppendFuture {
        let result = self.append_value(key, value, true);
        McAppendFuture {
            result: Some(result),
        }
    }

    pub fn delete(&mut self, key: &[u8]) -> McDeleteFuture {
        let result = if self.connected {
            let before = self.entries.len();
            self.entries.retain(|entry| entry.key != key);
            Ok(before != self.entries.len())
        } else {
            Err(())
        };
        McDeleteFuture {
            result: Some(result),
        }
    }

    pub fn incr(&mut self, key: &[u8], value: u64) -> McIncrFuture {
        McIncrFuture {
            result: Some(self.update_counter(key, value, true)),
        }
    }

    pub fn decr(&mut self, key: &[u8], value: u64) -> McIncrFuture {
        McIncrFuture {
            result: Some(self.update_counter(key, value, false)),
        }
    }

    pub fn gets(&self, keys: &[&[u8]]) -> McGetsFuture {
        let result = if self.connected {
            Ok(keys
                .iter()
                .filter_map(|key| {
                    self.entries
                        .iter()
                        .find(|entry| entry.key == *key)
                        .map(|entry| (entry.key.clone(), entry.value.clone(), entry.flags))
                })
                .collect())
        } else {
            Err(())
        };
        McGetsFuture {
            result: Some(result),
        }
    }

    pub fn cas(
        &mut self,
        key: &[u8],
        value: &[u8],
        flags: u32,
        _ttl: u32,
        cas: u64,
    ) -> McCasFuture {
        let result = if !self.connected {
            Err(())
        } else if let Some(pos) = self.entries.iter().position(|entry| entry.key == key) {
            if self.entries[pos].cas == cas {
                let next_cas = self.take_cas();
                self.entries[pos] = Entry {
                    key: key.to_vec(),
                    value: value.to_vec(),
                    flags,
                    cas: next_cas,
                };
                Ok(())
            } else {
                Err(())
            }
        } else {
            Err(())
        };
        McCasFuture {
            result: Some(result),
        }
    }

    pub fn stats(&self) -> McStatsFuture {
        let result = if self.connected {
            Ok(alloc::vec![
                (
                    b"curr_items".to_vec(),
                    self.entries.len().to_string().into_bytes()
                ),
                (b"version".to_vec(), b"edgerun-rt-memcached/0".to_vec()),
            ])
        } else {
            Err(())
        };
        McStatsFuture {
            result: Some(result),
        }
    }

    pub fn flush(&mut self, _delay: u32) -> McFlushFuture {
        if self.connected {
            self.entries.clear();
        }
        McFlushFuture {
            result: Some(if self.connected { Ok(()) } else { Err(()) }),
        }
    }

    pub fn version(&self) -> McVersionFuture {
        McVersionFuture {
            result: Some(if self.connected {
                Ok(b"edgerun-rt-memcached/0".to_vec())
            } else {
                Err(())
            }),
        }
    }

    pub fn quit(&mut self) -> McQuitFuture {
        self.connected = false;
        McQuitFuture {
            result: Some(Ok(())),
        }
    }

    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    fn store(&mut self, key: &[u8], value: &[u8], flags: u32, mode: StoreMode) -> Result<(), ()> {
        if !self.connected || key.is_empty() {
            return Err(());
        }
        match self.entries.iter().position(|entry| entry.key == key) {
            Some(_) if mode == StoreMode::Add => Err(()),
            Some(pos) => {
                let cas = self.take_cas();
                self.entries[pos] = Entry {
                    key: key.to_vec(),
                    value: value.to_vec(),
                    flags,
                    cas,
                };
                Ok(())
            }
            None if mode == StoreMode::Replace => Err(()),
            None => {
                let cas = self.take_cas();
                self.entries.push(Entry {
                    key: key.to_vec(),
                    value: value.to_vec(),
                    flags,
                    cas,
                });
                Ok(())
            }
        }
    }

    fn append_value(&mut self, key: &[u8], value: &[u8], prepend: bool) -> Result<(), ()> {
        if !self.connected {
            return Err(());
        }
        let cas = self.take_cas();
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.key == key) {
            if prepend {
                let mut next = value.to_vec();
                next.extend_from_slice(&entry.value);
                entry.value = next;
            } else {
                entry.value.extend_from_slice(value);
            }
            entry.cas = cas;
            Ok(())
        } else {
            Err(())
        }
    }

    fn update_counter(&mut self, key: &[u8], value: u64, increment: bool) -> Result<u64, ()> {
        if !self.connected {
            return Err(());
        }
        let cas = self.take_cas();
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.key == key) {
            let current = parse_u64(&entry.value)?;
            let next = if increment {
                current.saturating_add(value)
            } else {
                current.saturating_sub(value)
            };
            entry.value = next.to_string().into_bytes();
            entry.cas = cas;
            Ok(next)
        } else {
            Err(())
        }
    }

    fn take_cas(&mut self) -> u64 {
        let cas = self.next_cas;
        self.next_cas = self.next_cas.saturating_add(1);
        cas
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StoreMode {
    Set,
    Add,
    Replace,
}

fn parse_u64(bytes: &[u8]) -> Result<u64, ()> {
    let mut value = 0u64;
    for byte in bytes {
        if !byte.is_ascii_digit() {
            return Err(());
        }
        value = value
            .saturating_mul(10)
            .saturating_add(u64::from(byte - b'0'));
    }
    Ok(value)
}

impl Default for MemcachedClient {
    fn default() -> Self {
        Self::new()
    }
}

macro_rules! ready_future {
    ($name:ident, $output:ty, $fallback:expr) => {
        pub struct $name {
            result: Option<$output>,
        }

        impl Future for $name {
            type Output = $output;

            fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
                Poll::Ready(self.result.take().unwrap_or($fallback))
            }
        }
    };
}

ready_future!(McConnectFuture, Result<(), ()>, Ok(()));
ready_future!(McGetFuture, Result<Option<(Vec<u8>, u32)>, ()>, Err(()));
ready_future!(McSetFuture, Result<(), ()>, Ok(()));
ready_future!(McAppendFuture, Result<(), ()>, Ok(()));
ready_future!(McDeleteFuture, Result<bool, ()>, Err(()));
ready_future!(McIncrFuture, Result<u64, ()>, Err(()));
ready_future!(
    McGetsFuture,
    Result<Vec<(Vec<u8>, Vec<u8>, u32)>, ()>,
    Err(())
);
ready_future!(McCasFuture, Result<(), ()>, Ok(()));
ready_future!(McStatsFuture, Result<Vec<(Vec<u8>, Vec<u8>)>, ()>, Err(()));
ready_future!(McFlushFuture, Result<(), ()>, Ok(()));
ready_future!(McVersionFuture, Result<Vec<u8>, ()>, Err(()));
ready_future!(McQuitFuture, Result<(), ()>, Ok(()));

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;

    #[test]
    fn memcached_store_append_counter_and_flush_complete() {
        let mut client = MemcachedClient::new();

        crate::block_on(Box::pin(client.connect(b"host", 11211))).unwrap();
        crate::block_on(Box::pin(client.set(b"k", b"v", 7, 0))).unwrap();
        crate::block_on(Box::pin(client.append(b"k", b"2"))).unwrap();
        let item = crate::block_on(Box::pin(client.get(b"k")))
            .unwrap()
            .unwrap();
        crate::block_on(Box::pin(client.set(b"n", b"1", 0, 0))).unwrap();
        let n = crate::block_on(Box::pin(client.incr(b"n", 41))).unwrap();
        let stats = crate::block_on(Box::pin(client.stats())).unwrap();
        crate::block_on(Box::pin(client.flush(0))).unwrap();

        assert_eq!(item, (b"v2".to_vec(), 7));
        assert_eq!(n, 42);
        assert_eq!(stats[0].0, b"curr_items");
        assert_eq!(crate::block_on(Box::pin(client.get(b"k"))).unwrap(), None);
    }
}
