# APK Behavior Analysis

## Identity

- Source package: `com.revolut.revolut`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `11096`
- Static call sites: `1158871`

## Event Model

- `ComRevolutUiLoginPinLoginactivity` routes to `com.revolut.ui.login.pin.LoginActivity` (activity)
  - Android action `OPEN_MAIN_ACTIVITY`
  - Android action `android.intent.action.MAIN`
  - Android action `android.intent.action.VIEW`
  - Android action `com.revolut.category.CHAT_MESSAGE`
- `ComRevolutFeatureAppLauncherImplUiLauncheractivity` routes to `com.revolut.feature.app_launcher.impl.ui.LauncherActivity` (activity)

## Main Workflows

- `com.revolut.ui.login.pin.LoginActivity` (activity) exported=true
  actions: `OPEN_MAIN_ACTIVITY`, `android.intent.action.MAIN`, `android.intent.action.VIEW`, `com.revolut.category.CHAT_MESSAGE`
  method `onCreate`: platform=12, internal=50 signals=activity_ui:6, package_intents:1
  method `onStart`: platform=1, internal=3
  method `onActivityResult`: platform=1, internal=2 signals=activity_ui:1
  method `onNewIntent`: platform=1, internal=1 signals=activity_ui:1, package_intents:1
  method `onBackPressed`: platform=3, internal=6 signals=package_intents:2, activity_ui:1
- `com.revolut.feature.app_launcher.impl.ui.LauncherActivity` (activity) exported=true
  method `onCreate`: platform=0, internal=5
  method `t1`: platform=0, internal=2
  method `<init>`: platform=0, internal=1
  method `J1`: platform=0, internal=1

## Capability Evidence

- `accounts_identity`: 524 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 53279 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 151 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 1595 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 500 call sites; contacts or calendar provider access
- `crypto_security`: 9173 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 2601 call sites; local database or preference storage behavior
- `files_storage`: 10152 call sites; file, document, media store, or filesystem behavior
- `location`: 313 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 3416 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 182 call sites; notification posting, channels, or listener behavior
- `package_intents`: 12159 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 635 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 55 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 87428 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- `com.revolut.ui.login.pin.LoginActivity` -> `onCreate` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
- `com.revolut.ui.login.pin.LoginActivity` -> `onNewIntent` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
- `com.revolut.ui.login.pin.LoginActivity` -> `onBackPressed` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence

## UI And Resource Hints

- Resource buckets: `layout`(2739) `drawable`(2283)
- UI elements: `include`(1283) `ViewStub`(1072)
- Literal UI text samples:
  - `Address`
  - `CVV`
- Asset samples:
  - `assets/PublicSuffixDatabase.list`
  - `assets/aqueduct_products.properties`

## Network And Native Surface

- Static domain strings:
  - `account.firstvet.com`
  - `accounts.google.com`
- Native libraries: none found

## Third-Party Modules

- `AndroidX Camera`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required
- `AndroidX Room database`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required

## Runtime Observation Needed

- capture launch-to-first-use screen flow and selected UI actions
- identify camera/media entrypoints and whether they are core workflow or vendor SDK
- identify exact user action that requests location
- record actual contacted domains and authentication redirects
- record which services/receivers fire during normal use
- toggle third-party modules and observe breakage before default opt-out
