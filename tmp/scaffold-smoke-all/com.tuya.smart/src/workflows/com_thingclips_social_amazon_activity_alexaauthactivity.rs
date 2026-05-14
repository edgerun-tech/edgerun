use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.thingclips.social.amazon.activity.AlexaAuthActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    onnewintent(state, capabilities);
    wc(state, capabilities);
    xc(state, capabilities);
    yc(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.thingclips.social.amazon.activity.AlexaAuthActivity->onCreate
    // Static call sites: platform=4, internal=4
    // Signal: activity_ui (1 call sites)
    // Signal: package_intents (1 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onnewintent(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.thingclips.social.amazon.activity.AlexaAuthActivity->onNewIntent
    // Static call sites: platform=0, internal=2
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn wc(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.thingclips.social.amazon.activity.AlexaAuthActivity->Wc
    // Static call sites: platform=1, internal=2
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn xc(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.thingclips.social.amazon.activity.AlexaAuthActivity->Xc
    // Static call sites: platform=1, internal=2
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn yc(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.thingclips.social.amazon.activity.AlexaAuthActivity->Yc
    // Static call sites: platform=0, internal=3
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

