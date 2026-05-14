use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.tcl.bmloginui.ui.appflip.THAppFlipTCLHomeActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    onactivityresult(state, capabilities);
    loaddata(state, capabilities);
    inittips(state, capabilities);
    initflipunlinkgoogleaccount(state, capabilities);
    initreviewgooglepolicy(state, capabilities);
}

fn onactivityresult(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.tcl.bmloginui.ui.appflip.THAppFlipTCLHomeActivity->onActivityResult
    // Static call sites: platform=0, internal=2
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn loaddata(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.tcl.bmloginui.ui.appflip.THAppFlipTCLHomeActivity->loadData
    // Static call sites: platform=23, internal=16
    // Signal: package_intents (8 call sites)
    // Signal: activity_ui (4 call sites)
    // Signal: work_background (3 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    state.capability_intents.push(capabilities.background_tasks.request("schedule_or_handle_background_work", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn inittips(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.tcl.bmloginui.ui.appflip.THAppFlipTCLHomeActivity->initTips
    // Static call sites: platform=18, internal=10
    // Signal: activity_ui (3 call sites)
    // Signal: package_intents (3 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn initflipunlinkgoogleaccount(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.tcl.bmloginui.ui.appflip.THAppFlipTCLHomeActivity->initFlipUnLinkGoogleAccount
    // Static call sites: platform=15, internal=9
    // Signal: activity_ui (2 call sites)
    // Signal: package_intents (2 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn initreviewgooglepolicy(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.tcl.bmloginui.ui.appflip.THAppFlipTCLHomeActivity->initReviewGooglePolicy
    // Static call sites: platform=15, internal=9
    // Signal: activity_ui (2 call sites)
    // Signal: package_intents (2 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

