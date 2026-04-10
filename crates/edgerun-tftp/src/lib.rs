//! Dependency-free TFTP server (RFC 1350) with RFC 2347/2348 option negotiation.
//!
//! # Architecture
//! - **TFTP protocol** — RRQ/WRQ/DATA/ACK/ERROR/OACK over UDP
//! - **RFC 2347 options** — blksize, tsize, timeout negotiation
//! - **FileProvider trait** — pluggable backend (filesystem, encrypted blobs, memory)
//! - **BlobProvider** — serves files from `edgerun-storage` encrypted blob store
//!
//! # PXE Boot Flow
//! ```text
//! Client                          Server (edgerund)
//!   |                                 |
//!   |--- DHCP DISCOVER (with arch) -->|
//!   |<-- DHCP OFFER (IP + bootfile) -|
//!   |--- DHCP REQUEST -------------->|
//!   |<-- DHCP ACK -------------------|
//!   |                                 |
//!   |--- TFTP RRQ "bootx64.efi" ---->|  (UDP 69)
//!   |<-- DATA block 1 ---------------|  (random port)
//!   |-- ACK block 1 ---------------->|
//!   |<-- DATA block 2 ---------------|
//!   |-- ACK block 2 ---------------->|
//!   |        ...                      |
//!   |<-- DATA block N (< 512B) ------|
//!   |-- ACK block N ---------------->|
//!   |                                 |
//!   |  (BIOS/UEFI loads and executes)|
//! ```

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod message;
pub mod server;
pub mod blob_provider;

pub use message::{TftpOpcode, TftpMessage, TftpError, TftpOptions};
pub use server::TftpServer;
pub use blob_provider::BlobTftpProvider;
