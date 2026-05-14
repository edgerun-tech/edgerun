pub const STATIC_ENDPOINT_URLS: &[&str] = &[
    "https://api.onfido.com",
    "https://api.usercentrics.eu",
    "https://api.avo.app/inspector/v1/track",
    "https://api.shakebugs.com/",
    "https://consent-api.service.consent.eu1.usercentrics.eu",
    "https://consent-api.service.consent.usercentrics.eu",
    "https://accounts.google.com/o/oauth2/revoke?token=",
    "https://documentation.onfido.com/api/latest/#sdk-tokens",
    "https://global.fd-api.com/",
    "https://localization.fd-api.com/",
    "https://static.fd-api.com/feature-config/{env}/",
    "https://login.klarna.com/eu/lp/idp/.well-known/openid-configuration",
];

pub const STATIC_URLS: &[&str] = &[
    "http://g.co/dev/packagevisibility",
    "http://goo.gl/8Rd3yj",
    "http://goo.gl/naFqQk",
    "http://ns.adobe.com/xap/1.0/",
    "http://ns.adobe.com/xap/1.0/��",
    "http://schemas.android.com/apk/res-auto",
    "http://schemas.android.com/apk/res/android",
    "http://www.bouncycastle.org",
    "http://www.ccil.org/~cowan/tagsoup/features/bogons-empty",
    "http://www.ccil.org/~cowan/tagsoup/features/cdata-elements",
    "http://www.ccil.org/~cowan/tagsoup/features/default-attributes",
    "http://www.ccil.org/~cowan/tagsoup/features/ignorable-whitespace",
];

pub const ALLOW_DOMAINS: &[&str] = &[
    "accounts.google.com",
    "aggregator.eu.usercentrics.eu",
    "aggregator.service.usercentrics.eu",
    "aomedia.org",
    "api.avo.app",
    "api.onfido.com",
    "api.shakebugs.com",
    "api.usercentrics.eu",
    "app-measurement.com",
    "app.adjust.com",
    "app.adjust.io",
    "app.eu.usercentrics.eu",
];

pub fn is_allowed_domain(domain: &str) -> bool {
    ALLOW_DOMAINS.iter().any(|allowed| *allowed == domain)
}
