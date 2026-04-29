# Mail Server Deployment

## Configuration Files

| File | Purpose |
|------|---------|
| `01-dns-zone.yaml` | DNS zone for `edgerun.tech` - A, MX, SPF, DMARC records |
| `02-dns-zone-dkim.yaml` | DKIM public key DNS record (selector: mail) |
| `03-smtp-server.yaml` | SMTP server config (implemented: port 25 MX, port 465 SMTPS submission, port 587 STARTTLS submission, relay) |
| `04-imap-server.yaml` | IMAP server config (port 143, IMAPS) |

## Required Key Material

| Name | Description |
|------|-------------|
| `/etc/edgerun/mail/dkim-mail.private.pem` | DKIM RSA private key (PEM format) |
| `/etc/edgerun/mail/tls/fullchain.pem` | TLS certificate chain (PEM format), generated or renewed by `edgerun-acme` |
| `/etc/edgerun/mail/tls/privkey.pem` | TLS private key (PEM format), generated or renewed by `edgerun-acme` |

`edgerun-mail-server` consumes these paths through `edgerun-config` YAML fields
(`dkim_key_path`, `tls_cert`, and `tls_key`). The DKIM TXT record in
`02-dns-zone-dkim.yaml` must contain the public key produced from the private
key at `/etc/edgerun/mail/dkim-mail.private.pem`.

## DNS Records Generated

- `@ A 172.245.67.49`
- `mail A 172.245.67.49`
- `ns1 A 172.245.67.49`
- `ns2 A 172.245.67.49`
- `@ MX 0 mail.edgerun.tech`
- `@ TXT v=spf1 mx -all`
- `_dmarc TXT v=DMARC1; p=quarantine; rua=mailto:dmarc-reports@edgerun.tech`
- `mail._domainkey TXT v=DKIM1; k=rsa; p=<public-key>`

## Service Ports

| Service | Port | Protocol |
|---------|------|----------|
| SMTP | 25 | SMTP (with STARTTLS) |
| SMTP submission | 465 | SMTPS (implicit TLS, authenticated when users have passwords) |
| SMTP submission | 587 | SMTP with STARTTLS (authenticated when users have passwords) |
| IMAP | 143 | IMAP (with STARTTLS) |
| IMAP | 993 | IMAPS (implicit TLS) |
| DNS | 53 | UDP/TCP |
