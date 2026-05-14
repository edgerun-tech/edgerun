#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComStagnationlabSkMainactivity,
    ComHuaweiHmsFlutterPushHmsFlutterhmsmessageservice,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComStagnationlabSkMainactivity,
    AppEvent::ComHuaweiHmsFlutterPushHmsFlutterhmsmessageservice,
    AppEvent::AndroidAction("android.intent.action.MAIN"),
    AppEvent::AndroidAction("android.intent.action.VIEW"),
    AppEvent::AndroidAction("com.huawei.push.action.MESSAGING_EVENT"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComStagnationlabSkMainactivity => "com.stagnationlab.sk.MainActivity",
            AppEvent::ComHuaweiHmsFlutterPushHmsFlutterhmsmessageservice => "com.huawei.hms.flutter.push.hms.FlutterHmsMessageService",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
