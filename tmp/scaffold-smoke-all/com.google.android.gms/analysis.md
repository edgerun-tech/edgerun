# APK Behavior Analysis

## Identity

- Source package: `com.google.android.gms`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `19333`
- Static call sites: `1075155`

## Event Model

- `ComGoogleAndroidGmsAccountsettingsUiZeropartyentrypointactivity` routes to `com.google.android.gms.accountsettings.ui.ZeroPartyEntryPointActivity` (activity)
  - Android action `com.android.settings.action.VIEW_ACCOUNT`
  - Android action `com.google.android.gms.accountsettings.VIEW_SETTINGS_0P`
  - Android action `com.google.android.gms.accountsettings.action.SAFETY_CENTER_SECURITY_CHECKUP`
- `ComGoogleAndroidGmsAccountsettingsUiSettingsloaderactivity` routes to `com.google.android.gms.accountsettings.ui.SettingsLoaderActivity` (activity)
  - Android action `com.google.android.gms.accountsettings.action.BROWSE_SETTINGS`
  - Android action `com.google.android.gms.accountsettings.action.VIEW_SETTINGS`

## Main Workflows

- `com.google.android.gms.accountsettings.ui.ZeroPartyEntryPointActivity` (activity) exported=false
  actions: `com.android.settings.action.VIEW_ACCOUNT`, `com.google.android.gms.accountsettings.VIEW_SETTINGS_0P`, `com.google.android.gms.accountsettings.action.SAFETY_CENTER_SECURITY_CHECKUP`
  method `<init>`: platform=0, internal=1
- `com.google.android.gms.accountsettings.ui.SettingsLoaderActivity` (activity) exported=true
  actions: `com.google.android.gms.accountsettings.action.BROWSE_SETTINGS`, `com.google.android.gms.accountsettings.action.VIEW_SETTINGS`
  method `<init>`: platform=0, internal=1

## Capability Evidence

- `accounts_identity`: 1425 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 57373 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 2705 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 2194 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 1134 call sites; contacts or calendar provider access
- `crypto_security`: 11401 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 14754 call sites; local database or preference storage behavior
- `files_storage`: 22606 call sites; file, document, media store, or filesystem behavior
- `location`: 2287 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 12196 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 977 call sites; notification posting, channels, or listener behavior
- `package_intents`: 61227 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 1356 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 1257 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 59017 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- No generated workflow method currently records capability intents.

## UI And Resource Hints

- Resource buckets: `color`(358) `color-v31`(82)
- UI elements: `entry`(74) `language`(1)
- Asset samples:
  - `assets/Activities.pb`
  - `assets/AltFormats_255`

## Network And Native Surface

- Static domain strings:
  - `accounts.g.cn`
  - `accounts.googel.cn`
- Native libraries:
  - `lib/x86/libandroidx.graphics.path.so`
  - `lib/x86/libconscrypt_gmscore_jni.so`

## Third-Party Modules

- `AndroidX Camera`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required
- `Bouncy Castle crypto`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required

## Runtime Observation Needed

- capture launch-to-first-use screen flow and selected UI actions
- identify camera/media entrypoints and whether they are core workflow or vendor SDK
- identify exact user action that requests location
- record actual contacted domains and authentication redirects
- record which services/receivers fire during normal use
- toggle third-party modules and observe breakage before default opt-out
