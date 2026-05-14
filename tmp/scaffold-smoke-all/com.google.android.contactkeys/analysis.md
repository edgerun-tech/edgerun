# APK Behavior Analysis

## Identity

- Source package: `com.google.android.contactkeys`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `6623`
- Static call sites: `82131`

## Event Model

- `ComGoogleAndroidGmsContactkeysMainactivity` routes to `com.google.android.gms.contactkeys.MainActivity` (activity)
  - Android action `android.intent.action.INSERT`
  - Android action `android.intent.action.VIEW`
- `ComGoogleAndroidLibrariesPhenotypeRegistrationPhenotypemetadataholderservice` routes to `com.google.android.libraries.phenotype.registration.PhenotypeMetadataHolderService` (service)
  - Android action `com.google.android.libraries.phenotype.registration.PhenotypeMetadataHolderService`

## Main Workflows

- `com.google.android.gms.contactkeys.MainActivity` (activity) exported=true
  actions: `android.intent.action.INSERT`, `android.intent.action.VIEW`
  method `g`: platform=4, internal=28 signals=activity_ui:4
  method `<init>`: platform=0, internal=4
  method `<clinit>`: platform=0, internal=1
- `com.google.android.libraries.phenotype.registration.PhenotypeMetadataHolderService` (service) exported=false
  actions: `com.google.android.libraries.phenotype.registration.PhenotypeMetadataHolderService`
  method `<init>`: platform=0, internal=1
  method `a`: platform=1, internal=0

## Capability Evidence

- `accounts_identity`: 52 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 7513 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 4 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 15 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 16 call sites; contacts or calendar provider access
- `crypto_security`: 76 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 251 call sites; local database or preference storage behavior
- `files_storage`: 1521 call sites; file, document, media store, or filesystem behavior
- `location`: 46 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 466 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 69 call sites; notification posting, channels, or listener behavior
- `package_intents`: 2999 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 6 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 1 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 2847 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- No generated workflow method currently records capability intents.

## UI And Resource Hints

- Resource buckets: `color`(242) `color-v31`(34)
- UI elements: `entry`(74) `language`(1)
- Asset samples:
  - `assets/AltFormats_255`
  - `assets/AltFormats_27`

## Network And Native Surface

- Static domain strings:
  - `fonts.gstatic.com`
  - `github.com`
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
