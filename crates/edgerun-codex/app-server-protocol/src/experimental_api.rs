/// Marker trait for protocol types that can signal experimental usage.
///
/// The lifted client does not gate experimental fields, so every type reports
/// stable by default.
pub trait ExperimentalApi {
    fn experimental_reason(&self) -> Option<&'static str>;
}

impl<T> ExperimentalApi for T {
    fn experimental_reason(&self) -> Option<&'static str> {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExperimentalField {
    pub type_name: &'static str,
    pub field_name: &'static str,
    pub reason: &'static str,
}

edgerun_inventory::collect!(ExperimentalField);

pub fn experimental_fields() -> Vec<&'static ExperimentalField> {
    Vec::new()
}

pub fn experimental_required_message(reason: &str) -> String {
    format!("{reason} requires experimentalApi capability")
}
