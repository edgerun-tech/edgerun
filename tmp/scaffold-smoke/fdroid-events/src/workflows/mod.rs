use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod org_fdroid_fdroid_panic_panicpreferencesactivity;
pub mod org_fdroid_fdroid_panic_panicresponderactivity;
pub mod org_fdroid_fdroid_panic_calculatoractivity;
pub mod org_fdroid_fdroid_views_repos_addrepoactivity;
pub mod org_fdroid_fdroid_views_appdetailsactivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::OrgFdroidFdroidPanicPanicpreferencesactivity => org_fdroid_fdroid_panic_panicpreferencesactivity::handle_event(event, state, capabilities),
        AppEvent::OrgFdroidFdroidPanicPanicresponderactivity => org_fdroid_fdroid_panic_panicresponderactivity::handle_event(event, state, capabilities),
        AppEvent::OrgFdroidFdroidPanicCalculatoractivity => org_fdroid_fdroid_panic_calculatoractivity::handle_event(event, state, capabilities),
        AppEvent::OrgFdroidFdroidViewsReposAddrepoactivity => org_fdroid_fdroid_views_repos_addrepoactivity::handle_event(event, state, capabilities),
        AppEvent::OrgFdroidFdroidViewsAppdetailsactivity => org_fdroid_fdroid_views_appdetailsactivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "info.guardianproject.panic.action.CONNECT" => org_fdroid_fdroid_panic_panicpreferencesactivity::handle_event(event, state, capabilities),
            "info.guardianproject.panic.action.DISCONNECT" => org_fdroid_fdroid_panic_panicpreferencesactivity::handle_event(event, state, capabilities),
            "info.guardianproject.panic.action.TRIGGER" => org_fdroid_fdroid_panic_panicresponderactivity::handle_event(event, state, capabilities),
            "android.intent.action.MAIN" => org_fdroid_fdroid_panic_calculatoractivity::handle_event(event, state, capabilities),
            "android.intent.action.SEND" => org_fdroid_fdroid_views_repos_addrepoactivity::handle_event(event, state, capabilities),
            "android.intent.action.VIEW" => org_fdroid_fdroid_views_repos_addrepoactivity::handle_event(event, state, capabilities),
            "android.intent.action.SHOW_APP_INFO" => org_fdroid_fdroid_views_appdetailsactivity::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
