use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_thingclips_social_amazon_activity_triplealexaaccountlinkactivity;
pub mod com_thingclips_social_amazon_activity_alexaauthactivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComThingclipsSocialAmazonActivityTriplealexaaccountlinkactivity => com_thingclips_social_amazon_activity_triplealexaaccountlinkactivity::handle_event(event, state, capabilities),
        AppEvent::ComThingclipsSocialAmazonActivityAlexaauthactivity => com_thingclips_social_amazon_activity_alexaauthactivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "android.intent.action.VIEW" => com_thingclips_social_amazon_activity_triplealexaaccountlinkactivity::handle_event(event, state, capabilities),
            "android.intent.action.VIEW" => com_thingclips_social_amazon_activity_alexaauthactivity::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
