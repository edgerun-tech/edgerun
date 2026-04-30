---
title: edgerun-blog: avaldamine ilma eraldi CMS-ita
date: 2026-04-30
author: Ken
summary: Väike first-party blog crate Edgeruni ehituslogi taga ja miks see elab sama serveri sees kui mail ja kood.
tags: [edgerun, crates, blog, email]
---
# edgerun-blog: avaldamine ilma eraldi CMS-ita

See ehituslogi ei kasuta hosted CMS-i, andmebaasi ega eraldi static-site SaaS-i.
Seda teenindab `edgerun-blog`, väike host-only crate, mis loeb Markdowni samast
Git checkout'ist kui kood.

See on üks platformi tükkidest, mis toetab praegust Edgeruni e-posti
deployment'i. Scope on meelega väike: avalda märkmeid, renderda postitusi,
paku feed'i ja lase dashboard'il sama sisu workspace surface'ina näidata.

## Mida See Teeb

Koodis on praegu olemas:

- Markdowni ja HTML postituste lugemine `docs/blog` kaustast;
- nõue, et public postitustel oleks English, Thai ja Estonian versioon;
- front matter kontroll title, date, author, summary ja tags väljadele;
- post pages, index pages, feed, search metadata, sitemap, robots ja app shell;
- otse checkout'ist serve'imine või deterministic static output'i genereerimine;
- nähtavate crate nimede linkimine public code explorerisse.

Eesmärk ei ole asendada iga küpse CMS-i funktsiooni. Eesmärk on, et production
mail host saaks avaldada release note'e ilma andmebaasi, plugin runtime'i,
admin paneeli või välise analytics scriptita.

## Võrdlus

Üldise static-site generatoriga võrreldes on `edgerun-blog` kitsam. See ei püüa
olla theme ecosystem. Layout, feed, metadata, language rules ja dashboard
fragments on ehitatud ühe avaldamise workflow jaoks.

Database-backed CMS-iga võrreldes on runtime state'i vähem. Source of truth on
Git checkout. Review, rollback ja deployment liiguvad sama rada pidi kui kood.

Käsitsi HTML-iga võrreldes hoiab see korduva safety töö koodis: front matter
checks, escaping, feeds, sitemap entries, canonical URLs ja language routes.

See sobib praeguse release strateegiaga: avaldada piisavalt, et projekt oleks
arusaadav, ilma et peaks avama suure operatsioonilise surface'i ainult mõne
lehe hostimiseks.
