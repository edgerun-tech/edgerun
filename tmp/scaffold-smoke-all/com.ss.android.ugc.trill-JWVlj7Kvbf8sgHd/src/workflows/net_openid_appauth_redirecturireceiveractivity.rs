use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "net.openid.appauth.RedirectUriReceiverActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    onstop(state, capabilities);
    attachbasecontext(state, capabilities);
    init(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: net.openid.appauth.RedirectUriReceiverActivity->onCreate
    // Static call sites: platform=7, internal=9
    // Signal: package_intents (5 call sites)
    // Signal: activity_ui (3 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onstop(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: net.openid.appauth.RedirectUriReceiverActivity->onStop
    // Static call sites: platform=9, internal=1
    // Signal: activity_ui (9 call sites)
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn attachbasecontext(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: net.openid.appauth.RedirectUriReceiverActivity->attachBaseContext
    // Static call sites: platform=6, internal=3
    // Signal: activity_ui (1 call sites)
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn init(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: net.openid.appauth.RedirectUriReceiverActivity-><init>
    // Static call sites: platform=1, internal=0
    // Signal: activity_ui (1 call sites)
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

