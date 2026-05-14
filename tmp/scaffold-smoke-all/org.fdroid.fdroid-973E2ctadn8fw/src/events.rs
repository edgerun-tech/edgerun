#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    OrgFdroidFdroidPanicPanicpreferencesactivity,
    OrgFdroidFdroidPanicPanicresponderactivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::OrgFdroidFdroidPanicPanicpreferencesactivity,
    AppEvent::OrgFdroidFdroidPanicPanicresponderactivity,
    AppEvent::AndroidAction("info.guardianproject.panic.action.CONNECT"),
    AppEvent::AndroidAction("info.guardianproject.panic.action.DISCONNECT"),
    AppEvent::AndroidAction("info.guardianproject.panic.action.TRIGGER"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::OrgFdroidFdroidPanicPanicpreferencesactivity => "org.fdroid.fdroid.panic.PanicPreferencesActivity",
            AppEvent::OrgFdroidFdroidPanicPanicresponderactivity => "org.fdroid.fdroid.panic.PanicResponderActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
