#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComDiscordShareShareactivity,
    ComDiscordMainMainactivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComDiscordShareShareactivity,
    AppEvent::ComDiscordMainMainactivity,
    AppEvent::AndroidAction("android.intent.action.SEND"),
    AppEvent::AndroidAction("android.intent.action.SEND_MULTIPLE"),
    AppEvent::AndroidAction("android.intent.action.VIEW"),
    AppEvent::AndroidAction("com.discord.intent.action.CONNECT"),
    AppEvent::AndroidAction("com.discord.intent.action.SDK"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComDiscordShareShareactivity => "com.discord.share.ShareActivity",
            AppEvent::ComDiscordMainMainactivity => "com.discord.main.MainActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
