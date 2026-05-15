# APK Conversion Research

This note documents the APK analysis work done for building a safer Edgerun-style replacement pipeline for Android apps. The goal is not to run Android apps unchanged. The goal is to extract behavior, protocols, entrypoints, permissions, storage, and vendor dependencies so a new app can be rebuilt with explicit capabilities and user-controlled third-party modules.

## Analyzer Work

The current work is centered on `crates/edgerun-codelyzer/src/bin/apk_api_calls.rs`.

Built-in static extraction now covers:

- APK ZIP reading without external crates.
- DEX method refs, invoke targets, class summaries, method summaries, and argument/return types.
- Android API capability signals.
- AndroidManifest package, permissions, activities, services, receivers, providers, exported state, actions, and categories.
- Native libraries, assets, resource buckets, binary XML UI hints, literal UI text.
- Static URI extraction across HTTP, HTTPS, WebSocket, FTP/SFTP, MQTT, RTSP/RTMP, TCP/UDP, content, file, android-app, market, intent, geo, mailto, tel, and SMS schemes.
- Method-local `const-string` and URI constants.
- Scaffold output with typed events, capabilities, state, network allowlists, workflows, third-party module policy, `analysis.md`, and `conversion_plan.md`.

Optional external-tool mode was added via `--external-tools DIR`.

It can call installed tools:

- `jadx` for Java/Kotlin decompile output.
- `apktool` for decoded manifest, resources, and smali.
- `androguard` CLI and Python APIs for APK metadata, decoded manifest, components, DEX counts, URI strings, and interesting constants.
- `strings` for raw APK string extraction.

External reports added:

- `source-network.md`: JADX source evidence for Retrofit, OkHttp, WebSocket, URL connection, URI parsing, GraphQL, JSON, and related network patterns.
- `apktool-evidence.md`: apktool resource/smali evidence for deep links, network security config, cleartext policy, WebView, JS bridge, TLS, crypto, keystore, reflection, dynamic loading, process execution, storage, SQLite, preferences, clipboard, camera, location, Bluetooth, NFC, SMS, contacts, calendar, biometrics, Firebase, Play Services, Braze, Amplitude, Segment, Sentry, Datadog, Crashlytics, and related SDKs.
- `external-conversion-plan.md`: normalized external evidence counts, implications, Androguard component summary, and samples.

## Current Limitation

Large APKs expose a performance problem in the broad external evidence scanner. WhatsApp completed, but Revolut’s decoded apktool tree was about 2 GB and the broad Python scanner was too slow. A targeted `rg` summary was used instead.

The Rust analyzer should be upgraded so external evidence scanning is:

- Streaming, not whole-tree accumulation.
- Category-specific instead of scanning every file with every broad pattern.
- Bounded by sample count per evidence kind.
- Progress-reporting for large decoded trees.
- Able to ignore library implementation internals unless app code references them.
- Able to read split APKs alongside `base.apk`.

## Run Artifacts

Analysis outputs were written under:

- `apk-analysis/partnersetup/`
- `apk-analysis/whatsapp/`
- `apk-analysis/revolut/`

`partnersetup` is a privileged Google/system app and is not a good target for the conversion model. System/privileged apps should generally be excluded from this work.

## WhatsApp Findings

Analyzed APK:

`/home/ken/.local/share/waydroid/data/app/~~0YYSfa5s_6Tr5mlSsry_0Q==/com.whatsapp-p2ETMs27vcOOzOSTRw4xAA==/base.apk`

Tool results:

- Androguard completed.
- apktool completed with unresolved resource warnings but usable output.
- JADX completed with 236 decompiler errors and still produced about 77,400 source files.
- Output size: about 1.5 GB.

Identity:

- Package: `com.whatsapp`
- Version: `2.26.17.72`
- DEX files: `10`
- DEX classes: `83,761`
- DEX methods: `425,122`

Top evidence:

- `smali_whatsapp`: `356,285`
- `smali_facebook_meta`: `33,426`
- `smali_file_io`: `19,427`
- `smali_preferences`: `11,016`
- `json_key`: `9,514`
- `resource_uri`: `8,381`
- `graphql`: `2,391`
- `smali_crypto`: `1,871`
- `smali_camera`: `1,595`
- `smali_location`: `841`
- `smali_signal_protocol`: `833`
- `smali_webview`: `778`
- `xmpp`: `250`
- `manifest_deep_link`: `134`

Interpretation:

WhatsApp is a hard but relevant user-app target. A replacement cannot be a simple wrapper. It needs a staged rebuild around:

1. Account registration/login, including SMS and phone verification.
2. Contacts import and identity mapping.
3. Chat list and message thread UI.
4. Message transport, likely XMPP/custom protocol plus GraphQL/Meta infrastructure.
5. Local message and media persistence.
6. Signal-protocol style key/session/message crypto.
7. Media capture, upload, download, and transcode.
8. Calls/VoIP/audio/video as later modules.
9. Backup/restore as a later module.
10. Optional Meta/Facebook/Firebase/Play Services adapters, disabled by default where possible.

Primary risk:

The core behavior depends on custom messaging transport and crypto/session state. Static analysis can identify modules, constants, and protocols, but usable behavior needs module-level fuzzing or controlled protocol reconstruction.

## Revolut Findings

Analyzed APK:

`/home/ken/.local/share/waydroid/data/app/~~kjVi1ISXLcs--2Fv8pc5Pw==/com.revolut.revolut-VWrWeKeT-uC072uUazeW3A==/base.apk`

