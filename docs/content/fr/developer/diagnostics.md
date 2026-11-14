---
title: "Diagnostics et preuves sans secrets"
docId: "developer.diagnostics"
translationKey: "developer.diagnostics"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "bd492eda2007394d29d152e20ec0dba70614f06e6d29b75a3b741ed6b23f11b4"
translationRevision: "0dcfd0c5c342e99745938bddfc38d8a97f725aee260d9d7fb91181e96b905453"
order: 130
---

# Diagnostics et preuves sans secrets {#diagnostics-and-safe-evidence}

## Objectif et prérequis {#goal-and-prerequisites}

Recueillir assez d’éléments pour localiser un échec sans conserver de secrets de compte ni confondre diagnostics et récupération. Utilisez un hôte de test privé et connaissez le [transport](transports.md) sélectionné. L’application installe un collecteur par session ; les composants d’état y naviguent plutôt que d’ajouter des entrées pendant le rendu.

## Examiner un échec {#inspect-a-failure}

1. Ouvrez l’action d’état contextuelle de la connexion, du plugin, de la synchronisation ou du système de mise à jour en échec. Le focus clavier révèle l’action de journal de bureau comme le survol.
2. Filtrez par source, gravité et texte, puis examinez le contexte de l’entrée sélectionnée et son nombre de répétitions.
3. Copiez les entrées sélectionnées ou filtrées lorsqu’elles expliquent le problème. Examinez les exportations avant de les partager.
4. Comparez la santé native à l’observation de l’interface :

```sh
target/debug/tiktools --json system info
target/debug/tiktools --json system health
target/debug/tiktools --json system doctor
```

Une sonde LIVE déconnectée peut être ignorée. Une capacité compilée ne prouve pas sa disponibilité à l’exécution. Le panneau d’interface est non modal sur bureau ; les dispositions étroites utilisent `/debug` avec un emplacement de retour. L’exportation texte native utilise `system.saveFile` ; les navigateurs téléchargent localement.

## Comprendre la rétention {#understand-retention}

`src/web/shared/diagnostics/log.ts` limite l’historique à 2 000 entrées et 2 Mio d’UTF-8 sérialisé. Les événements consécutifs de même source/entité dans un intervalle de cinq secondes sont regroupés avec un compteur de répétitions. L’expurgation a lieu avant conservation, notamment pour les identifiants reconnus et emplacements de secrets dans les URL ; les secrets tiers arbitraires non étiquetés ne peuvent pas toujours être reconnus. N’ajoutez pas de paramètres bruts, cookies, paramètres RPC ou corps HTTP aux collecteurs.

## Résultat attendu et état {#expected-result-and-status}

La visionneuse fournit un historique de session borné et une navigation contextuelle, pas un journal d’audit durable. Le rechargement/démontage efface la mémoire. Effacer l’historique ne supprime pas une erreur de connexion et ne répare pas sa cause. Les identifiants de requêtes Cloud peuvent aider à corréler une enveloppe d’échec sûre ; la couverture des sources suit les collecteurs installés et ne garantit pas la capture de toutes les erreurs du registre ou natives.

## Vérifier et récupérer {#verify-and-recover}

Exécutez `bun scripts/task.ts test web diagnostics` et examinez la [source du collecteur](https://github.com/nglmercer/TikTools-app/tree/remake/src/web/shared/diagnostics). Reproduisez l’expurgation, l’éviction et les répétitions avec des valeurs synthétiques. Pour un conflit, une commande inconnue ou une expiration d’identifiants, revenez au parcours de récupération propriétaire. Copier une entrée diagnostique recueille des preuves ; cela ne réessaie jamais une action et n’approuve pas une nouvelle politique de confiance.
