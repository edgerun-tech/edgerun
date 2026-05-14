use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_discord_share_shareactivity;
pub mod com_discord_main_mainactivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComDiscordShareShareactivity => com_discord_share_shareactivity::handle_event(event, state, capabilities),
        AppEvent::ComDiscordMainMainactivity => com_discord_main_mainactivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "android.intent.action.SEND" => com_discord_share_shareactivity::handle_event(event, state, capabilities),
            "android.intent.action.SEND_MULTIPLE" => com_discord_share_shareactivity::handle_event(event, state, capabilities),
            "android.intent.action.VIEW" => com_discord_main_mainactivity::handle_event(event, state, capabilities),
            "com.discord.intent.action.CONNECT" => com_discord_main_mainactivity::handle_event(event, state, capabilities),
            "com.discord.intent.action.SDK" => com_discord_main_mainactivity::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
