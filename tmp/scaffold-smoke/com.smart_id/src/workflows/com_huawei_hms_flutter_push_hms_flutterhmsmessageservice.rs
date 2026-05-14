use crate::capabilities::Capabilities;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.huawei.hms.flutter.push.hms.FlutterHmsMessageService";
pub const KIND: &str = "service";

pub fn register(state: &mut AppState, capabilities: &Capabilities) {
    let _ = capabilities;
    state.started_workflows.push(SOURCE_CLASS);
    onmessagereceived(state, capabilities);
    oncreate(state, capabilities);
    attachbasecontext(state, capabilities);
    ontokenerror(state, capabilities);
    g(state, capabilities);
}

fn onmessagereceived(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.huawei.hms.flutter.push.hms.FlutterHmsMessageService->onMessageReceived
    // Static call sites: platform=8, internal=17
    // Signal: package_intents (6 call sites)
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.huawei.hms.flutter.push.hms.FlutterHmsMessageService->onCreate
    // Static call sites: platform=1, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn attachbasecontext(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.huawei.hms.flutter.push.hms.FlutterHmsMessageService->attachBaseContext
    // Static call sites: platform=63, internal=13
    // Signal: package_intents (19 call sites)
    // Signal: activity_ui (3 call sites)
    // Signal: work_background (1 call sites)
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn ontokenerror(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.huawei.hms.flutter.push.hms.FlutterHmsMessageService->onTokenError
    // Static call sites: platform=17, internal=33
    // Signal: package_intents (3 call sites)
    // Signal: activity_ui (2 call sites)
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn g(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.huawei.hms.flutter.push.hms.FlutterHmsMessageService->g
    // Static call sites: platform=23, internal=7
    // Signal: activity_ui (4 call sites)
    // Signal: location (1 call sites)
    // Signal: sms_telephony (1 call sites)
    // TODO: request location only when this workflow runs
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

