#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComInstagramMainactivityLauncheractivity,
    ComInstagramMainactivityInstagrammainactivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComInstagramMainactivityLauncheractivity,
    AppEvent::ComInstagramMainactivityInstagrammainactivity,
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComInstagramMainactivityLauncheractivity => "com.instagram.mainactivity.LauncherActivity",
            AppEvent::ComInstagramMainactivityInstagrammainactivity => "com.instagram.mainactivity.InstagramMainActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
