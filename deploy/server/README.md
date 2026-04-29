# Edgerun Server Deployment

## Configuration Files

| File | Purpose |
|------|---------|
| `01-dns-zone.yaml` | DNS zone for `edgerun.tech` - A, MX, SPF, DMARC, CAA, MTA-STS, TLS-RPT, service host records |
| `02-dns-zone-dkim.yaml` | DKIM public key DNS record (selector: mail) |
| `03-smtp-server.yaml` | SMTP server config (implemented: port 25 MX, port 465 SMTPS submission, port 587 STARTTLS submission, relay) |
| `04-imap-server.yaml` | IMAP server config (port 143, IMAPS) |

## Required Key Material

| Name | Description |
|------|-------------|
| `/etc/edgerun/server/dkim-mail.private.pem` | DKIM RSA private key (PEM format) |
| `/etc/edgerun/server/tls/fullchain.pem` | TLS certificate chain (PEM format), generated or renewed by `edgerun-acme` |
| `/etc/edgerun/server/tls/privkey.pem` | TLS private key (PEM format), generated or renewed by `edgerun-acme` |

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
- `@ TXT v=spf1 mx -all`
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
- DNSSEC types and signing primitives exist in `edgerun-dns`, but DNSSEC is not
  yet enabled for this host. Public DNSSEC also requires stable zone key
  material and a DS record installed at the registrar; do not document the zone
  as DNSSEC-signed until those are implemented and delegated.

## Service Ports

| Service | Port | Protocol |
|---------|------|----------|
| SMTP | 25 | SMTP (with STARTTLS) |
| SMTP submission | 465 | SMTPS (implicit TLS, authenticated when users have passwords) |
| SMTP submission | 587 | SMTP with STARTTLS (authenticated when users have passwords) |
| IMAP | 143 | IMAP (with STARTTLS) |
| IMAP | 993 | IMAPS (implicit TLS) |
| DNS | 53 | UDP/TCP |
