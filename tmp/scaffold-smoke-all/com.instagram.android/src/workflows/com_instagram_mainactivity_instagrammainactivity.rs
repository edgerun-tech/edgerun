use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.instagram.mainactivity.InstagramMainActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    onnewintent(state, capabilities);
    a0h(state, capabilities);
    a0i(state, capabilities);
    a1p(state, capabilities);
    a21(state, capabilities);
}

fn onnewintent(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.instagram.mainactivity.InstagramMainActivity->onNewIntent
    // Static call sites: platform=15, internal=44
    // Signal: package_intents (7 call sites)
    // Signal: network_web (3 call sites)
    // Signal: activity_ui (2 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    state.capability_intents.push(capabilities.network.request("request_network_or_webview", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: replace direct Android/JVM network access with crate::network allowlist checks
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn a0h(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.instagram.mainactivity.InstagramMainActivity->A0h
    // Static call sites: platform=75, internal=343
    // Signal: package_intents (36 call sites)
    // Signal: activity_ui (14 call sites)
    // Signal: work_background (2 call sites)
    // Signal: network_web (1 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    state.capability_intents.push(capabilities.background_tasks.request("schedule_or_handle_background_work", SOURCE_CLASS));
    state.capability_intents.push(capabilities.network.request("request_network_or_webview", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: replace direct Android/JVM network access with crate::network allowlist checks
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn a0i(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.instagram.mainactivity.InstagramMainActivity->A0i
    // Static call sites: platform=71, internal=327
    // Signal: package_intents (39 call sites)
    // Signal: activity_ui (15 call sites)
    // Signal: work_background (2 call sites)
    // Signal: network_web (1 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    state.capability_intents.push(capabilities.background_tasks.request("schedule_or_handle_background_work", SOURCE_CLASS));
    state.capability_intents.push(capabilities.network.request("request_network_or_webview", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: replace direct Android/JVM network access with crate::network allowlist checks
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn a1p(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.instagram.mainactivity.InstagramMainActivity->A1p
    // Static call sites: platform=68, internal=151
    // Signal: package_intents (47 call sites)
    // Signal: activity_ui (3 call sites)
    // Signal: work_background (3 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    state.capability_intents.push(capabilities.background_tasks.request("schedule_or_handle_background_work", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn a21(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.instagram.mainactivity.InstagramMainActivity->A21
    // Static call sites: platform=20, internal=156
    // Signal: package_intents (4 call sites)
    // Signal: activity_ui (3 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

