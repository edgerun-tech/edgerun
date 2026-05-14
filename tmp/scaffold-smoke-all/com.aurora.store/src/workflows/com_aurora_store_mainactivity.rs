use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.aurora.store.MainActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    l(state, capabilities);
    init(state, capabilities);
    k(state, capabilities);
    n(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.aurora.store.MainActivity->onCreate
    // Static call sites: platform=6, internal=40
    // Signal: package_intents (2 call sites)
    // Signal: activity_ui (1 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn l(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.aurora.store.MainActivity->L
    // Static call sites: platform=4, internal=9
    // Signal: activity_ui (2 call sites)
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn init(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.aurora.store.MainActivity-><init>
    // Static call sites: platform=3, internal=7
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn k(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.aurora.store.MainActivity->K
    // Static call sites: platform=4, internal=5
    // Signal: activity_ui (2 call sites)
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn n(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.aurora.store.MainActivity->N
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

