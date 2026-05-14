use crate::capabilities::Capabilities;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.thingclips.smart.speech.activity.AssisantMainActivity";
pub const KIND: &str = "activity";

pub fn register(state: &mut AppState, capabilities: &Capabilities) {
    let _ = capabilities;
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    onnewintent(state, capabilities);
    onrequestpermissionsresult(state, capabilities);
    initview(state, capabilities);
    vd(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.thingclips.smart.speech.activity.AssisantMainActivity->onCreate
    // Static call sites: platform=3, internal=7
    // Signal: activity_ui (3 call sites)
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onnewintent(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.thingclips.smart.speech.activity.AssisantMainActivity->onNewIntent
    // Static call sites: platform=0, internal=2
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onrequestpermissionsresult(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.thingclips.smart.speech.activity.AssisantMainActivity->onRequestPermissionsResult
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn initview(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.thingclips.smart.speech.activity.AssisantMainActivity->initView
    // Static call sites: platform=9, internal=25
    // Signal: activity_ui (8 call sites)
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn vd(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.thingclips.smart.speech.activity.AssisantMainActivity->vd
    // Static call sites: platform=14, internal=11
    // Signal: activity_ui (7 call sites)
    // Signal: package_intents (7 call sites)
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

