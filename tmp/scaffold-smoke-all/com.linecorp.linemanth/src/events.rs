#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComLinemanMartFeatureTelemedFeatureVideocallTelemedvideocallactivity,
    ComLinecorpLinemanthAndroidFeatureVoipPresentationVoipactivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComLinemanMartFeatureTelemedFeatureVideocallTelemedvideocallactivity,
    AppEvent::ComLinecorpLinemanthAndroidFeatureVoipPresentationVoipactivity,
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComLinemanMartFeatureTelemedFeatureVideocallTelemedvideocallactivity => "com.lineman.mart.feature.telemed.feature.videocall.TelemedVideoCallActivity",
            AppEvent::ComLinecorpLinemanthAndroidFeatureVoipPresentationVoipactivity => "com.linecorp.linemanth.android.feature.voip.presentation.VoipActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
