# APK Behavior Analysis

## Identity

- Source package: `com.google.android.partnersetup`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `1745`
- Static call sites: `17555`

## Event Model

- `ComGoogleAndroidPartnersetupBootreceiver` routes to `com.google.android.partnersetup.BootReceiver` (receiver)
  - Android action `android.intent.action.BOOT_COMPLETED`
  - Android action `android.intent.action.MY_PACKAGE_REPLACED`
- `ComGoogleAndroidPartnersetupPhonestatereceiver` routes to `com.google.android.partnersetup.PhoneStateReceiver` (receiver)
  - Android action `android.intent.action.SIM_STATE_CHANGED`

## Main Workflows

- `com.google.android.partnersetup.BootReceiver` (receiver) exported=false
  actions: `android.intent.action.BOOT_COMPLETED`, `android.intent.action.MY_PACKAGE_REPLACED`
  method `onReceive`: platform=20, internal=25 signals=package_intents:12
  method `<clinit>`: platform=0, internal=1
  method `<init>`: platform=0, internal=1
- `com.google.android.partnersetup.PhoneStateReceiver` (receiver) exported=true
  actions: `android.intent.action.SIM_STATE_CHANGED`
  method `onReceive`: platform=8, internal=17 signals=package_intents:5
  method `<clinit>`: platform=0, internal=1
  method `<init>`: platform=0, internal=1

## Capability Evidence

- `accounts_identity`: 11 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 203 call sites; Android UI, activity, dialog, window, or view behavior
- `camera_media_capture`: 2 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 3 call sites; contacts or calendar provider access
- `crypto_security`: 27 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 265 call sites; local database or preference storage behavior
- `files_storage`: 626 call sites; file, document, media store, or filesystem behavior
- `network_web`: 128 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 70 call sites; notification posting, channels, or listener behavior
- `package_intents`: 698 call sites; intent, package manager, broadcast, or cross-app behavior
- `sms_telephony`: 1 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 923 call sites; jobs, alarms, wake locks, services, or background work

## UI And Resource Hints

- Resource buckets: `xml`(1)
- UI elements: `entry`(74) `language`(1)
- Asset samples:
  - `assets/dexopt/baseline.prof`
  - `assets/dexopt/baseline.profm`

## Network And Native Surface

- Static domain strings: none found
- Native libraries: none found

## Third-Party Modules

- `Google Play services`: default=`preserve_minimal`, breakage=`medium`; often carries auth, push, maps, safety, or wearable integrations; analytics should remain opt-out

## Runtime Observation Needed

- capture launch-to-first-use screen flow and selected UI actions
- identify camera/media entrypoints and whether they are core workflow or vendor SDK
- record actual contacted domains and authentication redirects
- record which services/receivers fire during normal use
- toggle third-party modules and observe breakage before default opt-out
