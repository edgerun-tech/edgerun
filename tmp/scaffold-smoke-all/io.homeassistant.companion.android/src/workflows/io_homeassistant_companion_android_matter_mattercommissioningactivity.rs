use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "io.homeassistant.companion.android.matter.MatterCommissioningActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    onresume(state, capabilities);
    oncreate(state, capabilities);
    onnewintent(state, capabilities);
    oncreate_lambda_0_0(state, capabilities);
    init(state, capabilities);
}

fn onresume(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: io.homeassistant.companion.android.matter.MatterCommissioningActivity->onResume
    // Static call sites: platform=12, internal=21
    // Signal: package_intents (1 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: io.homeassistant.companion.android.matter.MatterCommissioningActivity->onCreate
    // Static call sites: platform=0, internal=5
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onnewintent(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: io.homeassistant.companion.android.matter.MatterCommissioningActivity->onNewIntent
    // Static call sites: platform=0, internal=3
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn oncreate_lambda_0_0(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: io.homeassistant.companion.android.matter.MatterCommissioningActivity->onCreate$lambda$0$0
    // Static call sites: platform=0, internal=42
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn init(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: io.homeassistant.companion.android.matter.MatterCommissioningActivity-><init>
    // Static call sites: platform=0, internal=10
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

