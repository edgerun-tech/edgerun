# APK Behavior Analysis

## Identity

- Source package: `com.google.android.contacts`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `9415`
- Static call sites: `208839`

## Event Model

- `ComGoogleAndroidAppsContactsActivitiesPeopleactivity` routes to `com.google.android.apps.contacts.activities.PeopleActivity` (activity)
- `ComGoogleAndroidAppsContactsActivitiesShoworcreateactivity` routes to `com.google.android.apps.contacts.activities.ShowOrCreateActivity` (activity)
  - Android action `com.android.contacts.action.SHOW_OR_CREATE_CONTACT`

## Main Workflows

- `com.google.android.apps.contacts.activities.PeopleActivity` (activity) exported=true
  method `onCreate`: platform=0, internal=4
  method `onActivityResult`: platform=0, internal=2
  method `onNewIntent`: platform=0, internal=2
  method `onResume`: platform=0, internal=2
  method `onStart`: platform=0, internal=2
- `com.google.android.apps.contacts.activities.ShowOrCreateActivity` (activity) exported=true
  actions: `com.android.contacts.action.SHOW_OR_CREATE_CONTACT`
  method `onCreate`: platform=15, internal=12 signals=network_web:5, package_intents:4, work_background:4
  method `a`: platform=19, internal=13 signals=package_intents:8, database_preferences:6, work_background:3
  method `onStop`: platform=0, internal=2
  method `<clinit>`: platform=0, internal=1
  method `<init>`: platform=0, internal=1

## Capability Evidence

- `accounts_identity`: 186 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 16430 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 3 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 91 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 317 call sites; contacts or calendar provider access
- `crypto_security`: 120 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 2613 call sites; local database or preference storage behavior
- `files_storage`: 3234 call sites; file, document, media store, or filesystem behavior
- `location`: 96 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 1516 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 250 call sites; notification posting, channels, or listener behavior
- `package_intents`: 12070 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 3 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 106 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 8646 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- `com.google.android.apps.contacts.activities.ShowOrCreateActivity` -> `onCreate` records:
  - `network.request_network_or_webview` from `network_web` evidence
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
  - `background_tasks.schedule_or_handle_background_work` from `work_background` evidence
- `com.google.android.apps.contacts.activities.ShowOrCreateActivity` -> `a` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
  - `local_state.read_or_write_local_state` from `database_preferences` evidence
  - `background_tasks.schedule_or_handle_background_work` from `work_background` evidence
  - `contacts_calendar.read_or_write_contact_calendar` from `contacts_calendar` evidence

## UI And Resource Hints

- Resource buckets: `color`(274) `color-v31`(55)
- UI elements: `entry`(75) `language`(1)
- Asset samples:
  - `assets/AltFormats_255`
  - `assets/AltFormats_27`

## Network And Native Surface

- Static domain strings:
  - `accounts.google.com`
  - `autopush-notifications-pa.sandbox.googleapis.com`
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
