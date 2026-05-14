use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.lazada.activities.EnterActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    onactivityresult(state, capabilities);
    onresume(state, capabilities);
    onrequestpermissionsresult(state, capabilities);
    onstart(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.lazada.activities.EnterActivity->onCreate
    // Static call sites: platform=31, internal=45
    // Signal: activity_ui (15 call sites)
    // Signal: package_intents (14 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onactivityresult(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.lazada.activities.EnterActivity->onActivityResult
    // Static call sites: platform=5, internal=8
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onresume(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.lazada.activities.EnterActivity->onResume
    // Static call sites: platform=0, internal=12
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onrequestpermissionsresult(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.lazada.activities.EnterActivity->onRequestPermissionsResult
    // Static call sites: platform=2, internal=5
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onstart(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.lazada.activities.EnterActivity->onStart
    // Static call sites: platform=0, internal=6
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

