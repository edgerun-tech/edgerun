# APK Behavior Analysis

## Identity

- Source package: `com.google.android.apps.restore`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `7812`
- Static call sites: `136472`

## Event Model

- `ComGoogleAndroidAppsRestoreBackupApiserviceBackupoptinapiendpointservice` routes to `com.google.android.apps.restore.backup.apiservice.BackupOptInApiEndpointService` (service)
- `ComGoogleAndroidAppsRestoreBackupExternalstorageApiserviceExternalstoragebackupapiendpointservice` routes to `com.google.android.apps.restore.backup.externalstorage.apiservice.ExternalStorageBackupApiEndpointService` (service)

## Main Workflows

- `com.google.android.apps.restore.backup.apiservice.BackupOptInApiEndpointService` (service) exported=true
  method `onCreate`: platform=6, internal=21
  method `onBind`: platform=2, internal=7
  method `onDestroy`: platform=2, internal=5
  method `<init>`: platform=0, internal=3
  method `a`: platform=2, internal=0
- `com.google.android.apps.restore.backup.externalstorage.apiservice.ExternalStorageBackupApiEndpointService` (service) exported=true
  method `onBind`: platform=5, internal=16
  method `onCreate`: platform=5, internal=15
  method `onDestroy`: platform=2, internal=5
  method `<init>`: platform=0, internal=3

## Capability Evidence

- `accounts_identity`: 91 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 9405 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 6 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 100 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 49 call sites; contacts or calendar provider access
- `crypto_security`: 226 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 879 call sites; local database or preference storage behavior
- `files_storage`: 4191 call sites; file, document, media store, or filesystem behavior
- `location`: 42 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 961 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 88 call sites; notification posting, channels, or listener behavior
- `package_intents`: 6374 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 107 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 8 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 5837 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- No generated workflow method currently records capability intents.

## UI And Resource Hints

- Resource buckets: `drawable`(397) `layout`(284)
- UI elements: `LinearLayout`(363) `log-source`(299)
- Asset samples:
  - `assets/dexopt/baseline.prof`
  - `assets/dexopt/baseline.profm`

## Network And Native Surface

- Static domain strings:
  - `android.com`
  - `android.googleapis.com`
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
