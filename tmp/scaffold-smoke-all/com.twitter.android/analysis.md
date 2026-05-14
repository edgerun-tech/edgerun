# APK Behavior Analysis

## Identity

- Source package: `com.twitter.android`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `11247`
- Static call sites: `1062707`

## Event Model

- `ComTwitterAppSettingsSettingsrootcompatactivity` routes to `com.twitter.app.settings.SettingsRootCompatActivity` (activity)
  - Android action `android.intent.action.MAIN`
- `ComTwitterAndroidAuthorizeappactivity` routes to `com.twitter.android.AuthorizeAppActivity` (activity)

## Main Workflows

- `com.twitter.app.settings.SettingsRootCompatActivity` (activity) exported=false
  actions: `android.intent.action.MAIN`
  method `<init>`: platform=0, internal=1
- `com.twitter.android.AuthorizeAppActivity` (activity) exported=true
  method `onActivityResult`: platform=11, internal=1 signals=package_intents:9, activity_ui:2
  method `onCreate`: platform=1, internal=0
  method `<clinit>`: platform=0, internal=1
  method `<init>`: platform=0, internal=1
  method `contains$005`: platform=1, internal=0

## Capability Evidence

- `accounts_identity`: 230 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 47245 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 76 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 2923 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 422 call sites; contacts or calendar provider access
- `crypto_security`: 6781 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 5542 call sites; local database or preference storage behavior
- `files_storage`: 10331 call sites; file, document, media store, or filesystem behavior
- `location`: 373 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 6511 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 307 call sites; notification posting, channels, or listener behavior
- `package_intents`: 21844 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 593 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 106 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 20364 call sites; jobs, alarms, wake locks, services, or background work

## UI And Resource Hints

- Resource buckets: `drawable`(2362) `layout`(1734)
- UI elements: `com.twitter.ui.components.text.legacy.TypefacesTextView`(1552) `LinearLayout`(1115)
- Literal UI text samples:
  - `+100 * big long title that needs to be marqueed`
  - `Attach database snapshot (PII will be removed)`
- Asset samples:
  - `assets/0nGX4yaFJEWalqN5`
  - `assets/4UhwGqg4y9Tb1ugx`

## Network And Native Surface

- Static domain strings:
  - `about.x.com`
  - `abs.twimg.com`
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
