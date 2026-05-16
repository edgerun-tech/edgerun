use crate::prelude::*;

pub(crate) fn format_bytes(bytes: u64, unit_separator: &str) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;

    if bytes >= GIB {
        format!("{:.1}{unit_separator}GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1}{unit_separator}MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1}{unit_separator}KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes}{unit_separator}B")
    }
}
