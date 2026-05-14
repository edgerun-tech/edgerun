#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComTwitterAppSettingsSettingsrootcompatactivity,
    ComTwitterAndroidAuthorizeappactivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComTwitterAppSettingsSettingsrootcompatactivity,
    AppEvent::ComTwitterAndroidAuthorizeappactivity,
    AppEvent::AndroidAction("android.intent.action.MAIN"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComTwitterAppSettingsSettingsrootcompatactivity => "com.twitter.app.settings.SettingsRootCompatActivity",
            AppEvent::ComTwitterAndroidAuthorizeappactivity => "com.twitter.android.AuthorizeAppActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
