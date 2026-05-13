extern crate alloc;

use alloc::format;

use super::{HeaderField, dynamic::DynamicTable};

pub const TABLE_SIZE: usize = 4096;

pub fn build_table() -> DynamicTable {
    let mut table = DynamicTable::new();
    table.set_max_size(TABLE_SIZE).unwrap();
    table.set_max_blocked(100).unwrap();
    table
}

pub fn build_table_with_size(n_field: usize) -> DynamicTable {
    let mut table = DynamicTable::new();
    table.set_max_size(TABLE_SIZE).unwrap();
    table.set_max_blocked(100).unwrap();

    for i in 0..n_field {
        table
            .put(HeaderField::new(format!("foo{}", i + 1), "bar"))
            .unwrap();
    }

    table
}
