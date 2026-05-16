# Edgerun Server Deployment

These files are public example deployment material, not a live production
configuration. Replace every `example.edgerun.local`, `203.0.113.10`, and
`REPLACE_WITH_*` value before using them on an internet-facing host.

## Configuration Files

| File | Purpose |
|------|---------|
| `01-dns-zone.yaml` | Example DNS zone - A, MX, SPF, DMARC, CAA, MTA-STS, TLS-RPT, service host records |
| `02-dns-zone-dkim.yaml` | DKIM public key DNS record (selector: mail) |
| `03-smtp-server.yaml` | SMTP server config (implemented: port 25 MX, port 465 SMTPS submission, port 587 STARTTLS submission, relay) |
| `04-imap-server.yaml` | IMAP server config (port 143, IMAPS) |
| `05-browser-apps.yaml` | Browser-node app catalog and policy for Dash surfaces |
| `edgerun-server.service` | Host-only systemd unit with boot target enablement, failure restart, and sandboxing |
| `edgerun-dns.service` | DNS-only systemd example for hosts that run the DNS role separately |
| `edgerun-codelyzer.service` | User-level codelyzer/Xray bridge example for local repo inspection |
| `edgerun-server-health.service` | Host-only systemd oneshot health check using `edgerun-server --health-check` |
| `edgerun-server-health.timer` | Host-only systemd timer that runs the health check every 15 minutes |
| `edgerun-server-report.service` | Host-only systemd oneshot that delivers a local machine report email |
| `edgerun-server-report.timer` | Host-only systemd timer that sends the report every 12 hours |

## Required Key Material

| Name | Description |
|------|-------------|
| `/etc/edgerun/server/dkim-mail.private.pem` | DKIM RSA private key (PEM format) |
| `/etc/edgerun/server/tls/fullchain.pem` | TLS certificate chain (PEM format), generated or renewed by `edgerun-node` ACME orchestration |
| `/etc/edgerun/server/tls/privkey.pem` | TLS private key (PEM format), generated or renewed by `edgerun-node` ACME orchestration |
| `/etc/edgerun/server/dnssec-zone-ksk.pem` | DNSSEC ECDSAP256SHA256 private key for the configured zone |

`edgerun-server` consumes these paths through `edgerun-config` YAML fields
(`dkim_key_path`, `tls_cert`, and `tls_key`). The DKIM TXT record in
`02-dns-zone-dkim.yaml` must contain the public key produced from the private
key at `/etc/edgerun/server/dkim-mail.private.pem`.

## DNS Records Generated

- `@ A 203.0.113.10`
- `mail A 203.0.113.10`
- `mta-sts A 203.0.113.10`
- `blog A 203.0.113.10`
- `ns1 A 203.0.113.10`
- `ns2 A 203.0.113.10`
- `@ MX 0 mail.example.edgerun.local`
- `nodes MX 0 mail.example.edgerun.local`
- `@ TXT v=spf1 mx -all`
- `nodes TXT v=spf1 -all`
- `@ CAA 0 issue letsencrypt.org`
- `@ CAA 0 iodef mailto:admin@example.edgerun.local`
- `_dmarc TXT v=DMARC1; p=quarantine; rua=mailto:dmarc-reports@example.edgerun.local`
- `_mta-sts TXT v=STSv1; id=<policy-id>`
- `_smtp._tls TXT v=TLSRPTv1; rua=mailto:tls-reports@example.edgerun.local`
- `mail._domainkey TXT v=DKIM1; k=rsa; p=<public-key>`

## Mail Transport Hardening

- MTA-STS is implemented in code: the server serves
  `https://mta-sts.example.edgerun.local/.well-known/mta-sts.txt` with
  `mode: enforce`, `mx: mail.example.edgerun.local`, and `max_age: 604800`.
- TLS-RPT is a DNS/reporting configuration requirement. Reports are directed to
  `tls-reports@example.edgerun.local`; delivery depends on that local mailbox or
  catch-all remaining configured.
- CAA is implemented as generated DNS record material and restricts public
  issuance to Let's Encrypt for this zone.
- DNSSEC is implemented in code for host-side authoritative serving:
  `edgerun-server` signs configured zones with persistent ECDSAP256SHA256 key
  material, publishes DNSKEY/NSEC/RRSIG records, and refreshes signed zones
  before signature expiry. Public chain validation additionally requires the
  generated DS record to remain installed at the registrar.

## Health Checks

`edgerun-server --health-check --config /etc/edgerun/server/server.yaml` is a
host-only operational check. It verifies configured local listeners, plaintext
SMTP/IMAP banners, the local HTTP surface, TLS socket listeners, and DNSSEC
answers for signed zones, including DNSKEY/RRSIG and NXDOMAIN NSEC proofs. It
also parses configured TLS certificates and fails when a certificate is not
currently valid or has less than 14 days remaining.

## Service Ports

| Service | Port | Protocol |
|---------|------|----------|
| SMTP | 25 | SMTP (with STARTTLS) |
| SMTP submission | 465 | SMTPS (implicit TLS, authenticated when users have passwords) |
| SMTP submission | 587 | SMTP with STARTTLS (authenticated when users have passwords) |
| IMAP | 143 | IMAP (with STARTTLS) |
| IMAP | 993 | IMAPS (implicit TLS) |
| DNS | 53 | UDP/TCP |

## Dash Browser Apps

The configured Dash host is implemented in code as the human-facing workspace. The
`BrowserApp` and `BrowserNodePolicy` resources in `05-browser-apps.yaml` are
implemented configuration material that describe browser-loadable Wasm agents
and the capability selectors they request. Dash serves same-origin modules from
`/modules/*.wasm`; the first app module emits canonical Edgerun v0
`CapabilityDescriptor` and `QueryRequest` records through the browser-node host
imports. Browser-local app state is exposed as a capability-scoped
`vfs://browser/<app-id>/...` path backed by the browser node, so Wasm apps can
use a filesystem-shaped interface without bypassing grants. Current surface
rendering is still server-rendered HTML fragments, but the browser app boundary
now carries protocol records instead of an ad hoc app format. When no
`BrowserApp` resources are configured, the server exposes default Build Log,
Code, and Mail app catalog entries so the workspace remains navigable.

## systemd

`edgerun-server.service` is host-only deployment material. It is intended to be
installed in `/etc/systemd/system/`, then enabled with `systemctl enable
edgerun-server.service` so systemd starts it through `multi-user.target` on
boot. Runtime crash handling is configured with `Restart=on-failure` and
`RestartSec=5`. The unit is sandboxed with a static unprivileged user, a
bounded `CAP_NET_BIND_SERVICE` capability set, strict filesystem protection,
private temporary storage, namespace/realtime/personality restrictions,
kernel/sysctl/device protections, writable-executable memory denial, and a
denylist for high-risk syscall classes.

`edgerun-server-health.timer` is host-only deployment material. It should be
installed next to the service unit and enabled with
`systemctl enable --now edgerun-server-health.timer`; failures are reported by
the `edgerun-server-health.service` journal.

`edgerun-server-report.timer` is host-only deployment material for visibility
without external monitoring. It runs
`edgerun-server --send-system-report --config /etc/edgerun/server/server.yaml`
every 12 hours. The report is delivered directly into the configured local
Maildir for the catch-all or first configured user.
