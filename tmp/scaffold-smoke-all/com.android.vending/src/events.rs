#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::AndroidAction(action) => action,
        }
    }
}
