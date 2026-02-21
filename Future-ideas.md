mcp-server
edgerun registry for reusable modules

greed. Best product shape is:

  1. Instant browser onboarding (zero setup)

  - Hydrated “real” terminal UX.
  - Browser-native commands for first success:
      - edgerun init
      - edgerun run demo
      - edgerun deploy (wallet-signed)
  - Local vanity demo and docs integrated.

  2. Seamless upgrade to “full mode”

  - Same terminal UI, but command execution routed to an
    Edgerun-backed workspace.
  - Real Rust toolchain, real cargo, real wasm builds, real
    tests.
  - Users don’t have to learn a new interface when switching
    modes.

  3. Edgerun as acceleration backend

  - Workspace jobs scheduled on Edgerun.
  - Build caching + artifact reuse.
  - Optional distributed compile/test and replayable build
    receipts.
  - Trust controls (policy sessions + attestation) for build/
    deploy integrity.

  4. Single control plane

  - One auth/session model.
  - Wallet + org/project scopes.
  - Unified artifact registry for bundles produced from browser
    or full workspace.