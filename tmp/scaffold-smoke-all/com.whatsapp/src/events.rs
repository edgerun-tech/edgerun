#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComWhatsappAccountsyncProfileactivity,
    ComWhatsappAccountsyncCallcontactlandingactivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComWhatsappAccountsyncProfileactivity,
    AppEvent::ComWhatsappAccountsyncCallcontactlandingactivity,
    AppEvent::AndroidAction("android.intent.action.VIEW"),
    AppEvent::AndroidAction("android.intent.action.VIEW"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComWhatsappAccountsyncProfileactivity => "com.whatsapp.accountsync.ProfileActivity",
            AppEvent::ComWhatsappAccountsyncCallcontactlandingactivity => "com.whatsapp.accountsync.CallContactLandingActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
