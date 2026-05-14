# APK Behavior Analysis

## Identity

- Source package: `com.global.foodpanda.android`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `10282`
- Static call sites: `550040`

## Event Model

- `ComDeliveryheroConfigDashboardUiConfigdashboardactivity` routes to `com.deliveryhero.config.dashboard.ui.ConfigDashboardActivity` (activity)
  - Android action `com.deliveryhero.config.dashboard.CONFIG_DASHBOARD`
- `ComDeliveryheroConfigDashboardUiExperimentationdashboardactivity` routes to `com.deliveryhero.config.dashboard.ui.ExperimentationDashboardActivity` (activity)
  - Android action `com.deliveryhero.config.dashboard.EXPERIMENTATION`
- `ComDeliveryheroPushServiceSpPushmessagingservice` routes to `com.deliveryhero.push.service.sp.PushMessagingService` (service)
  - Android action `com.google.firebase.MESSAGING_EVENT`

## Main Workflows

- `com.deliveryhero.config.dashboard.ui.ConfigDashboardActivity` (activity) exported=false
  actions: `com.deliveryhero.config.dashboard.CONFIG_DASHBOARD`
  method `onCreate`: platform=0, internal=5
  method `M`: platform=0, internal=18
  method `<init>`: platform=0, internal=6
  method `L`: platform=0, internal=2
  method `N`: platform=0, internal=1
- `com.deliveryhero.config.dashboard.ui.ExperimentationDashboardActivity` (activity) exported=false
  actions: `com.deliveryhero.config.dashboard.EXPERIMENTATION`
  method `onCreate`: platform=0, internal=5
  method `<init>`: platform=0, internal=1
- `com.deliveryhero.push.service.sp.PushMessagingService` (service) exported=false
  actions: `com.google.firebase.MESSAGING_EVENT`
  method `b`: platform=2, internal=4 signals=package_intents:2
  method `c`: platform=1, internal=2 signals=package_intents:1
  method `<init>`: platform=0, internal=1

## Capability Evidence

- `accounts_identity`: 64 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 27576 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 19 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 1258 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 308 call sites; contacts or calendar provider access
- `crypto_security`: 10399 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 3524 call sites; local database or preference storage behavior
- `files_storage`: 10093 call sites; file, document, media store, or filesystem behavior
- `location`: 295 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 3644 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 248 call sites; notification posting, channels, or listener behavior
- `package_intents`: 13858 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 436 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 79 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 20496 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- `com.deliveryhero.push.service.sp.PushMessagingService` -> `b` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
- `com.deliveryhero.push.service.sp.PushMessagingService` -> `c` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence

## UI And Resource Hints

- Resource buckets: `drawable`(1517) `layout`(698) `color`(213) `raw`(104) `anim`(68) `animator`(38) `font`(34) `color-v31`(32) `drawable-anydpi-v24`(32) `xml`(13) `anim-v21`(11) `drawable-v21`(10)
- UI elements: `com.deliveryhero.pretty.core.CoreTextView`(373) `LinearLayout`(372) `TextView`(340) `View`(294) `androidx.constraintlayout.widget.ConstraintLayout`(284) `include`(220) `ImageView`(177) `FrameLayout`(167) `com.deliveryhero.pretty.core.image.CoreImageView`(118) `RelativeLayout`(113) `androidx.compose.ui.platform.ComposeView`(86) `merge`(83)
- Literal UI text samples:
  - `(IN) +91`
  - `Api Environment:`
  - `Back Button`
  - `Cancel`
  - `Close Button`
  - `Description (Required)`
  - `Download our app directly from the Play Store to enjoy the best food experience`
  - `EditText`
  - `Email`
  - `Error would be shown here`
  - `ForgotPasswordDialog`
  - `Hide Ime`
