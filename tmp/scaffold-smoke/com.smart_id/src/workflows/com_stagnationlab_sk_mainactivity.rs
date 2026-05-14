use crate::capabilities::Capabilities;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.stagnationlab.sk.MainActivity";
pub const KIND: &str = "activity";

pub fn register(state: &mut AppState, capabilities: &Capabilities) {
    let _ = capabilities;
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    onresume(state, capabilities);
    onnewintent(state, capabilities);
    onactivityresult(state, capabilities);
    onstart(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.stagnationlab.sk.MainActivity->onCreate
    // Static call sites: platform=1113, internal=354
    // Signal: activity_ui (165 call sites)
    // Signal: package_intents (129 call sites)
    // Signal: work_background (56 call sites)
    // Signal: camera_media_capture (8 call sites)
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: request location only when this workflow runs
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
    // TODO: wrap camera/media access in explicit user sessions
}

fn onresume(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.stagnationlab.sk.MainActivity->onResume
    // Static call sites: platform=75, internal=16
    // Signal: package_intents (20 call sites)
    // Signal: activity_ui (6 call sites)
    // Signal: work_background (2 call sites)
    // Signal: location (1 call sites)
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: request location only when this workflow runs
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onnewintent(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.stagnationlab.sk.MainActivity->onNewIntent
    // Static call sites: platform=9, internal=17
    // Signal: package_intents (4 call sites)
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onactivityresult(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.stagnationlab.sk.MainActivity->onActivityResult
    // Static call sites: platform=0, internal=2
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onstart(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.stagnationlab.sk.MainActivity->onStart
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

