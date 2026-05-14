# APK Behavior Analysis

## Identity

- Source package: `com.shopee.th`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `10850`
- Static call sites: `789695`

## Event Model

- `ComShopeeAppUiHomeHomeactivity` routes to `com.shopee.app.ui.home.HomeActivity_` (activity)
  - Android action `android.intent.action.MAIN`
- `ComShopeeAppUiProxyProxyactivity` routes to `com.shopee.app.ui.proxy.ProxyActivity` (activity)
  - Android action `android.intent.action.VIEW`

## Main Workflows

- `com.shopee.app.ui.home.HomeActivity_` (activity) exported=true
  actions: `android.intent.action.MAIN`
  method `onActivityResult`: platform=26, internal=3 signals=work_background:20, package_intents:6
  method `onCreate`: platform=0, internal=2
  method `q7`: platform=16, internal=0 signals=work_background:14, package_intents:2, activity_ui:1
  method `setContentView`: platform=0, internal=6
  method `<init>`: platform=0, internal=2
- `com.shopee.app.ui.proxy.ProxyActivity` (activity) exported=true
  actions: `android.intent.action.VIEW`
  method `onCreate`: platform=56, internal=108 signals=package_intents:9, network_web:8, activity_ui:4
  method `onNewIntent`: platform=5, internal=11 signals=package_intents:3, activity_ui:1
  method `I6`: platform=101, internal=124 signals=network_web:21, package_intents:12, activity_ui:2
  method `G6`: platform=119, internal=64 signals=package_intents:45, network_web:34, activity_ui:12
  method `attachBaseContext`: platform=9, internal=14 signals=work_background:4, activity_ui:1

## Capability Evidence

- `accounts_identity`: 178 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 77746 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 87 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 4270 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 642 call sites; contacts or calendar provider access
- `crypto_security`: 1364 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 6339 call sites; local database or preference storage behavior
- `files_storage`: 16060 call sites; file, document, media store, or filesystem behavior
- `location`: 578 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 6685 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 345 call sites; notification posting, channels, or listener behavior
- `package_intents`: 24146 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 1298 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 253 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 34764 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- `com.shopee.app.ui.home.HomeActivity_` -> `onActivityResult` records:
  - `background_tasks.schedule_or_handle_background_work` from `work_background` evidence
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
- `com.shopee.app.ui.home.HomeActivity_` -> `q7` records:
  - `background_tasks.schedule_or_handle_background_work` from `work_background` evidence
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
- `com.shopee.app.ui.proxy.ProxyActivity` -> `onCreate` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
  - `network.request_network_or_webview` from `network_web` evidence
  - `background_tasks.schedule_or_handle_background_work` from `work_background` evidence
- `com.shopee.app.ui.proxy.ProxyActivity` -> `onNewIntent` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
- `com.shopee.app.ui.proxy.ProxyActivity` -> `I6` records:
  - `network.request_network_or_webview` from `network_web` evidence
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
- `com.shopee.app.ui.proxy.ProxyActivity` -> `G6` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
  - `network.request_network_or_webview` from `network_web` evidence
- `com.shopee.app.ui.proxy.ProxyActivity` -> `attachBaseContext` records:
  - `background_tasks.schedule_or_handle_background_work` from `work_background` evidence

## UI And Resource Hints

- Resource buckets: `layout`(1514) `drawable`(1448)
- UI elements: `LinearLayout`(1337) `TextView`(1198)
- Literal UI text samples:
  - `15% Cashback`
  - `AB Testing/Toggle/Config Status`
- Asset samples:
  - `assets/DataBinding_v9.hbc`
  - `assets/HummerDefinition_es5_v9.hbc`

## Network And Native Surface

- Static domain strings:
  - `access.line.me`
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
