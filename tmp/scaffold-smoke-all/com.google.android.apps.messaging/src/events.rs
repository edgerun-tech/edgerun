#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComGoogleAndroidAppsMessagingUiConversationLaunchconversationactivity,
    ComGoogleAndroidAppsMessagingMainMainactivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComGoogleAndroidAppsMessagingUiConversationLaunchconversationactivity,
    AppEvent::ComGoogleAndroidAppsMessagingMainMainactivity,
    AppEvent::AndroidAction("android.intent.action.SENDTO"),
    AppEvent::AndroidAction("android.intent.action.VIEW"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComGoogleAndroidAppsMessagingUiConversationLaunchconversationactivity => "com.google.android.apps.messaging.ui.conversation.LaunchConversationActivity",
            AppEvent::ComGoogleAndroidAppsMessagingMainMainactivity => "com.google.android.apps.messaging.main.MainActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
