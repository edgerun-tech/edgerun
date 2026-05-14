use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod org_fdroid_fdroid_panic_panicpreferencesactivity;
pub mod org_fdroid_fdroid_panic_panicresponderactivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::OrgFdroidFdroidPanicPanicpreferencesactivity => org_fdroid_fdroid_panic_panicpreferencesactivity::handle_event(event, state, capabilities),
        AppEvent::OrgFdroidFdroidPanicPanicresponderactivity => org_fdroid_fdroid_panic_panicresponderactivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "info.guardianproject.panic.action.CONNECT" => org_fdroid_fdroid_panic_panicpreferencesactivity::handle_event(event, state, capabilities),
            "info.guardianproject.panic.action.DISCONNECT" => org_fdroid_fdroid_panic_panicpreferencesactivity::handle_event(event, state, capabilities),
            "info.guardianproject.panic.action.TRIGGER" => org_fdroid_fdroid_panic_panicresponderactivity::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
