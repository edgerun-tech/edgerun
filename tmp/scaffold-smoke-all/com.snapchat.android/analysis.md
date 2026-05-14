# APK Behavior Analysis

## Identity

- Source package: `com.snapchat.android`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `9593`
- Static call sites: `762726`

## Event Model

- `ComBraintreepaymentsApiBraintreebrowserswitchactivity` routes to `com.braintreepayments.api.BraintreeBrowserSwitchActivity` (activity)
  - Android action `android.intent.action.VIEW`
- `ComRazorpayCheckoutactivity` routes to `com.razorpay.CheckoutActivity` (activity)
  - Android action `android.intent.action.MAIN`

## Main Workflows

- `com.braintreepayments.api.BraintreeBrowserSwitchActivity` (activity) exported=true
  actions: `android.intent.action.VIEW`
  method `<init>`: platform=1, internal=1 signals=activity_ui:1
- `com.razorpay.CheckoutActivity` (activity) exported=false
  actions: `android.intent.action.MAIN`
  method `<init>`: platform=1, internal=0 signals=activity_ui:1

## Capability Evidence

- `accounts_identity`: 105 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 51648 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 164 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 2657 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 672 call sites; contacts or calendar provider access
- `crypto_security`: 892 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 3466 call sites; local database or preference storage behavior
- `files_storage`: 6910 call sites; file, document, media store, or filesystem behavior
- `location`: 701 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 7793 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 311 call sites; notification posting, channels, or listener behavior
- `package_intents`: 20691 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 431 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 116 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 13966 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- No generated workflow method currently records capability intents.

## UI And Resource Hints

- Resource buckets: `color`(91) `color-v23`(7)
- UI elements: `entry`(35) `language`(1)
- Asset samples:
  - `assets/account_challenge.valdimodule`
  - `assets/activity_center_api.valdimodule`

## Network And Native Surface

- Static domain strings:
  - `accounts.google.com`
  - `accounts.snapchat.com`
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
