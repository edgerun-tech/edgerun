# APK Behavior Analysis

## Identity

- Source package: `com.tcl.tclhome`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `15518`
- Static call sites: `521911`

## Event Model

- `ComTclBmmainSplashactivity` routes to `com.tcl.bmmain.SplashActivity` (activity)
  - Android action `android.intent.action.MAIN`
  - Android action `android.intent.action.VIEW`
- `ComTclBmloginuiUiAppflipThappfliptclhomeactivity` routes to `com.tcl.bmloginui.ui.appflip.THAppFlipTCLHomeActivity` (activity)
  - Android action `com.tcl.tclhome.appflip.obg.iot`

## Main Workflows

- `com.tcl.bmmain.SplashActivity` (activity) exported=true
  actions: `android.intent.action.MAIN`, `android.intent.action.VIEW`
  method `onCreate`: platform=51, internal=50 signals=package_intents:29, activity_ui:17, work_background:7
  method `handleIntent`: platform=0, internal=1
  method `checkAlexaCodeSendEvent`: platform=12, internal=8 signals=activity_ui:2, package_intents:2, network_web:1
  method `jumpToAdActivity`: platform=4, internal=14
  method `<init>`: platform=0, internal=8
- `com.tcl.bmloginui.ui.appflip.THAppFlipTCLHomeActivity` (activity) exported=true
  actions: `com.tcl.tclhome.appflip.obg.iot`
  method `onActivityResult`: platform=0, internal=2
  method `loadData`: platform=23, internal=16 signals=package_intents:8, activity_ui:4, work_background:3
  method `initTips`: platform=18, internal=10 signals=activity_ui:3, package_intents:3
  method `initFlipUnLinkGoogleAccount`: platform=15, internal=9 signals=activity_ui:2, package_intents:2
  method `initReviewGooglePolicy`: platform=15, internal=9 signals=activity_ui:2, package_intents:2

## Capability Evidence

- `accounts_identity`: 559 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 45027 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 1201 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 2518 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 802 call sites; contacts or calendar provider access
- `crypto_security`: 2346 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 4311 call sites; local database or preference storage behavior
- `files_storage`: 12933 call sites; file, document, media store, or filesystem behavior
- `location`: 405 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 14028 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 310 call sites; notification posting, channels, or listener behavior
- `package_intents`: 18581 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 401 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 48 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 22034 call sites; jobs, alarms, wake locks, services, or background work

## UI And Resource Hints

- Resource buckets: `mipmap-xxhdpi-v4`(1134) `layout`(1038)
- UI elements: `TextView`(2074) `LinearLayout`(883)
- Literal UI text samples:
  - `/kWh`
  - `11:00pm-07:00am`
- Asset samples:
  - `assets/1.png`
  - `assets/2.gif`

## Network And Native Surface

- Static domain strings:
  - `account-inn-test.tcljd.com`
  - `account-rus-test.tcljd.com`
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
