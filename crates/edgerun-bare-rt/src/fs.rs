//! Filesystem stub.

#![no_std]

pub struct File;
impl File { pub fn new() -> Self { Self } }

pub struct DirEntry;
impl DirEntry { pub fn name(&self) -> &str { "" } pub fn is_dir(&self) -> bool { false } }

pub struct Dir;
impl Dir { pub fn new() -> Self { Self } }

#[derive(Debug)]
pub struct Error;
impl Error { pub fn not_found() -> Self { Self } }