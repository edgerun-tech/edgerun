# Security

EdgeRun is a research and pre-release infrastructure workspace. Do not deploy it
as production infrastructure without reviewing the exact crate, binary, config,
and proof path you intend to expose.

## Public Snapshot Hygiene

This repository should not contain private keys, API tokens, live `.env` files,
or machine-local build products. Deployment files under `deploy/server/` are
templates and use example hostnames, TEST-NET addresses, and `REPLACE_WITH_*`
placeholders by design.

If you find a real credential in the public repository, treat it as compromised:
revoke it first, then open an issue or contact the maintainer.

## Reporting

Please report security issues through GitHub issues unless disclosure would put
active users or infrastructure at risk. For sensitive reports, ask for a private
contact path in a minimal public issue or email `kensservices@gmail.com`.
