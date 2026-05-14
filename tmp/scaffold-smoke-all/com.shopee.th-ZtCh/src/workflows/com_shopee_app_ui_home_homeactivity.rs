use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.shopee.app.ui.home.HomeActivity_";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    onactivityresult(state, capabilities);
    oncreate(state, capabilities);
    q7(state, capabilities);
    setcontentview(state, capabilities);
    init(state, capabilities);
}

fn onactivityresult(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.shopee.app.ui.home.HomeActivity_->onActivityResult
    // Static call sites: platform=26, internal=3
    // Signal: work_background (20 call sites)
    // Signal: package_intents (6 call sites)
    state.capability_intents.push(capabilities.background_tasks.request("schedule_or_handle_background_work", SOURCE_CLASS));
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.shopee.app.ui.home.HomeActivity_->onCreate
    // Static call sites: platform=0, internal=2
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn q7(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.shopee.app.ui.home.HomeActivity_->q7
    // Static call sites: platform=16, internal=0
    // Signal: work_background (14 call sites)
    // Signal: package_intents (2 call sites)
    // Signal: activity_ui (1 call sites)
    state.capability_intents.push(capabilities.background_tasks.request("schedule_or_handle_background_work", SOURCE_CLASS));
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn setcontentview(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.shopee.app.ui.home.HomeActivity_->setContentView
    // Static call sites: platform=0, internal=6
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn init(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.shopee.app.ui.home.HomeActivity_-><init>
    // Static call sites: platform=0, internal=2
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

