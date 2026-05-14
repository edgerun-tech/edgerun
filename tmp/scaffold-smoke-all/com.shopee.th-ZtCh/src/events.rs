#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComShopeeAppUiHomeHomeactivity,
    ComShopeeAppUiProxyProxyactivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComShopeeAppUiHomeHomeactivity,
    AppEvent::ComShopeeAppUiProxyProxyactivity,
    AppEvent::AndroidAction("android.intent.action.MAIN"),
    AppEvent::AndroidAction("android.intent.action.VIEW"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComShopeeAppUiHomeHomeactivity => "com.shopee.app.ui.home.HomeActivity_",
            AppEvent::ComShopeeAppUiProxyProxyactivity => "com.shopee.app.ui.proxy.ProxyActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
