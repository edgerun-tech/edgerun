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

- No generated workflow method currently records capability intents.

## UI And Resource Hints

- Resource buckets: `drawable`(1517) `layout`(698)
- UI elements: `com.deliveryhero.pretty.core.CoreTextView`(373) `LinearLayout`(372)
- Literal UI text samples:
  - `(IN) +91`
  - `Api Environment:`
- Asset samples:
  - `assets/adjust-aliases.json`
  - `assets/blaze_face_short_range.tflite`

## Network And Native Surface

- Static domain strings:
  - `accounts.google.com`
  - `aggregator.eu.usercentrics.eu`
- Native libraries: none found

## Third-Party Modules

- `Adjust attribution`: default=`opt_out`, breakage=`medium`; developer included it, but it is usually telemetry, attribution, campaigns, or support; user can enable with acknowledgement
- `AndroidX Camera`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required

## Runtime Observation Needed

- capture launch-to-first-use screen flow and selected UI actions
- identify camera/media entrypoints and whether they are core workflow or vendor SDK
- identify exact user action that requests location
- record actual contacted domains and authentication redirects
- record which services/receivers fire during normal use
- toggle third-party modules and observe breakage before default opt-out
