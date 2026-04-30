---
title: edgerun-git: kontrollitud source release samast checkout'ist
date: 2026-04-30
author: Ken
summary: Git ja crate explorer Edgeruni public code surface'i taga, ehitatud valikuliseks release'iks, mitte kogu platformi korraga avaldamiseks.
tags: [edgerun, crates, git, email]
---
# edgerun-git: kontrollitud source release samast checkout'ist

`edgerun-git` on väike Git ja crate explorer public code surface'i jaoks. See
ei ole full forge. Seal ei ole issue'sid, pull request'e, kontosid, CI lehti ega
sotsiaalseid funktsioone.

See on teadlik valik. Praegune eesmärk on controlled release: näidata tükke,
mis selgitavad production'is töötavat süsteemi, eriti email-facing platform
code'i, ilma et kogu sisemine ja eksperimentaalne ala korraga avalikuks läheks.

## Visibility Model

Koodis on praegu olemas:

- repo on peidus, kuni `.edgerun/git.yaml` ütleb `visible: true`;
- failid ja kaustad on peidus, kuni need on `.gitvisible` markeriga released;
- generated crate metadata loetakse `.edgerun/git/crates/*.txt` failidest;
- dashboard näitab released surface'i põhjal crate, API, call-edge, test ja RFC summary't;
- raw source tree ja crate explorer kasutavad sama visibility policy't.

Praegu on public slice meelega väike: blog crate, Git crate ja blog content.
Sellest piisab, et selgitada build log'i ja source explorerit ennast, ilma
seotud platformi sisemisi osi avaldamata.

## Võrdlus

`cgit`-stiilis source browsinguga võrreldes on `edgerun-git` rohkem release
boundary keskne. See ei ole lihtsalt "näita kogu repo ära". Visibility on osa
repost.

Full forge'iga võrreldes on see palju väiksem. Pole account system'i ega
project-management surface'i. Selles deployment'is oleksid need lisakihid ümber
lihtsa vajaduse: lase lugejal kontrollida koodi, millele public claim viitab.

Hosted code pages'idega võrreldes jääb source publication samasse masinasse ja
samasse operational model'isse kui email server. Edgeruni puhul on see oluline,
sest story ei ole ainult kood; story on mail, web, blog ja code surfaces koos
väikese stackina.
