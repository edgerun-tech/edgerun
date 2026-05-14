# Edgerun APK Replacement Scaffold

- Source Android package: `ee.mtakso.client`
- Unique API signatures: `11601`
- Total static call sites: `563230`
- Workflow modules: `2`

This scaffold is generated from static APK analysis. It is not a working app yet. Start with `analysis.md` for the extracted behavior map.

Next steps:
- capture launch-to-first-use screen flow and selected UI actions
- identify camera/media entrypoints and whether they are core workflow or vendor SDK
- identify exact user action that requests location
- record actual contacted domains and authentication redirects
- record which services/receivers fire during normal use
- toggle third-party modules and observe breakage before default opt-out
