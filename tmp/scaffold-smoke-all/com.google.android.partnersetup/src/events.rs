#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComGoogleAndroidPartnersetupBootreceiver,
    ComGoogleAndroidPartnersetupPhonestatereceiver,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComGoogleAndroidPartnersetupBootreceiver,
    AppEvent::ComGoogleAndroidPartnersetupPhonestatereceiver,
    AppEvent::AndroidAction("android.intent.action.BOOT_COMPLETED"),
    AppEvent::AndroidAction("android.intent.action.MY_PACKAGE_REPLACED"),
    AppEvent::AndroidAction("android.intent.action.SIM_STATE_CHANGED"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComGoogleAndroidPartnersetupBootreceiver => "com.google.android.partnersetup.BootReceiver",
            AppEvent::ComGoogleAndroidPartnersetupPhonestatereceiver => "com.google.android.partnersetup.PhoneStateReceiver",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