- Asset samples:
  - `assets/adjust-aliases.json`
  - `assets/blaze_face_short_range.tflite`
  - `assets/braze-html-in-app-message-bridge.js`
  - `assets/config.preconditions`
  - `assets/configs/addressconfig_bd.json`
  - `assets/configs/addressconfig_hk.json`
  - `assets/configs/addressconfig_kh.json`
  - `assets/configs/addressconfig_la.json`
  - `assets/configs/addressconfig_mm.json`
  - `assets/configs/addressconfig_my.json`
  - `assets/configs/addressconfig_ph.json`
  - `assets/configs/addressconfig_pk.json`

## Network And Native Surface

- Static endpoint URL candidates:
  - `https://api.onfido.com`
  - `https://api.usercentrics.eu`
  - `https://api.avo.app/inspector/v1/track`
  - `https://api.shakebugs.com/`
  - `https://consent-api.service.consent.eu1.usercentrics.eu`
  - `https://consent-api.service.consent.usercentrics.eu`
  - `https://accounts.google.com/o/oauth2/revoke?token=`
  - `https://documentation.onfido.com/api/latest/#sdk-tokens`
  - `https://global.fd-api.com/`
  - `https://localization.fd-api.com/`
  - `https://static.fd-api.com/feature-config/{env}/`
  - `https://login.klarna.com/eu/lp/idp/.well-known/openid-configuration`
- Static URL strings:
  - `http://g.co/dev/packagevisibility`
  - `http://goo.gl/8Rd3yj`
  - `http://goo.gl/naFqQk`
  - `http://ns.adobe.com/xap/1.0/`
  - `http://ns.adobe.com/xap/1.0/��`
  - `http://schemas.android.com/apk/res-auto`
  - `http://schemas.android.com/apk/res/android`
  - `http://www.bouncycastle.org`
  - `http://www.ccil.org/~cowan/tagsoup/features/bogons-empty`
  - `http://www.ccil.org/~cowan/tagsoup/features/cdata-elements`
  - `http://www.ccil.org/~cowan/tagsoup/features/default-attributes`
  - `http://www.ccil.org/~cowan/tagsoup/features/ignorable-whitespace`
- Static domain strings:
  - `accounts.google.com`
  - `aggregator.eu.usercentrics.eu`
  - `aggregator.service.usercentrics.eu`
  - `aomedia.org`
  - `api.avo.app`
  - `api.onfido.com`
  - `api.shakebugs.com`
  - `api.usercentrics.eu`
  - `app-measurement.com`
  - `app.adjust.com`
  - `app.adjust.io`
  - `app.eu.usercentrics.eu`
- Native libraries: none found

## Third-Party Modules

- `Adjust attribution`: default=`opt_out`, breakage=`medium`; developer included it, but it is usually telemetry, attribution, campaigns, or support; user can enable with acknowledgement
- `AndroidX Camera`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required
- `AndroidX Room database`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required
- `AndroidX WorkManager`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required
- `Bouncy Castle crypto`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required
- `Braze engagement SDK`: default=`opt_out`, breakage=`medium`; developer included it, but it is usually telemetry, attribution, campaigns, or support; user can enable with acknowledgement
- `Chromium/WebView code`: default=`replace_gradually`, breakage=`high`; runtime or rendering framework; rebuild screen-by-screen instead of silently dropping it
- `Firebase`: default=`preserve_minimal`, breakage=`medium`; often carries auth, push, maps, safety, or wearable integrations; analytics should remain opt-out
- `Google Play libraries`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required
- `Google Play services`: default=`preserve_minimal`, breakage=`medium`; often carries auth, push, maps, safety, or wearable integrations; analytics should remain opt-out
- `Incognia risk/location SDK`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required
- `Klarna payment/auth SDK`: default=`preserve_if_checkout_required`, breakage=`high`; payment and auth redirects can be core checkout behavior and need explicit user-visible replacement

## Runtime Observation Needed

- capture launch-to-first-use screen flow and selected UI actions
- identify camera/media entrypoints and whether they are core workflow or vendor SDK
- identify exact user action that requests location
- record actual contacted domains and authentication redirects
- record which services/receivers fire during normal use
- toggle third-party modules and observe breakage before default opt-out
