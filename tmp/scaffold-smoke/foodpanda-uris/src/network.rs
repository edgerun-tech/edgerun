pub const STATIC_ENDPOINT_URIS: &[&str] = &[
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
];

pub const STATIC_URIS: &[&str] = &[
    "android-app://androidx.navigation/",
    "android-app://com.google.android.googlequicksearchbox/https/www.google.com",
    "android-app://com.google.appcrawler",
    "content://",
    "content://%s/%s",
    "content://com.facebook.katana.provider.AttributionIdProvider",
    "content://com.facebook.wakizashi.provider.AttributionIdProvider",
    "content://com.google.android.gms.instantapps.provider.api/",
    "content://com.google.android.gms.phenotype/",
    "content://com.google.android.gsf.gservices",
];

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
];

pub fn is_allowed_domain(domain: &str) -> bool {
    ALLOW_DOMAINS.iter().any(|allowed| *allowed == domain)
}
