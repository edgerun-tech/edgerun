#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComLazadaActivitiesEnteractivity,
    ComLazadaAndroidVideoproductionBizPlayerVideoplayeractivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComLazadaActivitiesEnteractivity,
    AppEvent::ComLazadaAndroidVideoproductionBizPlayerVideoplayeractivity,
    AppEvent::AndroidAction("android.intent.action.MAIN"),
    AppEvent::AndroidAction("android.intent.action.VIEW"),
    AppEvent::AndroidAction("com.lazada.wireless.action.navigator.INTERNAL_NAVIGATION"),
    AppEvent::AndroidAction("com.lazada.wireless.action.navigator.INTERNAL_NAVIGATION"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComLazadaActivitiesEnteractivity => "com.lazada.activities.EnterActivity",
            AppEvent::ComLazadaAndroidVideoproductionBizPlayerVideoplayeractivity => "com.lazada.android.videoproduction.biz.player.VideoPlayerActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
