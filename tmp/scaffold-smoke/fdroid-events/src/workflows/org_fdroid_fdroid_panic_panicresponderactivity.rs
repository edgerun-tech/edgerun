use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "org.fdroid.fdroid.panic.PanicResponderActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    exitandclear(state, capabilities);
    nest_mexitandclear(state, capabilities);
    init(state, capabilities);
    resetrepos(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: org.fdroid.fdroid.panic.PanicResponderActivity->onCreate
    // Static call sites: platform=22, internal=22
    // Signal: package_intents (4 call sites)
    // Signal: activity_ui (3 call sites)
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn exitandclear(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: org.fdroid.fdroid.panic.PanicResponderActivity->exitAndClear
    // Static call sites: platform=1, internal=1
    // Signal: activity_ui (1 call sites)
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn nest_mexitandclear(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: org.fdroid.fdroid.panic.PanicResponderActivity->-$$Nest$mexitAndClear
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn init(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: org.fdroid.fdroid.panic.PanicResponderActivity-><init>
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn resetrepos(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: org.fdroid.fdroid.panic.PanicResponderActivity->resetRepos
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

