use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "ee.mtakso.client.newbase.voip.VoipTrampolineActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    o(state, capabilities);
    init(state, capabilities);
    clinit(state, capabilities);
    n(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: ee.mtakso.client.newbase.voip.VoipTrampolineActivity->onCreate
    // Static call sites: platform=15, internal=14
    // Signal: package_intents (8 call sites)
    // Signal: activity_ui (6 call sites)
    // Signal: work_background (4 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    state.capability_intents.push(capabilities.background_tasks.request("schedule_or_handle_background_work", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn o(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: ee.mtakso.client.newbase.voip.VoipTrampolineActivity->o
    // Static call sites: platform=0, internal=6
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn init(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: ee.mtakso.client.newbase.voip.VoipTrampolineActivity-><init>
    // Static call sites: platform=0, internal=3
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn clinit(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: ee.mtakso.client.newbase.voip.VoipTrampolineActivity-><clinit>
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn n(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: ee.mtakso.client.newbase.voip.VoipTrampolineActivity->n
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