Related split APKs were present but not folded into this pass:

- `split_config.hdpi.apk`
- `split_config.en.apk`
- `split_config.arm64_v8a.apk`

Tool results:

- Androguard completed.
- apktool completed with unresolved resource warnings but usable output.
- JADX completed with 445 decompiler errors and still produced about 180,489 source files.
- Output size: about 3.3 GB.
- apktool tree: about 2.0 GB.

Identity:

- Package: `com.revolut.revolut`
- Version: `10.129.1`
- DEX files: `23`
- DEX classes: `241,584`
- DEX methods: `1,257,095`

Top targeted evidence:

- `banking`: `1,075,507`
- `kyc_vendor`: `277,689`
- `crypto_keystore`: `218,328`
- `auth_login`: `178,458`
- `camera_media`: `139,403`
- `analytics_flags`: `136,512`
- `persistence`: `92,817`
- `nfc_bluetooth_location`: `67,528`
- `anti_tamper`: `24,429`
- `retrofit`: `22,353`
- `okhttp`: `20,733`
- `biometric`: `16,970`
- `process_dynamic`: `12,459`
- `webview_js`: `5,225`
- `plaid`: `4,045`
- `websocket`: `1,052`
- `uri_deeplink`: `974`
- `contacts_phone`: `687`
- `graphql`: `45`

Manifest and component signals:

- Protected app component factory/application.
- Backups disabled.
- Cleartext traffic disabled.
- Deep links for Revolut Pay, Open Banking, UPI, Plaid, merchant redirects, and access recovery.
- Login PIN, 2FA, Sign SMS, card management, crypto, trading, KYC, contacts, NFC, biometrics, phone state, screen capture detection, and media projection surfaces.
- Vendor KYC activities from Incode and Onfido.
- Vendor/infra services including Firebase, Datadog/Sentry-style telemetry, feature flags, Play Services, and related integrations.

Interpretation:

Revolut is a high-value but very high-risk finance target. A usable replacement requires explicit module boundaries and cannot preserve opaque SDK behavior by default.

The first modules would be:

1. Strict banking network layer: Retrofit/OkHttp/WebSocket evidence is present.
2. Login/session/auth: PIN/passcode, 2FA, SMS/signing flows.
3. Secure storage: keystore, TLS, signing, session secrets.
4. Device integrity: root/debug/emulator/tamper/attestation decisions.
5. Deep-link router: Revolut Pay, Open Banking, UPI, Plaid, merchant redirects, access recovery.
6. Banking state: accounts, cards, transfers, credit, trading, crypto, beneficiaries.
7. KYC adapters: Onfido/Incode/document/selfie verification.
8. Sensitive capabilities: camera, biometrics, contacts, phone state, NFC, Bluetooth, location.
9. Persistence: accounts/cards/transfers/KYC/risk/settings/caches.
10. Optional adapters: Firebase, Datadog, Sentry, LaunchDarkly, analytics, attribution.

Primary risk:

Anti-tamper and device integrity behavior is central. A secure replacement needs a clear policy: preserve server-required checks where unavoidable, replace Android-specific checks with transparent Edgerun attestation where possible, and expose any optional telemetry/ad/attribution modules to the user.

## App Selection Policy

Good targets:

- Non-privileged user apps.
- Apps whose core behavior can be reconstructed from protocols, local state, UI flows, and user-granted capabilities.
- Apps where optional vendor SDKs can be disabled or replaced without destroying the main workflow.

Bad targets:

- Privileged system apps.
- Apps whose only purpose is OS integration, package management, device policy, or vendor services.
- Apps that require hidden platform privileges to be useful.

## Conversion Strategy

The conversion pipeline should move through these stages:

1. Extract manifest entrypoints, permissions, components, deep links, and URI schemes.
2. Extract DEX calls, method-local constants, static URIs, class summaries, and capability signals.
3. Run external decompilers and resource/smali scanners for evidence that DEX call summaries miss.
4. Normalize evidence into a conversion plan.
5. Split app behavior into modules:
   - launch/session
   - network/protocol
   - local state
   - UI workflows
   - sensitive capabilities
   - third-party adapters
   - native/dynamic code
6. Generate a scaffold with explicit events, state, capabilities, network allowlists, and vendor module policy.
7. Fuzz or execute individual modules where static evidence cannot determine behavior.
8. Keep vendor modules opt-in where breakage risk allows it.

## Next Engineering Steps

Short-term:

- Move the targeted external evidence scan into Rust.
- Add streaming traversal, progress output, and per-kind sample limits.
- Add split APK awareness.
- Add a system-app filter so privileged/vendor OS apps are skipped by default.
- Add summary comparison across apps.

Medium-term:

- Build a module graph from manifest entrypoints, class/package ownership, constants, and decompiled source references.
- Extract route tables for Retrofit, GraphQL/Pando, WebSocket, XMPP-like protocols, and custom URI routers.
- Extract local persistence schema hints from SQLite/Room/SQL strings.
- Extract KYC/payment/identity provider modules and classify them as required, optional, or replaceable.
- Add native library inventory and JNI call surface summaries.

Long-term:

- Add module-level fuzzing for handlers, parsers, deep links, and protocol codecs.
- Add controlled execution harnesses for individual Java/Kotlin modules where dependencies can be stubbed.
- Generate a practical Edgerun replacement skeleton that implements the core loop first, while leaving risky or vendor-specific modules behind explicit adapters.
