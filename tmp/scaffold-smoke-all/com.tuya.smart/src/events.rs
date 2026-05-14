#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComThingclipsSocialAmazonActivityTriplealexaaccountlinkactivity,
    ComThingclipsSocialAmazonActivityAlexaauthactivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComThingclipsSocialAmazonActivityTriplealexaaccountlinkactivity,
    AppEvent::ComThingclipsSocialAmazonActivityAlexaauthactivity,
    AppEvent::AndroidAction("android.intent.action.VIEW"),
    AppEvent::AndroidAction("android.intent.action.VIEW"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComThingclipsSocialAmazonActivityTriplealexaaccountlinkactivity => "com.thingclips.social.amazon.activity.TripleAlexaAccountLinkActivity",
            AppEvent::ComThingclipsSocialAmazonActivityAlexaauthactivity => "com.thingclips.social.amazon.activity.AlexaAuthActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
