use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod ee_mtakso_client_newbase_ridehailingmapactivity;
pub mod ee_mtakso_client_newbase_voip_voiptrampolineactivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::EeMtaksoClientNewbaseRidehailingmapactivity => ee_mtakso_client_newbase_ridehailingmapactivity::handle_event(event, state, capabilities),
        AppEvent::EeMtaksoClientNewbaseVoipVoiptrampolineactivity => ee_mtakso_client_newbase_voip_voiptrampolineactivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            _ => {}
        },
    }
}
