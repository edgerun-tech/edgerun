use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;


pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::AndroidAction(action) => match action {
            _ => {}
        },
    }
}
