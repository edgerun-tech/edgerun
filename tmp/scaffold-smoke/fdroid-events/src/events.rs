#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    OrgFdroidFdroidPanicPanicpreferencesactivity,
    OrgFdroidFdroidPanicPanicresponderactivity,
    OrgFdroidFdroidPanicCalculatoractivity,
    OrgFdroidFdroidViewsReposAddrepoactivity,
    OrgFdroidFdroidViewsAppdetailsactivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::OrgFdroidFdroidPanicPanicpreferencesactivity,
    AppEvent::OrgFdroidFdroidPanicPanicresponderactivity,
    AppEvent::OrgFdroidFdroidPanicCalculatoractivity,
    AppEvent::OrgFdroidFdroidViewsReposAddrepoactivity,
    AppEvent::OrgFdroidFdroidViewsAppdetailsactivity,
    AppEvent::AndroidAction("info.guardianproject.panic.action.CONNECT"),
    AppEvent::AndroidAction("info.guardianproject.panic.action.DISCONNECT"),
    AppEvent::AndroidAction("info.guardianproject.panic.action.TRIGGER"),
    AppEvent::AndroidAction("android.intent.action.MAIN"),
    AppEvent::AndroidAction("android.intent.action.SEND"),
    AppEvent::AndroidAction("android.intent.action.VIEW"),
    AppEvent::AndroidAction("android.intent.action.SHOW_APP_INFO"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::OrgFdroidFdroidPanicPanicpreferencesactivity => "org.fdroid.fdroid.panic.PanicPreferencesActivity",
            AppEvent::OrgFdroidFdroidPanicPanicresponderactivity => "org.fdroid.fdroid.panic.PanicResponderActivity",
            AppEvent::OrgFdroidFdroidPanicCalculatoractivity => "org.fdroid.fdroid.panic.CalculatorActivity",
            AppEvent::OrgFdroidFdroidViewsReposAddrepoactivity => "org.fdroid.fdroid.views.repos.AddRepoActivity",
            AppEvent::OrgFdroidFdroidViewsAppdetailsactivity => "org.fdroid.fdroid.views.AppDetailsActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
