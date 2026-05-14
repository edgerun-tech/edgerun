#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComGoogleAndroidAppsContactsActivitiesPeopleactivity,
    ComGoogleAndroidAppsContactsActivitiesShoworcreateactivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComGoogleAndroidAppsContactsActivitiesPeopleactivity,
    AppEvent::ComGoogleAndroidAppsContactsActivitiesShoworcreateactivity,
    AppEvent::AndroidAction("com.android.contacts.action.SHOW_OR_CREATE_CONTACT"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComGoogleAndroidAppsContactsActivitiesPeopleactivity => "com.google.android.apps.contacts.activities.PeopleActivity",
            AppEvent::ComGoogleAndroidAppsContactsActivitiesShoworcreateactivity => "com.google.android.apps.contacts.activities.ShowOrCreateActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
