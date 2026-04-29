# Edgerun Server Deployment

## Configuration Files

| File | Purpose |
|------|---------|
| `01-dns-zone.yaml` | DNS zone for `edgerun.tech` - A, MX, SPF, DMARC, CAA, MTA-STS, TLS-RPT, service host records |
| `02-dns-zone-dkim.yaml` | DKIM public key DNS record (selector: mail) |
| `03-smtp-server.yaml` | SMTP server config (implemented: port 25 MX, port 465 SMTPS submission, port 587 STARTTLS submission, relay) |
| `04-imap-server.yaml` | IMAP server config (port 143, IMAPS) |
| `edgerun-server.service` | Host-only systemd unit with boot target enablement, failure restart, and sandboxing |
| `edgerun-server-health.service` | Host-only systemd oneshot health check using `edgerun-server --health-check` |
| `edgerun-server-health.timer` | Host-only systemd timer that runs the health check every 15 minutes |

## Required Key Material

| Name | Description |
|------|-------------|
| `/etc/edgerun/server/dkim-mail.private.pem` | DKIM RSA private key (PEM format) |
| `/etc/edgerun/server/tls/fullchain.pem` | TLS certificate chain (PEM format), generated or renewed by `edgerun-acme` |
| `/etc/edgerun/server/tls/privkey.pem` | TLS private key (PEM format), generated or renewed by `edgerun-acme` |
| `/etc/edgerun/server/dnssec-edgerun-tech-ksk.pem` | DNSSEC ECDSAP256SHA256 private key for the `edgerun.tech` zone |

`edgerun-server` consumes these paths through `edgerun-config` YAML fields
(`dkim_key_path`, `tls_cert`, and `tls_key`). The DKIM TXT record in
`02-dns-zone-dkim.yaml` must contain the public key produced from the private
key at `/etc/edgerun/server/dkim-mail.private.pem`.

## DNS Records Generated

- `@ A 172.245.67.49`
- `mail A 172.245.67.49`
- `mta-sts A 172.245.67.49`
- `blog A 172.245.67.49`
- `ns1 A 172.245.67.49`
- `ns2 A 172.245.67.49`
- `@ MX 0 mail.edgerun.tech`
- `nodes MX 0 mail.edgerun.tech`
- `@ TXT v=spf1 mx -all`
- `nodes TXT v=spf1 -all`
- `@ CAA 0 issue letsencrypt.org`
- `@ CAA 0 iodef mailto:admin@edgerun.tech`
- `_dmarc TXT v=DMARC1; p=quarantine; rua=mailto:dmarc-reports@edgerun.tech`
- `_mta-sts TXT v=STSv1; id=<policy-id>`
- `_smtp._tls TXT v=TLSRPTv1; rua=mailto:tls-reports@edgerun.tech`
- `mail._domainkey TXT v=DKIM1; k=rsa; p=<public-key>`

## Mail Transport Hardening

- MTA-STS is implemented in code: the server serves
  `https://mta-sts.edgerun.tech/.well-known/mta-sts.txt` with `mode: enforce`,
  `mx: mail.edgerun.tech`, and `max_age: 604800`.
- TLS-RPT is a DNS/reporting configuration requirement. Reports are directed to
  `tls-reports@edgerun.tech`; delivery depends on that local mailbox or catch-all
  remaining configured.
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

## Blog Surface

`blog.edgerun.tech` is implemented in code by `edgerun-blog` and mounted into
`edgerun-server` with `--blog-host blog.edgerun.tech --blog-root /srv/blog`.
The blog can also generate deterministic static output from that Git checkout
with `edgerun-blog generate`; the service can prefer that output with
`--blog-static-root /srv/blog/.generated` while keeping live rendering as a
fallback. Generation should happen on developer machines before push, and the
generated `.generated/` tree should be part of the pushed blog revision. The
sample developer-side hook lives at `crates/edgerun-blog/hooks/pre-commit.sample`.
The content root is a host Git checkout scanned at request time; there is no
build step and no authentication.

## systemd

`edgerun-server.service` is host-only deployment material. It is intended to be
installed in `/etc/systemd/system/`, then enabled with `systemctl enable
edgerun-server.service` so systemd starts it through `multi-user.target` on
boot. Runtime crash handling is configured with `Restart=on-failure` and
`RestartSec=5`.

`edgerun-server-health.timer` is host-only deployment material. It should be
installed next to the service unit and enabled with
`systemctl enable --now edgerun-server-health.timer`; failures are reported by
the `edgerun-server-health.service` journal.
