# APK Behavior Analysis

## Identity

- Source package: `com.ss.android.ugc.trill`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `12614`
- Static call sites: `3338622`

## Event Model

- `NetOpenidAppauthRedirecturireceiveractivity` routes to `net.openid.appauth.RedirectUriReceiverActivity` (activity)
  - Android action `android.intent.action.VIEW`
- `ComSsAndroidUgcAwemeMusicAddtodspAuthRedirecturireceiveractivity` routes to `com.ss.android.ugc.aweme.music.addtodsp.auth.RedirectUriReceiverActivity` (activity)
  - Android action `android.intent.action.VIEW`

## Main Workflows

- `net.openid.appauth.RedirectUriReceiverActivity` (activity) exported=true
  actions: `android.intent.action.VIEW`
  method `onCreate`: platform=7, internal=9 signals=package_intents:5, activity_ui:3
  method `onStop`: platform=9, internal=1 signals=activity_ui:9
  method `attachBaseContext`: platform=6, internal=3 signals=activity_ui:1
  method `<init>`: platform=1, internal=0 signals=activity_ui:1
- `com.ss.android.ugc.aweme.music.addtodsp.auth.RedirectUriReceiverActivity` (activity) exported=true
  actions: `android.intent.action.VIEW`
  method `onCreate`: platform=0, internal=6
  method `onResume`: platform=0, internal=3
  method `onStart`: platform=0, internal=3
  method `setTheme`: platform=12, internal=10 signals=activity_ui:2, package_intents:2
  method `onStop`: platform=8, internal=2 signals=activity_ui:8

## Capability Evidence

- `accounts_identity`: 338 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 334184 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 125 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 4173 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 2283 call sites; contacts or calendar provider access
- `crypto_security`: 1343 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 10529 call sites; local database or preference storage behavior
- `files_storage`: 21656 call sites; file, document, media store, or filesystem behavior
- `location`: 1261 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 19127 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 560 call sites; notification posting, channels, or listener behavior
- `package_intents`: 60106 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 1214 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 152 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 95757 call sites; jobs, alarms, wake locks, services, or background work

## UI And Resource Hints

- Resource buckets: `b`(11198) `a`(4394)
- UI elements: `entry`(231) `fulfillment`(10)
- Asset samples:
  - `assets/10k_dark_web_filtered.txt`
  - `assets/I18N_sys_emoji.json`

## Network And Native Surface

- Static domain strings:
  - `accounts-staging.tokopedia.com`
  - `accounts.google.com`
- Native libraries: none found

## Third-Party Modules

- `AndroidX Room database`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required
- `AndroidX WorkManager`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required

## Runtime Observation Needed

- capture launch-to-first-use screen flow and selected UI actions
- identify camera/media entrypoints and whether they are core workflow or vendor SDK
- identify exact user action that requests location
- record actual contacted domains and authentication redirects
- record which services/receivers fire during normal use
- toggle third-party modules and observe breakage before default opt-out
