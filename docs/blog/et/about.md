---
title: Edgerunist
summary: Miks Edgerun olemas on ja kuidas seda ehitatakse.
---
# Edgerunist

Edgerun algab lihtsast murest: tarkvara ehitamine järjest uute läbipaistmatute abstraktsioonikihtide peale muutub aina vähem teostatavaks. Masin selle all läheb küll kiiremaks, aga süsteemid, mida inimestel palume käitada, muutuvad raskemaks, hapramaks ja raskemini mõistetavaks.

Vana lause, mida tihti omistatakse Bill Gatesile, "640K ought to be enough for anybody", on tõenäoliselt apokrüüfne. Sellegipoolest on see kasulik hoiatus. See näitab, kui kiiresti tarkvara paisub ära kasutama kogu olemasolevat riistvaraeelarvet. Minu enda kogemuses kasutas Kubernetes'e platvormi seadistamine enne ühegi päris rakenduse deploy'd juba umbes 12 GB mälu. Chrome'i lähtekood on kümnete gigabaitide suurune. Android on sadades gigabaitides ja build võib lisada veel sadu. Need numbrid on nüüd tavalised, aga need ei peaks tunduma normaalsed.

Meil on maailmas tohutult arvutusressurssi. Tavaliste inimeste vajaduste jaoks on CPU-d, mälu, salvestusruumi ja võrku juba rohkem kui küll. Probleem on selles, et tänapäevane tarkvarapraktika raiskab šokeerivalt palju sellest enne, kui kasulik töö üldse algab. Iga lisakiht tahab oma control plane'i, runtime'i, image formatit, cache'i, logivoogu, paketigraafi, dashboard'i ja failure mode'i.

Edgerun on katse võtta rohkem vastutust stack'i eest tagasi: identity, networking, storage, email, sites, code publishing ja nende ümber olevad operatsioonitööriistad. Mitte sellepärast, et iga abstraktsioon oleks halb, vaid sellepärast, et abstraktsioonid peaksid olema piisavalt väikesed, et neid kontrollida, piisavalt odavad, et neid käitada, ja ausad kulude suhtes, mida nad tekitavad.

Me dogfood'ime seda nüüd päriselt. Edgerun töötab Edgeruni enda peal: production mail service, webmail surface, dashboard, build log, DNS records ja kontrollitud code explorer on teenindatud Edgeruni komponentidega, mitte eraldi hosted platformiga. See ei tähenda, et kõik planeeritud võimekused oleksid public või valmis. See tähendab, et need osad, millest siin kirjutame, kannavad juba projekti enda igapäevast liiklust.

Eesmärk on välja selgitada, kui palju saame vähendada sõltuvust suurtest, saastavatest andmekeskustest, liikudes tõhusamate peer-to-peer süsteemide ja inimeste lähedal töötava tarkvara poole. See tähendab vähem kohustuslikke platvorme, vähem jõude seisvat riistvara, vähem peidetud keerukust ja rohkem süsteeme, millest üks inimene või väike grupp päriselt aru saab.

See algas praktilise uurimisena, mitte valmis manifestina. Selles videos katsetasin selle idee jaoks veel Kubernetes'ega, enne kui hakkasin tugevamalt küsima, kui palju sellist masinavärki üldse olemas peaks olema:

https://www.youtube.com/watch?v=AIMdIAoiR80

Kõige lihtsam on minuni jõuda e-posti teel. Ainus usaldusväärne viis minuga ühendust võtta on [ken@edgerun.tech](mailto:ken@edgerun.tech).

Blogi dokumenteerib seda tööd funktsioon funktsiooni haaval, kui tükid avalikuks saavad. Code explorer näitab vastavat koodi kontrollitud osadena, nii et postitused saavad viidata päris commit'idele ja päris failidele, mitte ebamäärastele kirjeldustele.
