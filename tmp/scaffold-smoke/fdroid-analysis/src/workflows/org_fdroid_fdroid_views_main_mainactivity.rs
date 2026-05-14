use crate::capabilities::Capabilities;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "org.fdroid.fdroid.views.main.MainActivity";
pub const KIND: &str = "activity";

pub fn register(state: &mut AppState, capabilities: &Capabilities) {
    let _ = capabilities;
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    onrequestpermissionsresult(state, capabilities);
    onresume(state, capabilities);
    onnewintent(state, capabilities);
    onstart(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: org.fdroid.fdroid.views.main.MainActivity->onCreate
    // Static call sites: platform=7, internal=28
    // Signal: activity_ui (6 call sites)
    // Signal: package_intents (2 call sites)
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onrequestpermissionsresult(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: org.fdroid.fdroid.views.main.MainActivity->onRequestPermissionsResult
    // Static call sites: platform=4, internal=4
    // Signal: activity_ui (2 call sites)
    // Signal: package_intents (2 call sites)
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onresume(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: org.fdroid.fdroid.views.main.MainActivity->onResume
    // Static call sites: platform=0, internal=4
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onnewintent(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: org.fdroid.fdroid.views.main.MainActivity->onNewIntent
    // Static call sites: platform=0, internal=3
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onstart(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: org.fdroid.fdroid.views.main.MainActivity->onStart
    // Static call sites: platform=0, internal=3
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

