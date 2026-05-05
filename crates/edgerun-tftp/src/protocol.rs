//! TFTP read-transfer protocol state without UDP socket ownership.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use crate::message::{DEFAULT_BLKSIZE, DEFAULT_TIMEOUT, TftpError, TftpMessage, TftpOptions};

pub trait TftpReadProvider {
    fn file_size(&self, filename: &str) -> Option<u64>;
    fn read_block(&self, filename: &str, offset: usize, max_size: usize) -> Option<Vec<u8>>;
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TftpPeerId(pub Vec<u8>);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TftpDatagram {
    pub peer: TftpPeerId,
    pub wire: Vec<u8>,
}

#[derive(Clone, Debug)]
struct TftpTransfer {
    filename: String,
    blksize: u16,
    total_size: u64,
    current_block: u16,
    offset: usize,
}

pub struct TftpReadCore<P> {
    provider: P,
    transfers: BTreeMap<(TftpPeerId, String), TftpTransfer>,
}

impl<P: TftpReadProvider> TftpReadCore<P> {
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            transfers: BTreeMap::new(),
        }
    }

    pub fn handle_wire(
        &mut self,
        peer: TftpPeerId,
        wire: &[u8],
    ) -> Result<Vec<TftpDatagram>, crate::message::io::Error> {
        let message = TftpMessage::from_wire(wire)?;
        Ok(self.handle_message(peer, message))
    }

    pub fn handle_message(&mut self, peer: TftpPeerId, message: TftpMessage) -> Vec<TftpDatagram> {
        match message {
            TftpMessage::RRQ {
                filename,
                mode,
                options,
                ..
            } => self.handle_rrq(peer, filename, mode, options),
            TftpMessage::WRQ { filename, .. } => vec![self.error(
                peer,
                TftpError::AccessViolation,
                &format!("Write access denied: {filename}"),
            )],
            TftpMessage::ACK { block } => self.handle_ack(peer, block),
            TftpMessage::DATA { .. } | TftpMessage::OACK { .. } => {
                vec![self.error(peer, TftpError::IllegalOperation, "Unexpected message")]
            }
            TftpMessage::ERROR { .. } => {
                self.transfers
                    .retain(|(stored_peer, _), _| *stored_peer != peer);
                Vec::new()
            }
        }
    }

    pub fn active_transfers(&self) -> usize {
        self.transfers.len()
    }

    fn handle_rrq(
        &mut self,
        peer: TftpPeerId,
        filename: String,
        mode: String,
        client_options: TftpOptions,
    ) -> Vec<TftpDatagram> {
        if mode.to_lowercase() != "octet" {
            return vec![self.error(
                peer,
                TftpError::IllegalOperation,
                "Only octet mode is supported",
            )];
        }

        let Some(total_size) = self.provider.file_size(&filename) else {
            return vec![self.error(
                peer,
                TftpError::FileNotFound,
                &format!("File not found: {filename}"),
            )];
        };

        let negotiated = TftpOptions {
            blksize: client_options.blksize,
            tsize: Some(total_size),
            timeout: client_options.timeout,
        };
        self.transfers.insert(
            (peer.clone(), filename.clone()),
            TftpTransfer {
                filename,
                blksize: negotiated.blksize,
                total_size,
                current_block: 0,
                offset: 0,
            },
        );

        if client_options.blksize != DEFAULT_BLKSIZE
            || client_options.tsize == Some(0)
            || client_options.timeout != DEFAULT_TIMEOUT
        {
            vec![TftpDatagram {
                peer,
                wire: (TftpMessage::OACK {
                    options: negotiated,
                })
                .to_wire(),
            }]
        } else {
            self.next_block(peer)
        }
    }

    fn handle_ack(&mut self, peer: TftpPeerId, block: u16) -> Vec<TftpDatagram> {
        let transfer = self
            .transfers
            .iter()
            .find(|((stored_peer, _), _)| *stored_peer == peer)
            .map(|(key, transfer)| (key.clone(), transfer.clone()));

        let Some((key, transfer)) = transfer else {
            return Vec::new();
        };

        if block != transfer.current_block && !(block == 0 && transfer.current_block == 0) {
            return Vec::new();
        }
        if transfer.offset >= transfer.total_size as usize {
            self.transfers.remove(&key);
            Vec::new()
        } else {
            self.next_block(peer)
        }
    }

    fn next_block(&mut self, peer: TftpPeerId) -> Vec<TftpDatagram> {
        let transfer = self
            .transfers
            .iter()
            .find(|((stored_peer, _), _)| *stored_peer == peer)
            .map(|(key, transfer)| (key.clone(), transfer.clone()));

        let Some((key, transfer)) = transfer else {
            return Vec::new();
        };

        let Some(data) = self.provider.read_block(
            &transfer.filename,
            transfer.offset,
            transfer.blksize as usize,
        ) else {
            self.transfers.remove(&key);
            return vec![self.error(peer, TftpError::NotDefined, "Failed to read file block")];
        };

        let data_len = data.len();
        let new_block = transfer.current_block.wrapping_add(1);
        if let Some(stored) = self.transfers.get_mut(&key) {
            stored.current_block = new_block;
            stored.offset += data_len;
        }

        vec![TftpDatagram {
            peer,
            wire: TftpMessage::data(new_block, data).to_wire(),
        }]
    }

    fn error(&self, peer: TftpPeerId, code: TftpError, message: &str) -> TftpDatagram {
        TftpDatagram {
            peer,
            wire: TftpMessage::error(code, message).to_wire(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    struct BytesProvider {
        data: Vec<u8>,
    }

    impl TftpReadProvider for BytesProvider {
        fn file_size(&self, filename: &str) -> Option<u64> {
            (filename == "boot.ipxe").then_some(self.data.len() as u64)
        }

        fn read_block(&self, filename: &str, offset: usize, max_size: usize) -> Option<Vec<u8>> {
            if filename != "boot.ipxe" || offset > self.data.len() {
                return None;
            }
            let end = offset.saturating_add(max_size).min(self.data.len());
            Some(self.data[offset..end].to_vec())
        }
    }

    #[test]
    fn rrq_produces_data_without_socket() {
        let mut core = TftpReadCore::new(BytesProvider {
            data: b"hello".to_vec(),
        });
        let peer = TftpPeerId(vec![1, 2, 3, 4]);

        let replies = core.handle_message(peer.clone(), TftpMessage::rrq("boot.ipxe"));
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0].peer, peer);

        let message = TftpMessage::from_wire(&replies[0].wire).unwrap();
        assert!(matches!(message, TftpMessage::DATA { block: 1, .. }));
        assert_eq!(core.active_transfers(), 1);
    }

    #[test]
    fn missing_file_produces_error_without_socket() {
        let mut core = TftpReadCore::new(BytesProvider { data: Vec::new() });
        let replies = core.handle_message(TftpPeerId(vec![9]), TftpMessage::rrq("missing"));

        assert_eq!(replies.len(), 1);
        let message = TftpMessage::from_wire(&replies[0].wire).unwrap();
        assert!(matches!(
            message,
            TftpMessage::ERROR {
                code: TftpError::FileNotFound,
                ..
            }
        ));
    }
}
