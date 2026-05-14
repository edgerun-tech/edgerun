use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_whatsapp_accountsync_profileactivity;
pub mod com_whatsapp_accountsync_callcontactlandingactivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComWhatsappAccountsyncProfileactivity => com_whatsapp_accountsync_profileactivity::handle_event(event, state, capabilities),
        AppEvent::ComWhatsappAccountsyncCallcontactlandingactivity => com_whatsapp_accountsync_callcontactlandingactivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "android.intent.action.VIEW" => com_whatsapp_accountsync_profileactivity::handle_event(event, state, capabilities),
            "android.intent.action.VIEW" => com_whatsapp_accountsync_callcontactlandingactivity::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
