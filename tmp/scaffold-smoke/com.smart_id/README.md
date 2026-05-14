# Edgerun APK Replacement Scaffold

- Source Android package: `com.smart_id`
- Unique API signatures: `8827`
- Total static call sites: `288927`
- Workflow modules: `3`

This scaffold is generated from static APK analysis. It is not a working app yet.

Next steps:
- capture launch-to-first-use screen flow and selected UI actions
- identify camera/media entrypoints and whether they are core workflow or vendor SDK
- identify exact user action that requests location
- record actual contacted domains and authentication redirects
- record which services/receivers fire during normal use
- toggle third-party modules and observe breakage before default opt-out
