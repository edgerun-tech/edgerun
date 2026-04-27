//! IPFS client

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
pub struct IpfsMultihash {
    pub algorithm: u8,
    pub digest: Vec<u8>,
}

impl IpfsMultihash {
    pub fn new(algorithm: u8, digest: &[u8]) -> Self {
        Self {
            algorithm,
            digest: digest.to_vec(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IpfsPin {
    pub hash: IpfsMultihash,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct IpfsBlock {
    hash: IpfsMultihash,
    data: Vec<u8>,
}

#[derive(Default)]
struct IpfsState {
    blocks: Vec<IpfsBlock>,
    pins: Vec<IpfsMultihash>,
    names: Vec<(Vec<u8>, IpfsMultihash)>,
}

pub struct IpfsClient {
    state: RefCell<IpfsState>,
}

impl IpfsClient {
    pub fn new() -> Self {
        Self {
            state: RefCell::new(IpfsState::default()),
        }
    }

    pub fn add(&self, data: &[u8]) -> IpfsAddFuture {
        IpfsAddFuture {
            result: Some(self.store_block(data, 0x12)),
        }
    }

    pub fn cat(&self, hash: &IpfsMultihash) -> IpfsCatFuture {
        IpfsCatFuture {
            result: Some(self.get_block(hash)),
        }
    }

    pub fn pin(&self, hash: &IpfsMultihash) -> IpfsPinFuture {
        let result = if self.has_block(hash) {
            let mut state = self.state.borrow_mut();
            if !state.pins.iter().any(|stored| stored == hash) {
                state.pins.push(hash.clone());
            }
            Ok(())
        } else {
            Err(())
        };
        IpfsPinFuture {
            result: Some(result),
        }
    }

    pub fn unpin(&self, hash: &IpfsMultihash) -> IpfsUnpinFuture {
        let mut state = self.state.borrow_mut();
        let before = state.pins.len();
        state.pins.retain(|stored| stored != hash);
        IpfsUnpinFuture {
            result: Some(if before == state.pins.len() {
                Err(())
            } else {
                Ok(())
            }),
        }
    }

    pub fn ls(&self, hash: &IpfsMultihash) -> IpfsLsFuture {
        IpfsLsFuture {
            result: Some(if self.has_block(hash) {
                Ok(vec![hash.clone()])
            } else {
                Err(())
            }),
        }
    }

    pub fn refs(&self, hash: &IpfsMultihash) -> IpfsRefsFuture {
        IpfsRefsFuture {
            result: Some(if self.has_block(hash) {
                Ok(Vec::new())
            } else {
                Err(())
            }),
        }
    }

    pub fn block_get(&self, hash: &IpfsMultihash) -> IpfsBlockGetFuture {
        IpfsBlockGetFuture {
            result: Some(self.get_block(hash)),
        }
    }

    pub fn block_put(&self, data: &[u8]) -> IpfsBlockPutFuture {
        IpfsBlockPutFuture {
            result: Some(self.store_block(data, 0x12)),
        }
    }

    pub fn dag_get(&self, hash: &IpfsMultihash, path: &[u8]) -> IpfsDagGetFuture {
        IpfsDagGetFuture {
            result: Some(if path.is_empty() {
                self.get_block(hash)
            } else {
                Err(())
            }),
        }
    }

    pub fn dag_put(&self, data: &[u8]) -> IpfsDagPutFuture {
        IpfsDagPutFuture {
            result: Some(self.store_block(data, 0x71)),
        }
    }

    pub fn publish(&self, name: &[u8], hash: &IpfsMultihash) -> IpfsPublishFuture {
        let result = if name.is_empty() || !self.has_block(hash) {
            Err(())
        } else {
            let mut state = self.state.borrow_mut();
            if let Some((_, stored)) = state
                .names
                .iter_mut()
                .find(|(stored_name, _)| stored_name.as_slice() == name)
            {
                *stored = hash.clone();
            } else {
                state.names.push((name.to_vec(), hash.clone()));
            }
            Ok(())
        };
        IpfsPublishFuture {
            result: Some(result),
        }
    }

    pub fn resolve(&self, path: &[u8]) -> IpfsResolveFuture {
        let state = self.state.borrow();
        IpfsResolveFuture {
            result: Some(
                state
                    .names
                    .iter()
                    .find(|(name, _)| name.as_slice() == path)
                    .map(|(_, hash)| hash.clone())
                    .ok_or(()),
            ),
        }
    }

    fn store_block(&self, data: &[u8], algorithm: u8) -> Result<IpfsMultihash, ()> {
        if data.is_empty() {
            return Err(());
        }

        let hash = IpfsMultihash::new(algorithm, &digest(data));
        let mut state = self.state.borrow_mut();
        if let Some(block) = state.blocks.iter_mut().find(|block| block.hash == hash) {
            block.data.clear();
            block.data.extend_from_slice(data);
        } else {
            state.blocks.push(IpfsBlock {
                hash: hash.clone(),
                data: data.to_vec(),
            });
        }
        Ok(hash)
    }

    fn get_block(&self, hash: &IpfsMultihash) -> Result<Vec<u8>, ()> {
        self.state
            .borrow()
            .blocks
            .iter()
            .find(|block| block.hash == *hash)
            .map(|block| block.data.clone())
            .ok_or(())
    }

    fn has_block(&self, hash: &IpfsMultihash) -> bool {
        self.state
            .borrow()
            .blocks
            .iter()
            .any(|block| block.hash == *hash)
    }
}

impl Default for IpfsClient {
    fn default() -> Self {
        Self::new()
    }
}

ready_future!(IpfsAddFuture, Result<IpfsMultihash, ()>);
ready_future!(IpfsCatFuture, Result<Vec<u8>, ()>);
ready_future!(IpfsPinFuture, Result<(), ()>);
ready_future!(IpfsUnpinFuture, Result<(), ()>);
ready_future!(IpfsLsFuture, Result<Vec<IpfsMultihash>, ()>);
ready_future!(IpfsRefsFuture, Result<Vec<IpfsMultihash>, ()>);
ready_future!(IpfsBlockGetFuture, Result<Vec<u8>, ()>);
ready_future!(IpfsBlockPutFuture, Result<IpfsMultihash, ()>);
ready_future!(IpfsDagGetFuture, Result<Vec<u8>, ()>);
ready_future!(IpfsDagPutFuture, Result<IpfsMultihash, ()>);
ready_future!(IpfsPublishFuture, Result<(), ()>);
ready_future!(IpfsResolveFuture, Result<IpfsMultihash, ()>);

fn digest(data: &[u8]) -> Vec<u8> {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in data {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash.to_be_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_on;

    #[test]
    fn ipfs_add_cat_pin_and_publish_complete() {
        let client = IpfsClient::new();

        let hash = block_on(client.add(b"payload")).unwrap();
        assert_eq!(block_on(client.cat(&hash)).unwrap(), b"payload");
        assert_eq!(block_on(client.block_get(&hash)).unwrap(), b"payload");
        assert_eq!(block_on(client.ls(&hash)).unwrap(), vec![hash.clone()]);
        assert!(block_on(client.refs(&hash)).unwrap().is_empty());

        block_on(client.pin(&hash)).unwrap();
        block_on(client.unpin(&hash)).unwrap();
        block_on(client.publish(b"name", &hash)).unwrap();
        assert_eq!(block_on(client.resolve(b"name")).unwrap(), hash);

        let dag_hash = block_on(client.dag_put(br#"{"ok":true}"#)).unwrap();
        assert_eq!(
            block_on(client.dag_get(&dag_hash, b"")).unwrap(),
            br#"{"ok":true}"#
        );
    }
}
