use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_braintreepayments_api_braintreebrowserswitchactivity;
pub mod com_razorpay_checkoutactivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComBraintreepaymentsApiBraintreebrowserswitchactivity => com_braintreepayments_api_braintreebrowserswitchactivity::handle_event(event, state, capabilities),
        AppEvent::ComRazorpayCheckoutactivity => com_razorpay_checkoutactivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "android.intent.action.VIEW" => com_braintreepayments_api_braintreebrowserswitchactivity::handle_event(event, state, capabilities),
            "android.intent.action.MAIN" => com_razorpay_checkoutactivity::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
