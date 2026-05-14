use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_shopee_app_ui_home_homeactivity;
pub mod com_shopee_app_ui_proxy_proxyactivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComShopeeAppUiHomeHomeactivity => com_shopee_app_ui_home_homeactivity::handle_event(event, state, capabilities),
        AppEvent::ComShopeeAppUiProxyProxyactivity => com_shopee_app_ui_proxy_proxyactivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "android.intent.action.MAIN" => com_shopee_app_ui_home_homeactivity::handle_event(event, state, capabilities),
            "android.intent.action.VIEW" => com_shopee_app_ui_proxy_proxyactivity::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
