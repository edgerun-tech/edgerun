---
title: Edgeruni e-posti benchmark: 10 000 kohalikku SMTP edastust
date: 2026-04-30
author: Ken
summary: Esimene reprodutseeritav võrdlus Edgeruni kõik-ühes e-posti serveri, Postfixi, OpenSMTPD ja Eximi vahel.
tags: [email, benchmarks, release]
---
# Edgeruni e-posti benchmark: 10 000 kohalikku SMTP edastust

Edgerun jooksutab nüüd `edgerun.tech` tootmispostkasti: SMTP, SMTPS, submission, IMAP, IMAPS, DNS, HTTPS, webmail, DKIM signing, queue handling, see blogi ja Git host töötavad ühe stripped `edgerun-server` binaari sees.

See benchmark mõõdab kitsast asja: 10 000 kohalikku SMTP sõnumit, 32 samaaegset klienti, 1 KiB keha ja üks ühendus sõnumi kohta. Kõik liiklus jääb localhost'i ja saaja on kohalik. Internetti kirju ei saadeta.

## Tulemused

<table>
<thead><tr><th>Stack</th><th>OK/Fail</th><th>Läbilase</th><th>p95 latency</th><th>Tippmälu koormuse ajal</th></tr></thead>
<tbody>
<tr><td>Edgerun</td><td>10000/0</td><td>342.71 ops/s</td><td>140.577 ms</td><td>3.58 MB RSS</td></tr>
<tr><td>Postfix</td><td>10000/0</td><td>351.69 ops/s</td><td>141.674 ms</td><td>94.02 MB container memory</td></tr>
<tr><td>OpenSMTPD</td><td>9976/24</td><td>176.01 ops/s</td><td>227.632 ms</td><td>17.91 MB container memory</td></tr>
<tr><td>Exim</td><td>2146/7854</td><td>27.41 ops/s</td><td>1663.886 ms</td><td>219.9 MB container memory</td></tr>
</tbody>
</table>

Postfix on selles testis kõige lähem võrdlus. Edgerun on kohalikul SMTP accept workload'il sisuliselt samas klassis, kuid kasutab palju vähem mälu. OpenSMTPD on väiksem ja meeldivalt seadistatav, aga selles jooksus oli 24 ebaõnnestunud operatsiooni ja kehvem tail latency.

Eximi tulemust ei tohiks lugeda Eximi lõpliku võimekuse mõõduna. See näitab pigem, et meie minimaalne rootless container config vajab häälestamist: 10 000 katsest võeti vastu ainult 2 146 ja samal ajal kasvasid nii mälu kui protsesside arv.

## Mida see tõestab

See tõestab kitsa väite: Edgeruni kohalik SMTP delivery tee on warmed 10k localhost testis juba Postfixiga konkurentsivõimeline ja kasutab oluliselt vähem mälu.

See ei tõesta veel, et Edgerun on kõigis mail-serveri workload'ides parem. Järgmisena tuleb mõõta Stalwarti ausa first-run konfiguratsiooniga, IMAP mailbox operatsioone, webmail'i, outbound STARTTLS delivery't, concurrent idle ühendusi ja vigaseid või aeglaseid kliente.

Raw evidence on source tree sees:

- `docs/benchmarks/email-stack/20260430Tscale-v2/`
- `docs/benchmarks/email-stack/20260430Tscale-v3/`
- `docs/benchmarks/email-stack/20260430Tbench-v1/`

Väide jääb meelega väikeseks: avalda mõõtmised, hoia scope ausana ja paranda serverit järgmise workload'i vastu.
