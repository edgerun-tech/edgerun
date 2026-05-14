use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.google.android.apps.safetycore.service.ClassificationApiService";
pub const KIND: &str = "service";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    b(state, capabilities);
    ondestroy(state, capabilities);
    init(state, capabilities);
    clinit(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.safetycore.service.ClassificationApiService->onCreate
    // Static call sites: platform=3, internal=11
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn b(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.safetycore.service.ClassificationApiService->b
    // Static call sites: platform=1, internal=13
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn ondestroy(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.safetycore.service.ClassificationApiService->onDestroy
    // Static call sites: platform=0, internal=6
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn init(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.safetycore.service.ClassificationApiService-><init>
    // Static call sites: platform=0, internal=3
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn clinit(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.safetycore.service.ClassificationApiService-><clinit>
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

