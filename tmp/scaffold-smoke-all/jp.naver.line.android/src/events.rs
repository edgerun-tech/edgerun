#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComLinecorpLineSettingsCustomappiconCustomappiconsplashactivityBasic1splashactivity,
    ComLinecorpLineSettingsCustomappiconCustomappiconsplashactivityPromotionhalloween1splashactivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComLinecorpLineSettingsCustomappiconCustomappiconsplashactivityBasic1splashactivity,
    AppEvent::ComLinecorpLineSettingsCustomappiconCustomappiconsplashactivityPromotionhalloween1splashactivity,
    AppEvent::AndroidAction("android.intent.action.MAIN"),
    AppEvent::AndroidAction("android.intent.action.MAIN"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComLinecorpLineSettingsCustomappiconCustomappiconsplashactivityBasic1splashactivity => "com.linecorp.line.settings.customappicon.CustomAppIconSplashActivity$Basic1SplashActivity",
            AppEvent::ComLinecorpLineSettingsCustomappiconCustomappiconsplashactivityPromotionhalloween1splashactivity => "com.linecorp.line.settings.customappicon.CustomAppIconSplashActivity$PromotionHalloween1SplashActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
