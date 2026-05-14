use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.deliveryhero.config.dashboard.ui.ConfigDashboardActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    m(state, capabilities);
    init(state, capabilities);
    l(state, capabilities);
    n(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.deliveryhero.config.dashboard.ui.ConfigDashboardActivity->onCreate
    // Static call sites: platform=0, internal=5
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn m(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.deliveryhero.config.dashboard.ui.ConfigDashboardActivity->M
    // Static call sites: platform=0, internal=18
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn init(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.deliveryhero.config.dashboard.ui.ConfigDashboardActivity-><init>
    // Static call sites: platform=0, internal=6
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn l(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.deliveryhero.config.dashboard.ui.ConfigDashboardActivity->L
    // Static call sites: platform=0, internal=2
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn n(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.deliveryhero.config.dashboard.ui.ConfigDashboardActivity->N
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

