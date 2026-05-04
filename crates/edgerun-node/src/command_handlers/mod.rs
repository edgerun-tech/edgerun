//! Command handler modules for edgerun-node.
//!
//! Splits the monolithic command_dispatch.rs into focused modules
//! organized by command type.

pub mod control;        // add/remove/transfer controller
pub mod custom;         // delegation/revocation
pub mod apps;          // install/uninstall
pub mod identity;      // create/import identity
pub mod infrastructure; // bootstrap/reachability/query
pub mod user;          // user presence/signature requests
pub mod config;        // update config
