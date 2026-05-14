use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.facebook.orca.threadview.ThreadViewBubblesActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    cih(state, capabilities);
    a31(state, capabilities);
    init(state, capabilities);
    a3g(state, capabilities);
    a2z(state, capabilities);
}

fn cih(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.facebook.orca.threadview.ThreadViewBubblesActivity->Cih
    // Static call sites: platform=1, internal=29
    // Signal: package_intents (1 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn a31(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.facebook.orca.threadview.ThreadViewBubblesActivity->A31
    // Static call sites: platform=0, internal=14
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn init(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.facebook.orca.threadview.ThreadViewBubblesActivity-><init>
    // Static call sites: platform=0, internal=7
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn a3g(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.facebook.orca.threadview.ThreadViewBubblesActivity->A3G
    // Static call sites: platform=1, internal=6
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn a2z(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.facebook.orca.threadview.ThreadViewBubblesActivity->A2z
    // Static call sites: platform=0, internal=3
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

