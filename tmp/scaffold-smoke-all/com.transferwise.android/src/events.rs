#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComWiseDeeplinkDeeplinkproxyactivity,
    ComWiseNotificationsPresentationPreferencesNotificationpreferencesactivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComWiseDeeplinkDeeplinkproxyactivity,
    AppEvent::ComWiseNotificationsPresentationPreferencesNotificationpreferencesactivity,
    AppEvent::AndroidAction("android.intent.action.VIEW"),
    AppEvent::AndroidAction("android.intent.action.MAIN"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComWiseDeeplinkDeeplinkproxyactivity => "com.wise.deeplink.DeepLinkProxyActivity",
            AppEvent::ComWiseNotificationsPresentationPreferencesNotificationpreferencesactivity => "com.wise.notifications.presentation.preferences.NotificationPreferencesActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
