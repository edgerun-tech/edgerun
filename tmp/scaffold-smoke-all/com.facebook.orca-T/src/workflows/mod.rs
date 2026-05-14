use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_facebook_orca_threadview_threadviewbubblesactivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComFacebookOrcaThreadviewThreadviewbubblesactivity => com_facebook_orca_threadview_threadviewbubblesactivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "com.facebook.orca.THREAD_VIEW_BUBBLE" => com_facebook_orca_threadview_threadviewbubblesactivity::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
