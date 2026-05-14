#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComFacebookOrcaThreadviewThreadviewbubblesactivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComFacebookOrcaThreadviewThreadviewbubblesactivity,
    AppEvent::AndroidAction("com.facebook.orca.THREAD_VIEW_BUBBLE"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComFacebookOrcaThreadviewThreadviewbubblesactivity => "com.facebook.orca.threadview.ThreadViewBubblesActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
