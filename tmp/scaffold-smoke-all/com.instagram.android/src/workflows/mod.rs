use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_instagram_mainactivity_launcheractivity;
pub mod com_instagram_mainactivity_instagrammainactivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComInstagramMainactivityLauncheractivity => com_instagram_mainactivity_launcheractivity::handle_event(event, state, capabilities),
        AppEvent::ComInstagramMainactivityInstagrammainactivity => com_instagram_mainactivity_instagrammainactivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            _ => {}
        },
    }
}
