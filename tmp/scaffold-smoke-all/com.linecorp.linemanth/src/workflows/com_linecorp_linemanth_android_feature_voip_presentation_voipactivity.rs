use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.linecorp.linemanth.android.feature.voip.presentation.VoipActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    onnewintent(state, capabilities);
    onstart(state, capabilities);
    oncreate(state, capabilities);
    h(state, capabilities);
    onstop(state, capabilities);
}

fn onnewintent(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.linecorp.linemanth.android.feature.voip.presentation.VoipActivity->onNewIntent
    // Static call sites: platform=0, internal=5
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onstart(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.linecorp.linemanth.android.feature.voip.presentation.VoipActivity->onStart
    // Static call sites: platform=4, internal=1
    // Signal: activity_ui (4 call sites)
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.linecorp.linemanth.android.feature.voip.presentation.VoipActivity->onCreate
    // Static call sites: platform=1, internal=3
    // Signal: activity_ui (1 call sites)
    // Signal: package_intents (1 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn h(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.linecorp.linemanth.android.feature.voip.presentation.VoipActivity->H
    // Static call sites: platform=2, internal=15
    // Signal: package_intents (2 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onstop(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.linecorp.linemanth.android.feature.voip.presentation.VoipActivity->onStop
    // Static call sites: platform=4, internal=1
    // Signal: activity_ui (4 call sites)
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

