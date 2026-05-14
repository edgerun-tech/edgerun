use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.twitter.android.AuthorizeAppActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    onactivityresult(state, capabilities);
    oncreate(state, capabilities);
    clinit(state, capabilities);
    init(state, capabilities);
    contains_005(state, capabilities);
}

fn onactivityresult(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.twitter.android.AuthorizeAppActivity->onActivityResult
    // Static call sites: platform=11, internal=1
    // Signal: package_intents (9 call sites)
    // Signal: activity_ui (2 call sites)
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.twitter.android.AuthorizeAppActivity->onCreate
    // Static call sites: platform=1, internal=0
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn clinit(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.twitter.android.AuthorizeAppActivity-><clinit>
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn init(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.twitter.android.AuthorizeAppActivity-><init>
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn contains_005(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.twitter.android.AuthorizeAppActivity->contains$005
    // Static call sites: platform=1, internal=0
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

