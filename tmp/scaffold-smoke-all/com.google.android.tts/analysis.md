# APK Behavior Analysis

## Identity

- Source package: `com.google.android.tts`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `7658`
- Static call sites: `106086`

## Event Model

- `ComGoogleAndroidLibrariesSpeechModelmanagerLanguagepackSettingsSettingsactivity` routes to `com.google.android.libraries.speech.modelmanager.languagepack.settings.SettingsActivity` (activity)
  - Android action `com.google.android.libraries.speech.modelmanager.languagepack.settings.SettingsActivity`
- `ComGoogleAndroidLibrariesSpeechModelmanagerLanguagepackSettingsAddlanguagesactivity` routes to `com.google.android.libraries.speech.modelmanager.languagepack.settings.AddLanguagesActivity` (activity)
  - Android action `com.google.android.libraries.speech.modelmanager.languagepack.settings.AddLanguagesActivity`

## Main Workflows

- `com.google.android.libraries.speech.modelmanager.languagepack.settings.SettingsActivity` (activity) exported=false
  actions: `com.google.android.libraries.speech.modelmanager.languagepack.settings.SettingsActivity`
  method `onCreate`: platform=6, internal=44 signals=activity_ui:1
  method `onActivityResult`: platform=1, internal=4
  method `onNewIntent`: platform=1, internal=4
  method `onRequestPermissionsResult`: platform=1, internal=4
  method `onResume`: platform=1, internal=4
- `com.google.android.libraries.speech.modelmanager.languagepack.settings.AddLanguagesActivity` (activity) exported=false
  actions: `com.google.android.libraries.speech.modelmanager.languagepack.settings.AddLanguagesActivity`
  method `onCreate`: platform=2, internal=18
  method `onActivityResult`: platform=1, internal=4
  method `onNewIntent`: platform=1, internal=4
  method `onRequestPermissionsResult`: platform=1, internal=4
  method `onResume`: platform=1, internal=4

## Capability Evidence

- `accounts_identity`: 36 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 8198 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 30 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 862 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 33 call sites; contacts or calendar provider access
- `crypto_security`: 898 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 500 call sites; local database or preference storage behavior
- `files_storage`: 2652 call sites; file, document, media store, or filesystem behavior
- `location`: 36 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 1169 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 91 call sites; notification posting, channels, or listener behavior
- `package_intents`: 4796 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 6 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 10 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 6351 call sites; jobs, alarms, wake locks, services, or background work

## UI And Resource Hints

- Resource buckets: `color`(265) `color-v31`(62)
- UI elements: `entry`(82) `language`(1)
- Asset samples:
  - `assets/dexopt/baseline.prof`
  - `assets/dexopt/baseline.profm`

## Network And Native Surface

- Static domain strings:
  - `default.url`
  - `developer.android.com`
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
