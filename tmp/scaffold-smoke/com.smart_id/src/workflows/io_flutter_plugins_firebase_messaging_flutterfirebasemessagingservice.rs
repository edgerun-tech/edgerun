use crate::capabilities::Capabilities;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "io.flutter.plugins.firebase.messaging.FlutterFirebaseMessagingService";
pub const KIND: &str = "service";

pub fn register(state: &mut AppState, capabilities: &Capabilities) {
    let _ = capabilities;
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    attachbasecontext(state, capabilities);
    i(state, capabilities);
    onnewtoken(state, capabilities);
    i_2(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: io.flutter.plugins.firebase.messaging.FlutterFirebaseMessagingService->onCreate
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn attachbasecontext(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: io.flutter.plugins.firebase.messaging.FlutterFirebaseMessagingService->attachBaseContext
    // Static call sites: platform=108, internal=13
    // Signal: package_intents (35 call sites)
    // Signal: activity_ui (3 call sites)
    // Signal: work_background (3 call sites)
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn i(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: io.flutter.plugins.firebase.messaging.FlutterFirebaseMessagingService->i
    // Static call sites: platform=33, internal=11
    // Signal: activity_ui (10 call sites)
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onnewtoken(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: io.flutter.plugins.firebase.messaging.FlutterFirebaseMessagingService->onNewToken
    // Static call sites: platform=0, internal=2
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn i_2(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: io.flutter.plugins.firebase.messaging.FlutterFirebaseMessagingService->$$i
    // Static call sites: platform=1, internal=0
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

