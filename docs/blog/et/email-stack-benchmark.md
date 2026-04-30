---
title: Edgeruni e-posti benchmark: serveri praegune seis
date: 2026-04-30
author: Ken
summary: Uus rootless Podmani võrdlus Edgeruni, Maddy ja Postfixi vahel pärast SMTP/IMAP runtime'i ja Maildir indeksi parandusi.
tags: [email, benchmarks, release]
---
# Edgeruni e-posti benchmark: serveri praegune seis

Edgerun jooksutab nüüd `edgerun.tech` tootmispostkasti: SMTP, SMTPS,
submission, IMAP, IMAPS, DNS, HTTPS, webmail, DKIM signing, queue handling, see
blogi ja Git host töötavad ühe stripped `edgerun-server` binaari sees.

See on tooteväide: väike integreeritud server võib väikse deployment'i puhul
asendada mitu eraldi daemonit. Benchmarki väide on kitsam: pärast runtime'i,
SMTP/IMAP puhverdamise ja Maildir indeksi tööd on Edgerun selles kohalikus
delivery testis kiirem kui mõõdetud Maddy ja Postfix.

## Mis muutus

Koodis on nüüd hosti multi-thread runtime, puhverdatud SMTP/IMAP I/O,
recipient-batch kohalik delivery, Maildir status sidecar IMAP `SELECT` jaoks
ja umbes 500 ms järel flushitavad SMTP delivery status-index uuendused.
Per-message Maildir `sync_all()` on nüüd strict-mode valik
`EDGERUN_MAILDIR_SYNC_DELIVERY=1` kaudu.

## Tulemused

Värskeim evidence on viie kordusega rootless Podmani jooks:

`docs/benchmarks/email-stack/20260430Tedgerun-postfix-maddy-batched-r5/`

Oluline piirang: kõik viis kordust jooksid sama stacki sees samas elavas
containeris, seega mailbox/spool state kasvas korduste vahel. See on sustained
growing-state tulemus, mitte viis sõltumatut cold start'i.

Mediaanläbilase viie korduse pealt:

| Stack | SMTP mediaan | IMAP mediaan |
| --- | ---: | ---: |
| Edgerun | 527.03 ops/s | 216.92 ops/s |
| Maddy | 255.45 ops/s | 81.10 ops/s |
| Postfix | 126.10 ops/s | n/a |

Esimene kordus enne suuremat state'i kogunemist:

| Stack | SMTP läbilase | SMTP p95 | Tippmälu / PID-id |
| --- | ---: | ---: | --- |
| Edgerun | 557.72 ops/s | 64.173 ms | 113.20 MB / 10 |
| Maddy | 344.62 ops/s | 123.828 ms | 148.70 MB / 41 |
| Postfix | 347.70 ops/s | 110.324 ms | 134.20 MB / 75 |

## Footprint

| Stack | Image size | Build time | Startup time | Process count |
| --- | ---: | ---: | ---: | ---: |
| Edgerun | 4,812,313 bytes | 797 ms | 2,199 ms | 10 |
| Maddy | 58,114,573 bytes | 1,310 ms | 2,230 ms | 41-45 |
| Postfix | 129,522,342 bytes | 14,198 ms | 2,186 ms | 75-76 |

Mäluväide peab jääma ettevaatlikuks. Edgeruni SMTP tippmälu kasvas viie korduse
jooksul 113.20 MB pealt 202.40 MB peale. Maddy kasvas 148.70 MB pealt
249.20 MB peale ja Postfix 134.20 MB pealt 209.00 MB peale. Praegune evidence
toetab tugevat image/process-footprint väidet, aga mitte üldist "alati kõige
väiksem memory" väidet.

## Mida see tõestab

Kitsas publishitav väide:

> Edgerun on kompaktne integreeritud mail stack, mida tasub kaaluda, kui operaator tahab SMTP, IMAP, web'i, DNS'i, DKIM'i, queue handling'ut ja site hosting'ut ühes väikeses teenuses. Värskeimas rootless Podmani kohaliku delivery benchmarkis oli Edgerun SMTP-s kiirem kui Maddy ja Postfix, IMAP-i basic smoke'is kiirem kui Maddy ning image/process footprint oli palju väiksem.

Veel ei tohiks väita, et Edgerun võidab kõiki mail-serveri workload'e,
realistlikku Dovecot IMAP-i, Moxi või Stalwarti production konfiguratsiooni.
Exim on aktiivsest võrdlusest välja jäetud, sest meil ei ole praegu ausat
tuned harness'i tulemust.

Raw evidence:

- `docs/benchmarks/email-stack/20260430Tedgerun-postfix-maddy-batched-r5/`
- `docs/benchmarks/email-stack/20260430Tedgerun-postfix-maddy-batched/`
- `docs/benchmarks/email-stack/20260430Tactive-smoke/`
- `docs/benchmarks/email-stack/research-status-20260430.md`
