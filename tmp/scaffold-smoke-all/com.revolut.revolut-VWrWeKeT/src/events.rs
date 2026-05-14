#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComRevolutUiLoginPinLoginactivity,
    ComRevolutFeatureAppLauncherImplUiLauncheractivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComRevolutUiLoginPinLoginactivity,
    AppEvent::ComRevolutFeatureAppLauncherImplUiLauncheractivity,
    AppEvent::AndroidAction("OPEN_MAIN_ACTIVITY"),
    AppEvent::AndroidAction("android.intent.action.MAIN"),
    AppEvent::AndroidAction("android.intent.action.VIEW"),
    AppEvent::AndroidAction("com.revolut.category.CHAT_MESSAGE"),
    AppEvent::AndroidAction("com.revolut.revolut.a2a"),
    AppEvent::AndroidAction("com.revolut.revolut.action.IN_APP_VERIFICATION"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComRevolutUiLoginPinLoginactivity => "com.revolut.ui.login.pin.LoginActivity",
            AppEvent::ComRevolutFeatureAppLauncherImplUiLauncheractivity => "com.revolut.feature.app_launcher.impl.ui.LauncherActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
