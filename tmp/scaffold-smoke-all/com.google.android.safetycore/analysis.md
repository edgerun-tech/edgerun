# APK Behavior Analysis

## Identity

- Source package: `com.google.android.safetycore`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `2891`
- Static call sites: `45255`

## Event Model

- `ComGoogleAndroidLibrariesPhenotypeRegistrationPhenotypemetadataholderservice` routes to `com.google.android.libraries.phenotype.registration.PhenotypeMetadataHolderService` (service)
  - Android action `com.google.android.libraries.phenotype.registration.PhenotypeMetadataHolderService`
- `ComGoogleAndroidAppsSafetycoreServiceClassificationapiservice` routes to `com.google.android.apps.safetycore.service.ClassificationApiService` (service)
  - Android action `com.google.android.apps.safetycore.classification.BIND`

## Main Workflows

- `com.google.android.libraries.phenotype.registration.PhenotypeMetadataHolderService` (service) exported=false
  actions: `com.google.android.libraries.phenotype.registration.PhenotypeMetadataHolderService`
  method `onBind`: platform=1, internal=0
  method `<init>`: platform=0, internal=1
- `com.google.android.apps.safetycore.service.ClassificationApiService` (service) exported=true
  actions: `com.google.android.apps.safetycore.classification.BIND`
  method `onCreate`: platform=3, internal=11
  method `b`: platform=1, internal=13
  method `onDestroy`: platform=0, internal=6
  method `<init>`: platform=0, internal=3
  method `<clinit>`: platform=0, internal=1

## Capability Evidence

- `accounts_identity`: 40 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 354 call sites; Android UI, activity, dialog, window, or view behavior
- `camera_media_capture`: 8 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 21 call sites; contacts or calendar provider access
- `crypto_security`: 1166 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 315 call sites; local database or preference storage behavior
- `files_storage`: 1328 call sites; file, document, media store, or filesystem behavior
- `location`: 1 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 543 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 72 call sites; notification posting, channels, or listener behavior
- `package_intents`: 1030 call sites; intent, package manager, broadcast, or cross-app behavior
- `work_background`: 1731 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- No generated workflow method currently records capability intents.

## UI And Resource Hints

- Resource buckets: `xml`(1)
- UI elements: `entry`(78) `language`(1)
- Asset samples:
  - `assets/dexopt/baseline.prof`
  - `assets/dexopt/baseline.profm`

## Network And Native Surface

- Static domain strings:
  - `developer.android.com`
  - `developers.google.com`
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
