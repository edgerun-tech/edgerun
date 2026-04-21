# Mail Server Deployment

## Configuration Files

| File | Purpose |
|------|---------|
| `01-dns-zone.yaml` | DNS zone for `edgerun.tech` - A, MX, SPF, DMARC records |
| `02-dns-zone-dkim.yaml` | DKIM public key DNS record (selector: mail) |
| `03-smtp-server.yaml` | SMTP server config (port 25, STARTTLS, relay) |
| `04-imap-server.yaml` | IMAP server config (port 143, IMAPS) |

## Secrets Integration with vals

This configuration uses vals `ref+` expressions. Before deployment, run:

```bash
# Replace refs with actual secrets from storage
vals eval -f deploy/mail/01-dns-zone.yaml
vals eval -f deploy/mail/02-dns-zone-dkim.yaml
vals eval -f deploy/mail/03-smtp-server.yaml
vals eval -f deploy/mail/04-imap-server.yaml
```

### Required Secrets in edgerun-storage

Store these credentials in edgerun-storage (namespace: `mail`):

| Name | Description |
|------|-------------|
| `dkim-private-key` | DKIM RSA private key (PEM format) |
| `tls-cert` | TLS certificate (PEM format) |
| `tls-key` | TLS private key (PEM format) |

### Required Environment Variables for vals

Set these before running vals:

```bash
# For storage backend (if using)
export STORAGE_BACKEND=...
export STORAGE_ENDPOINT=...

# For any other secrets
export DKIM_PUBLIC_KEY="v=DKIM1; k=rsa; p=..."
```

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
| SMTP | 465 | SMTPS (implicit TLS) |
| IMAP | 143 | IMAP (with STARTTLS) |
| IMAP | 993 | IMAPS (implicit TLS) |
| DNS | 53 | UDP/TCP |