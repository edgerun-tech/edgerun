#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComGrabPartnerSdkWrapperDeeplinkDeeplinkactivity,
    ComGrabGkycSdkFeaturesBasicUiBasicactivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComGrabPartnerSdkWrapperDeeplinkDeeplinkactivity,
    AppEvent::ComGrabGkycSdkFeaturesBasicUiBasicactivity,
    AppEvent::AndroidAction("android.intent.action.VIEW"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComGrabPartnerSdkWrapperDeeplinkDeeplinkactivity => "com.grab.partner.sdk.wrapper.deeplink.DeepLinkActivity",
            AppEvent::ComGrabGkycSdkFeaturesBasicUiBasicactivity => "com.grab.gkyc.sdk.features.basic.ui.BasicActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
