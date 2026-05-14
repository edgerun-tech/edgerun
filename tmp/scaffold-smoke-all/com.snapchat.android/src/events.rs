#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComBraintreepaymentsApiBraintreebrowserswitchactivity,
    ComRazorpayCheckoutactivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComBraintreepaymentsApiBraintreebrowserswitchactivity,
    AppEvent::ComRazorpayCheckoutactivity,
    AppEvent::AndroidAction("android.intent.action.VIEW"),
    AppEvent::AndroidAction("android.intent.action.MAIN"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComBraintreepaymentsApiBraintreebrowserswitchactivity => "com.braintreepayments.api.BraintreeBrowserSwitchActivity",
            AppEvent::ComRazorpayCheckoutactivity => "com.razorpay.CheckoutActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
