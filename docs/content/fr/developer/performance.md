---
title: "Performance et travail borné"
docId: "developer.performance"
translationKey: "developer.performance"
locale: fr
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "04bc0a89b86724872adae3ab8ddc8e29f5517dad14d51bc103d917582cf289ae"
translationRevision: "c967a02ac02b2dddadbeb2911dccf631f1b2d5b5dab113b661361fd4fa031c11"
order: 110
---

# Performance et travail borné {#performance-and-bounded-work}

## Objectif et prérequis {#goal-and-prerequisites}

Mesurer un goulot d’étranglement concret en préservant la comptabilisation des événements et l’arrêt. Commencez avec une compilation fixe, une entrée déterministe et un profil isolé. Lisez les [événements](events.md) pour ne pas confondre un flux plus rapide avec une persistance plus fiable. Les chiffres de benchmarks historiques sont des observations de leur compilation et machine nommées, pas des garanties produit actuelles.

## Mesurer la bonne frontière {#measure-the-right-boundary}

L’outbox du bureau sépare la présentation avec pertes des réponses fiables. Les instantanés peuvent être regroupés ; un débordement du canal fiable doit échouer visiblement. L’entrée LIVE, le travail des plugins, les appels des processeurs et l’admission d’automatisation ont chacun leurs limites. Agrandir une file peut déplacer la pression mémoire ou la latence vers l’étape suivante.

Le runtime de l’interface expose des diagnostics agrégés en développement ou lors d’une compilation avec `VITE_TIKTOOLS_DIAGNOSTICS=1`. Son instantané inclut les lignes rendues, percentiles d’intervalle entre trames, appels en attente du client de contrôle, images et temporisateurs possédés. Les données de tas JavaScript peuvent être indisponibles sur une WebView donnée. Les pages masquées et la visibilité de l’hôte gouvernent l’intérêt de présentation, pas la comptabilisation métier.

## Exécuter une comparaison ciblée {#run-a-focused-comparison}

1. Notez la révision de compilation, la plateforme, le mélange d’événements, l’activation des images, l’onglet visible et la durée.
2. Établissez une référence avec la même fixture et la même période d’échantillonnage.
3. Modifiez une frontière, puis comparez débit, latence p95/p99, abandons de files, lacunes, mémoire conservée et vidage à l’arrêt.
4. Incluez un plugin ou consommateur lent et une file saturée ; un benchmark au repos ne teste pas la contre-pression.

```sh
bun scripts/task.ts test web live-feed-state
bun scripts/task.ts rust test -p tiktools-core --features persistence,native-tiktok --test live_throughput --locked
```

Examinez les exigences de fonctionnalités avant d’activer un benchmark natif ; les vérifications unitaires ciblées ordinaires ne nécessitent pas de connexion réelle à TikTok. Utilisez `crates/tiktools-core/tests/live_throughput.rs` et les tests de l’outbox du bureau comme références d’implémentation.

## Résultat attendu et état {#expected-result-and-status}

Les parcours de projection et d’admission bornés sont implémentés, avec compteurs et signalement de lacunes. Le [rédacteur ordonné actuel](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/services/points/queue.rs) rejette les écritures saturées avant de modifier la mémoire et interdit le repli vers une écriture synchrone. Les anciens rapports de performance décrivent des défauts d’ordre désormais dépassés. Évaluez les limites du cache et la récupération face aux erreurs avec l’implémentation/les tests actuels plutôt que de déduire une correction complète d’une mémoire d’interface bornée.

## Récupérer {#recover}

Si les trames s’améliorent mais que la santé comptable se dégrade, annulez la modification de frontière et examinez les résultats d’écriture/admission. Si les temporisateurs d’onglets masqués augmentent, suivez le [nettoyage du cycle de vie](lifecycle.md). Gardez dans le rapport le nombre d’entrées brutes, le nombre accepté et la classification des pertes. Une lacune peut récupérer l’état d’affichage ; elle ne répare pas un effet rejeté ou devenu incertain.
