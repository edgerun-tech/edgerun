#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    NetOpenidAppauthRedirecturireceiveractivity,
    ComSsAndroidUgcAwemeMusicAddtodspAuthRedirecturireceiveractivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::NetOpenidAppauthRedirecturireceiveractivity,
    AppEvent::ComSsAndroidUgcAwemeMusicAddtodspAuthRedirecturireceiveractivity,
    AppEvent::AndroidAction("android.intent.action.VIEW"),
    AppEvent::AndroidAction("android.intent.action.VIEW"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::NetOpenidAppauthRedirecturireceiveractivity => "net.openid.appauth.RedirectUriReceiverActivity",
            AppEvent::ComSsAndroidUgcAwemeMusicAddtodspAuthRedirecturireceiveractivity => "com.ss.android.ugc.aweme.music.addtodsp.auth.RedirectUriReceiverActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
