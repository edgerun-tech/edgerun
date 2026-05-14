use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.google.android.apps.messaging.main.MainActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    onresume(state, capabilities);
    onnewintent(state, capabilities);
    onactivityresult(state, capabilities);
    oncreate(state, capabilities);
    onstart(state, capabilities);
}

fn onresume(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.messaging.main.MainActivity->onResume
    // Static call sites: platform=3, internal=37
    // Signal: package_intents (1 call sites)
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onnewintent(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.messaging.main.MainActivity->onNewIntent
    // Static call sites: platform=10, internal=26
    // Signal: package_intents (6 call sites)
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onactivityresult(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.messaging.main.MainActivity->onActivityResult
    // Static call sites: platform=5, internal=22
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.messaging.main.MainActivity->onCreate
    // Static call sites: platform=5, internal=19
    // Signal: activity_ui (2 call sites)
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onstart(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.messaging.main.MainActivity->onStart
    // Static call sites: platform=1, internal=13
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

