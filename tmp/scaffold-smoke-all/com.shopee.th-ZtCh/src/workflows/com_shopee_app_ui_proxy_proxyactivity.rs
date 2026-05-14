use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.shopee.app.ui.proxy.ProxyActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    onnewintent(state, capabilities);
    i6(state, capabilities);
    g6(state, capabilities);
    attachbasecontext(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.shopee.app.ui.proxy.ProxyActivity->onCreate
    // Static call sites: platform=56, internal=108
    // Signal: package_intents (9 call sites)
    // Signal: network_web (8 call sites)
    // Signal: activity_ui (4 call sites)
    // Signal: work_background (1 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    state.capability_intents.push(capabilities.network.request("request_network_or_webview", SOURCE_CLASS));
    state.capability_intents.push(capabilities.background_tasks.request("schedule_or_handle_background_work", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: replace direct Android/JVM network access with crate::network allowlist checks
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onnewintent(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.shopee.app.ui.proxy.ProxyActivity->onNewIntent
    // Static call sites: platform=5, internal=11
    // Signal: package_intents (3 call sites)
    // Signal: activity_ui (1 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn i6(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.shopee.app.ui.proxy.ProxyActivity->I6
    // Static call sites: platform=101, internal=124
    // Signal: network_web (21 call sites)
    // Signal: package_intents (12 call sites)
    // Signal: activity_ui (2 call sites)
    state.capability_intents.push(capabilities.network.request("request_network_or_webview", SOURCE_CLASS));
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: replace direct Android/JVM network access with crate::network allowlist checks
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn g6(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.shopee.app.ui.proxy.ProxyActivity->G6
    // Static call sites: platform=119, internal=64
    // Signal: package_intents (45 call sites)
    // Signal: network_web (34 call sites)
    // Signal: activity_ui (12 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    state.capability_intents.push(capabilities.network.request("request_network_or_webview", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: replace direct Android/JVM network access with crate::network allowlist checks
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn attachbasecontext(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.shopee.app.ui.proxy.ProxyActivity->attachBaseContext
    // Static call sites: platform=9, internal=14
    // Signal: work_background (4 call sites)
    // Signal: activity_ui (1 call sites)
    state.capability_intents.push(capabilities.background_tasks.request("schedule_or_handle_background_work", SOURCE_CLASS));
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

