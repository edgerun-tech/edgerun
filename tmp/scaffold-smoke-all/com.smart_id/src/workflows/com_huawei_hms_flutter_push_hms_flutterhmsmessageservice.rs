use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.huawei.hms.flutter.push.hms.FlutterHmsMessageService";
pub const KIND: &str = "service";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    onmessagereceived(state, capabilities);
    oncreate(state, capabilities);
    ontokenerror(state, capabilities);
    onmessagedelivered(state, capabilities);
    onnewtoken(state, capabilities);
}

fn onmessagereceived(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.huawei.hms.flutter.push.hms.FlutterHmsMessageService->onMessageReceived
    // Static call sites: platform=7, internal=12
    // Signal: package_intents (5 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.huawei.hms.flutter.push.hms.FlutterHmsMessageService->onCreate
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn ontokenerror(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.huawei.hms.flutter.push.hms.FlutterHmsMessageService->onTokenError
    // Static call sites: platform=16, internal=31
    // Signal: package_intents (3 call sites)
    // Signal: activity_ui (2 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onmessagedelivered(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.huawei.hms.flutter.push.hms.FlutterHmsMessageService->onMessageDelivered
    // Static call sites: platform=15, internal=11
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onnewtoken(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.huawei.hms.flutter.push.hms.FlutterHmsMessageService->onNewToken
    // Static call sites: platform=5, internal=17
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

