use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.whatsapp.accountsync.ProfileActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    onactivityresult(state, capabilities);
    a0w(state, capabilities);
    init(state, capabilities);
    a5c(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.whatsapp.accountsync.ProfileActivity->onCreate
    // Static call sites: platform=2, internal=20
    // Signal: activity_ui (2 call sites)
    // Signal: package_intents (1 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onactivityresult(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.whatsapp.accountsync.ProfileActivity->onActivityResult
    // Static call sites: platform=1, internal=5
    // Signal: activity_ui (1 call sites)
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn a0w(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.whatsapp.accountsync.ProfileActivity->A0W
    // Static call sites: platform=5, internal=19
    // Signal: activity_ui (3 call sites)
    // Signal: package_intents (3 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn init(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.whatsapp.accountsync.ProfileActivity-><init>
    // Static call sites: platform=0, internal=13
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn a5c(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.whatsapp.accountsync.ProfileActivity->A5C
    // Static call sites: platform=0, internal=7
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

