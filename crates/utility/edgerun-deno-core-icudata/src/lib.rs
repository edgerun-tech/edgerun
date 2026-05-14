#[repr(C, align(16))]
struct IcuData<T: ?Sized>(T);

static ICU_DATA_RAW: &IcuData<[u8]> = &IcuData(*include_bytes!("icudtl.dat"));

/// Raw ICU data for V8-compatible code paths.
pub static ICU_DATA: &[u8] = &ICU_DATA_RAW.0;
