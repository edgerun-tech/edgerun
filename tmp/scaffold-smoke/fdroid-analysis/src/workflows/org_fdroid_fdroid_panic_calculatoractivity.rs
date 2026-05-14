use crate::capabilities::Capabilities;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "org.fdroid.fdroid.panic.CalculatorActivity";
pub const KIND: &str = "activity";

pub fn register(state: &mut AppState, capabilities: &Capabilities) {
    let _ = capabilities;
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    op(state, capabilities);
    eval(state, capabilities);
    number(state, capabilities);
    c(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: org.fdroid.fdroid.panic.CalculatorActivity->onCreate
    // Static call sites: platform=1, internal=7
    // Signal: activity_ui (1 call sites)
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn op(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: org.fdroid.fdroid.panic.CalculatorActivity->op
    // Static call sites: platform=18, internal=5
    // Signal: activity_ui (7 call sites)
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn eval(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: org.fdroid.fdroid.panic.CalculatorActivity->eval
    // Static call sites: platform=15, internal=4
    // Signal: activity_ui (2 call sites)
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn number(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: org.fdroid.fdroid.panic.CalculatorActivity->number
    // Static call sites: platform=7, internal=2
    // Signal: activity_ui (3 call sites)
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn c(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: org.fdroid.fdroid.panic.CalculatorActivity->c
    // Static call sites: platform=6, internal=0
    // Signal: activity_ui (3 call sites)
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

