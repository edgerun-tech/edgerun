use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.grab.gkyc.sdk.features.basic.ui.BasicActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    g2(state, capabilities);
    l2(state, capabilities);
    init(state, capabilities);
    i2(state, capabilities);
    n2(state, capabilities);
}

fn g2(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.grab.gkyc.sdk.features.basic.ui.BasicActivity->G2
    // Static call sites: platform=1, internal=40
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn l2(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.grab.gkyc.sdk.features.basic.ui.BasicActivity->L2
    // Static call sites: platform=0, internal=8
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn init(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.grab.gkyc.sdk.features.basic.ui.BasicActivity-><init>
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn i2(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.grab.gkyc.sdk.features.basic.ui.BasicActivity->I2
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn n2(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.grab.gkyc.sdk.features.basic.ui.BasicActivity->N2
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

