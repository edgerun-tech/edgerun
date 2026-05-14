use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_google_android_apps_contacts_activities_peopleactivity;
pub mod com_google_android_apps_contacts_activities_showorcreateactivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComGoogleAndroidAppsContactsActivitiesPeopleactivity => com_google_android_apps_contacts_activities_peopleactivity::handle_event(event, state, capabilities),
        AppEvent::ComGoogleAndroidAppsContactsActivitiesShoworcreateactivity => com_google_android_apps_contacts_activities_showorcreateactivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "com.android.contacts.action.SHOW_OR_CREATE_CONTACT" => com_google_android_apps_contacts_activities_showorcreateactivity::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
