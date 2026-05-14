#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComOnesignalNotificationopenedactivityhms,
    ComOnesignalNotificationopenedreceiver,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComOnesignalNotificationopenedactivityhms,
    AppEvent::ComOnesignalNotificationopenedreceiver,
    AppEvent::AndroidAction("android.intent.action.VIEW"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComOnesignalNotificationopenedactivityhms => "com.onesignal.NotificationOpenedActivityHMS",
            AppEvent::ComOnesignalNotificationopenedreceiver => "com.onesignal.NotificationOpenedReceiver",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
