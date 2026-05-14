use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.grab.partner.sdk.wrapper.deeplink.DeepLinkActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    onnewintent(state, capabilities);
    getredirecturl(state, capabilities);
    launchchromemanageractivity(state, capabilities);
    init(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.grab.partner.sdk.wrapper.deeplink.DeepLinkActivity->onCreate
    // Static call sites: platform=1, internal=3
    // Signal: activity_ui (1 call sites)
    // Signal: package_intents (1 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onnewintent(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.grab.partner.sdk.wrapper.deeplink.DeepLinkActivity->onNewIntent
    // Static call sites: platform=0, internal=3
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn getredirecturl(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.grab.partner.sdk.wrapper.deeplink.DeepLinkActivity->getRedirectUrl
    // Static call sites: platform=3, internal=3
    // Signal: package_intents (2 call sites)
    // Signal: activity_ui (1 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn launchchromemanageractivity(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.grab.partner.sdk.wrapper.deeplink.DeepLinkActivity->launchChromeManagerActivity
    // Static call sites: platform=0, internal=5
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn init(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.grab.partner.sdk.wrapper.deeplink.DeepLinkActivity-><init>
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

