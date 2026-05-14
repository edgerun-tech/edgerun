use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.google.android.apps.contacts.activities.ShowOrCreateActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    a(state, capabilities);
    onstop(state, capabilities);
    clinit(state, capabilities);
    init(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.contacts.activities.ShowOrCreateActivity->onCreate
    // Static call sites: platform=15, internal=12
    // Signal: network_web (5 call sites)
    // Signal: package_intents (4 call sites)
    // Signal: work_background (4 call sites)
    state.capability_intents.push(capabilities.network.request("request_network_or_webview", SOURCE_CLASS));
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    state.capability_intents.push(capabilities.background_tasks.request("schedule_or_handle_background_work", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: replace direct Android/JVM network access with crate::network allowlist checks
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn a(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.contacts.activities.ShowOrCreateActivity->a
    // Static call sites: platform=19, internal=13
    // Signal: package_intents (8 call sites)
    // Signal: database_preferences (6 call sites)
    // Signal: work_background (3 call sites)
    // Signal: contacts_calendar (1 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    state.capability_intents.push(capabilities.local_state.request("read_or_write_local_state", SOURCE_CLASS));
    state.capability_intents.push(capabilities.background_tasks.request("schedule_or_handle_background_work", SOURCE_CLASS));
    state.capability_intents.push(capabilities.contacts_calendar.request("read_or_write_contact_calendar", SOURCE_CLASS));
    // TODO: move persistence into crate::state or Edgerun structured storage
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onstop(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.contacts.activities.ShowOrCreateActivity->onStop
    // Static call sites: platform=0, internal=2
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn clinit(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.contacts.activities.ShowOrCreateActivity-><clinit>
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn init(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.contacts.activities.ShowOrCreateActivity-><init>
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

