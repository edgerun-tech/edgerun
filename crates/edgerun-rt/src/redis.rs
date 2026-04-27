//! Redis client

extern crate alloc;

use alloc::string::ToString;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RedisType {
    String,
    List,
    Set,
    Zset,
    Hash,
    Stream,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedisValue {
    pub rtype: RedisType,
    pub data: Vec<u8>,
}

impl RedisValue {
    pub fn string(data: Vec<u8>) -> Self {
        Self {
            rtype: RedisType::String,
            data,
        }
    }

    pub fn is_string(&self) -> bool {
        matches!(self.rtype, RedisType::String)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RedisEntryValue {
    String(Vec<u8>),
    List(Vec<Vec<u8>>),
    Set(Vec<Vec<u8>>),
    Hash(Vec<(Vec<u8>, Vec<u8>)>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RedisEntry {
    key: Vec<u8>,
    value: RedisEntryValue,
    ttl: Option<u64>,
}

#[derive(Default)]
struct RedisState {
    connected: bool,
    authenticated: bool,
    entries: Vec<RedisEntry>,
}

pub struct RedisClient {
    state: RefCell<RedisState>,
}

impl RedisClient {
    pub fn new() -> Self {
        Self {
            state: RefCell::new(RedisState::default()),
        }
    }

    pub fn connect(&self, host: &[u8], port: u16) -> RedisConnectFuture {
        let result = if !host.is_empty() && port != 0 {
            self.state.borrow_mut().connected = true;
            Ok(())
        } else {
            Err(())
        };
        RedisConnectFuture {
            result: Some(result),
        }
    }

    pub fn auth(&self, pass: &[u8]) -> RedisAuthFuture {
        let result = if self.state.borrow().connected && !pass.is_empty() {
            self.state.borrow_mut().authenticated = true;
            Ok(())
        } else {
            Err(())
        };
        RedisAuthFuture {
            result: Some(result),
        }
    }

    pub fn get(&self, key: &[u8]) -> RedisGetFuture {
        let result = self.with_entry(key, |entry| match &entry.value {
            RedisEntryValue::String(data) => Ok(RedisValue::string(data.clone())),
            _ => Err(()),
        });
        RedisGetFuture {
            result: Some(result),
        }
    }

    pub fn set(&self, key: &[u8], value: &[u8]) -> RedisSetFuture {
        let result = self.set_string(key, value, None);
        RedisSetFuture {
            result: Some(result),
        }
    }

    pub fn set_ex(&self, key: &[u8], value: &[u8], ttl: u64) -> RedisSetFuture {
        let result = self.set_string(key, value, Some(ttl));
        RedisSetFuture {
            result: Some(result),
        }
    }

    pub fn del(&self, keys: &[&[u8]]) -> RedisDelFuture {
        let mut state = self.state.borrow_mut();
        let before = state.entries.len();
        state
            .entries
            .retain(|entry| !keys.iter().any(|key| *key == entry.key.as_slice()));
        RedisDelFuture {
            result: Some(Ok((before - state.entries.len()) as u64)),
        }
    }

    pub fn exists(&self, key: &[u8]) -> RedisExistsFuture {
        RedisExistsFuture {
            result: Some(Ok(self.entry_index(key).is_some())),
        }
    }

    pub fn expire(&self, key: &[u8], ttl: u64) -> RedisExpireFuture {
        let mut state = self.state.borrow_mut();
        let result = match find_entry_mut(&mut state.entries, key) {
            Some(entry) => {
                entry.ttl = Some(ttl);
                Ok(true)
            }
            None => Ok(false),
        };
        RedisExpireFuture {
            result: Some(result),
        }
    }

    pub fn ttl(&self, key: &[u8]) -> RedisTtlFuture {
        let result = self.with_entry(key, |entry| {
            Ok(entry.ttl.map(|ttl| ttl as i64).unwrap_or(-1))
        });
        RedisTtlFuture {
            result: Some(result.or(Ok(-2))),
        }
    }

    pub fn incr(&self, key: &[u8]) -> RedisIncrFuture {
        let result = self.add_integer(key, 1);
        RedisIncrFuture {
            result: Some(result),
        }
    }

    pub fn decr(&self, key: &[u8]) -> RedisIncrFuture {
        let result = self.add_integer(key, -1);
        RedisIncrFuture {
            result: Some(result),
        }
    }

    pub fn lpush(&self, key: &[u8], value: &[u8]) -> RedisLpushFuture {
        let result = self.push_list(key, value, true);
        RedisLpushFuture {
            result: Some(result),
        }
    }

    pub fn rpush(&self, key: &[u8], value: &[u8]) -> RedisLpushFuture {
        let result = self.push_list(key, value, false);
        RedisLpushFuture {
            result: Some(result),
        }
    }

    pub fn lpop(&self, key: &[u8]) -> RedisLpopFuture {
        let mut state = self.state.borrow_mut();
        let result = match find_entry_mut(&mut state.entries, key) {
            Some(entry) => match &mut entry.value {
                RedisEntryValue::List(values) => values
                    .first()
                    .cloned()
                    .map(|value| {
                        values.remove(0);
                        RedisValue::string(value)
                    })
                    .ok_or(()),
                _ => Err(()),
            },
            None => Err(()),
        };
        RedisLpopFuture {
            result: Some(result),
        }
    }

    pub fn lrange(&self, key: &[u8], start: i64, stop: i64) -> RedisLrangeFuture {
        let result = self.with_entry(key, |entry| match &entry.value {
            RedisEntryValue::List(values) => {
                let (start, stop) = normalized_range(values.len(), start, stop);
                Ok(values[start..stop]
                    .iter()
                    .cloned()
                    .map(RedisValue::string)
                    .collect())
            }
            _ => Err(()),
        });
        RedisLrangeFuture {
            result: Some(result),
        }
    }

    pub fn sadd(&self, key: &[u8], member: &[u8]) -> RedisSaddFuture {
        let mut state = self.state.borrow_mut();
        let entry = get_or_insert_entry(&mut state.entries, key, RedisEntryValue::Set(Vec::new()));
        let result = match &mut entry.value {
            RedisEntryValue::Set(values) => {
                if values.iter().any(|value| value.as_slice() == member) {
                    Ok(0)
                } else {
                    values.push(member.to_vec());
                    Ok(1)
                }
            }
            _ => Err(()),
        };
        RedisSaddFuture {
            result: Some(result),
        }
    }

    pub fn smembers(&self, key: &[u8]) -> RedisSmembersFuture {
        let result = self.with_entry(key, |entry| match &entry.value {
            RedisEntryValue::Set(values) => {
                Ok(values.iter().cloned().map(RedisValue::string).collect())
            }
            _ => Err(()),
        });
        RedisSmembersFuture {
            result: Some(result.or_else(|_| Ok(Vec::new()))),
        }
    }

    pub fn hset(&self, key: &[u8], field: &[u8], value: &[u8]) -> RedisHsetFuture {
        let mut state = self.state.borrow_mut();
        let entry = get_or_insert_entry(&mut state.entries, key, RedisEntryValue::Hash(Vec::new()));
        let result = match &mut entry.value {
            RedisEntryValue::Hash(fields) => {
                if let Some((_, stored)) = fields
                    .iter_mut()
                    .find(|(stored_field, _)| stored_field.as_slice() == field)
                {
                    stored.clear();
                    stored.extend_from_slice(value);
                } else {
                    fields.push((field.to_vec(), value.to_vec()));
                }
                Ok(())
            }
            _ => Err(()),
        };
        RedisHsetFuture {
            result: Some(result),
        }
    }

    pub fn hget(&self, key: &[u8], field: &[u8]) -> RedisHgetFuture {
        let result = self.with_entry(key, |entry| match &entry.value {
            RedisEntryValue::Hash(fields) => fields
                .iter()
                .find(|(stored_field, _)| stored_field.as_slice() == field)
                .map(|(_, value)| RedisValue::string(value.clone()))
                .ok_or(()),
            _ => Err(()),
        });
        RedisHgetFuture {
            result: Some(result),
        }
    }

    pub fn hgetall(&self, key: &[u8]) -> RedisHgetallFuture {
        let result = self.with_entry(key, |entry| match &entry.value {
            RedisEntryValue::Hash(fields) => Ok(fields
                .iter()
                .flat_map(|(field, value)| {
                    vec![
                        RedisValue::string(field.clone()),
                        RedisValue::string(value.clone()),
                    ]
                })
                .collect()),
            _ => Err(()),
        });
        RedisHgetallFuture {
            result: Some(result.or_else(|_| Ok(Vec::new()))),
        }
    }

    pub fn ping(&self) -> RedisPingFuture {
        RedisPingFuture {
            result: Some(if self.state.borrow().connected {
                Ok(())
            } else {
                Err(())
            }),
        }
    }

    pub fn quit(&self) -> RedisQuitFuture {
        let mut state = self.state.borrow_mut();
        state.connected = false;
        state.authenticated = false;
        RedisQuitFuture {
            result: Some(Ok(())),
        }
    }

    fn set_string(&self, key: &[u8], value: &[u8], ttl: Option<u64>) -> Result<(), ()> {
        if key.is_empty() {
            return Err(());
        }
        let mut state = self.state.borrow_mut();
        let entry =
            get_or_insert_entry(&mut state.entries, key, RedisEntryValue::String(Vec::new()));
        entry.value = RedisEntryValue::String(value.to_vec());
        entry.ttl = ttl;
        Ok(())
    }

    fn add_integer(&self, key: &[u8], delta: i64) -> Result<i64, ()> {
        let current = self
            .with_entry(key, |entry| match &entry.value {
                RedisEntryValue::String(data) => parse_i64(data).ok_or(()),
                _ => Err(()),
            })
            .unwrap_or(0);
        let next = current + delta;
        self.set_string(key, next.to_string().as_bytes(), None)?;
        Ok(next)
    }

    fn push_list(&self, key: &[u8], value: &[u8], front: bool) -> Result<u64, ()> {
        let mut state = self.state.borrow_mut();
        let entry = get_or_insert_entry(&mut state.entries, key, RedisEntryValue::List(Vec::new()));
        match &mut entry.value {
            RedisEntryValue::List(values) => {
                if front {
                    values.insert(0, value.to_vec());
                } else {
                    values.push(value.to_vec());
                }
                Ok(values.len() as u64)
            }
            _ => Err(()),
        }
    }

    fn with_entry<T>(
        &self,
        key: &[u8],
        f: impl FnOnce(&RedisEntry) -> Result<T, ()>,
    ) -> Result<T, ()> {
        let state = self.state.borrow();
        state
            .entries
            .iter()
            .find(|entry| entry.key.as_slice() == key)
            .ok_or(())
            .and_then(f)
    }

    fn entry_index(&self, key: &[u8]) -> Option<usize> {
        self.state
            .borrow()
            .entries
            .iter()
            .position(|entry| entry.key.as_slice() == key)
    }
}

impl Default for RedisClient {
    fn default() -> Self {
        Self::new()
    }
}

ready_future!(RedisConnectFuture, Result<(), ()>);
ready_future!(RedisAuthFuture, Result<(), ()>);
ready_future!(RedisGetFuture, Result<RedisValue, ()>);
ready_future!(RedisSetFuture, Result<(), ()>);
ready_future!(RedisDelFuture, Result<u64, ()>);
ready_future!(RedisExistsFuture, Result<bool, ()>);
ready_future!(RedisExpireFuture, Result<bool, ()>);
ready_future!(RedisTtlFuture, Result<i64, ()>);
ready_future!(RedisIncrFuture, Result<i64, ()>);
ready_future!(RedisLpushFuture, Result<u64, ()>);
ready_future!(RedisLpopFuture, Result<RedisValue, ()>);
ready_future!(RedisLrangeFuture, Result<Vec<RedisValue>, ()>);
ready_future!(RedisSaddFuture, Result<u64, ()>);
ready_future!(RedisSmembersFuture, Result<Vec<RedisValue>, ()>);
ready_future!(RedisHsetFuture, Result<(), ()>);
ready_future!(RedisHgetFuture, Result<RedisValue, ()>);
ready_future!(RedisHgetallFuture, Result<Vec<RedisValue>, ()>);
ready_future!(RedisPingFuture, Result<(), ()>);
ready_future!(RedisQuitFuture, Result<(), ()>);

fn find_entry_mut<'a>(entries: &'a mut [RedisEntry], key: &[u8]) -> Option<&'a mut RedisEntry> {
    entries.iter_mut().find(|entry| entry.key.as_slice() == key)
}

fn get_or_insert_entry<'a>(
    entries: &'a mut Vec<RedisEntry>,
    key: &[u8],
    value: RedisEntryValue,
) -> &'a mut RedisEntry {
    if let Some(index) = entries.iter().position(|entry| entry.key.as_slice() == key) {
        return &mut entries[index];
    }

    entries.push(RedisEntry {
        key: key.to_vec(),
        value,
        ttl: None,
    });
    entries.last_mut().expect("entry was just inserted")
}

fn normalized_range(len: usize, start: i64, stop: i64) -> (usize, usize) {
    if len == 0 {
        return (0, 0);
    }

    let len_i64 = len as i64;
    let start = if start < 0 { len_i64 + start } else { start }.clamp(0, len_i64);
    let stop = if stop < 0 { len_i64 + stop } else { stop }.clamp(-1, len_i64 - 1);

    if stop < start {
        return (0, 0);
    }

    (start as usize, (stop + 1) as usize)
}

fn parse_i64(bytes: &[u8]) -> Option<i64> {
    let mut value = 0i64;
    let mut negative = false;
    let mut digits = bytes;

    if let Some((&first, rest)) = bytes.split_first() {
        if first == b'-' {
            negative = true;
            digits = rest;
        }
    }

    if digits.is_empty() {
        return None;
    }

    for byte in digits {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value.checked_mul(10)?.checked_add((byte - b'0') as i64)?;
    }

    Some(if negative { -value } else { value })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_on;

    #[test]
    fn redis_string_list_set_and_hash_complete() {
        let client = RedisClient::new();

        block_on(client.connect(b"127.0.0.1", 6379)).unwrap();
        block_on(client.set(b"key", b"41")).unwrap();
        assert_eq!(block_on(client.incr(b"key")).unwrap(), 42);
        assert_eq!(block_on(client.get(b"key")).unwrap().data, b"42");

        block_on(client.lpush(b"list", b"b")).unwrap();
        block_on(client.rpush(b"list", b"c")).unwrap();
        block_on(client.lpush(b"list", b"a")).unwrap();
        let values = block_on(client.lrange(b"list", 0, -1)).unwrap();
        assert_eq!(values[0].data, b"a");
        assert_eq!(values[1].data, b"b");
        assert_eq!(values[2].data, b"c");
        assert_eq!(block_on(client.lpop(b"list")).unwrap().data, b"a");

        assert_eq!(block_on(client.sadd(b"set", b"member")).unwrap(), 1);
        assert_eq!(block_on(client.sadd(b"set", b"member")).unwrap(), 0);
        assert_eq!(block_on(client.smembers(b"set")).unwrap().len(), 1);

        block_on(client.hset(b"hash", b"field", b"value")).unwrap();
        assert_eq!(
            block_on(client.hget(b"hash", b"field")).unwrap().data,
            b"value"
        );
        assert_eq!(block_on(client.hgetall(b"hash")).unwrap().len(), 2);

        assert!(block_on(client.exists(b"key")).unwrap());
        assert_eq!(block_on(client.del(&[b"key"])).unwrap(), 1);
        assert!(!block_on(client.exists(b"key")).unwrap());
    }
}
