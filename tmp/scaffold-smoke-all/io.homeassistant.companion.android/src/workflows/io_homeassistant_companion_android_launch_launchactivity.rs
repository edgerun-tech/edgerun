use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "io.homeassistant.companion.android.launch.LaunchActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    oncreate_lambda_1_0(state, capabilities);
    oncreate_lambda_1(state, capabilities);
    init(state, capabilities);
    viewmodel_delegate_lambda_0_0(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: io.homeassistant.companion.android.launch.LaunchActivity->onCreate
    // Static call sites: platform=0, internal=8
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn oncreate_lambda_1_0(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: io.homeassistant.companion.android.launch.LaunchActivity->onCreate$lambda$1$0
    // Static call sites: platform=0, internal=54
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn oncreate_lambda_1(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: io.homeassistant.companion.android.launch.LaunchActivity->onCreate$lambda$1
    // Static call sites: platform=0, internal=10
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn init(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: io.homeassistant.companion.android.launch.LaunchActivity-><init>
    // Static call sites: platform=0, internal=7
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn viewmodel_delegate_lambda_0_0(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: io.homeassistant.companion.android.launch.LaunchActivity->viewModel_delegate$lambda$0$0
    // Static call sites: platform=0, internal=4
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

