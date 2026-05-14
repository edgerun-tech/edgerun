use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "ee.mtakso.client.newbase.RideHailingMapActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    onactivityresult(state, capabilities);
    onnewintent(state, capabilities);
    onstart(state, capabilities);
    createrouter(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: ee.mtakso.client.newbase.RideHailingMapActivity->onCreate
    // Static call sites: platform=6, internal=30
    // Signal: work_background (3 call sites)
    // Signal: package_intents (2 call sites)
    // Signal: activity_ui (1 call sites)
    state.capability_intents.push(capabilities.background_tasks.request("schedule_or_handle_background_work", SOURCE_CLASS));
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onactivityresult(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: ee.mtakso.client.newbase.RideHailingMapActivity->onActivityResult
    // Static call sites: platform=0, internal=21
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onnewintent(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: ee.mtakso.client.newbase.RideHailingMapActivity->onNewIntent
    // Static call sites: platform=2, internal=5
    // Signal: package_intents (2 call sites)
    // Signal: activity_ui (1 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onstart(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: ee.mtakso.client.newbase.RideHailingMapActivity->onStart
    // Static call sites: platform=0, internal=3
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn createrouter(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: ee.mtakso.client.newbase.RideHailingMapActivity->createRouter
    // Static call sites: platform=0, internal=11
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

