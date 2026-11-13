---
title: "Internationalisation et accessibilité"
docId: "developer.i18n-accessibility"
translationKey: "developer.i18n-accessibility"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "db87c2ee0db326f993350cc6ccb2511e7b082779195fc57df49b3c80436b5721"
translationRevision: "db87c2ee0db326f993350cc6ccb2511e7b082779195fc57df49b3c80436b5721"
order: 120
---

# Internationalisation et accessibilité {#internationalization-and-accessibility}

## Objectif et prérequis {#goal-and-prerequisites}

Ajouter un écran qui reste compréhensible dans les langues prises en charge et utilisable au clavier, au toucher et avec les technologies d’assistance. Lisez les [couches de l’interface](frontend.md) et utilisez les composants partagés existants. Les langues de l’application à l’exécution et les éditions anglaise, espagnole, portugaise et française de la documentation sont des systèmes distincts.

## Ajouter du contenu localisé {#add-localized-content}

Le catalogue de l’application se trouve dans `src/web/shared/i18n/`. L’anglais est la langue de repli typée ; l’espagnol, le portugais et le français sont chargés à la demande. `activateLocale` charge les messages choisis et met à jour `document.documentElement.lang`. Enregistrez la préférence de langue via le store de langue existant plutôt qu’une autre clé de stockage local.

1. Ajoutez la clé sémantique du message au catalogue anglais et aux catalogues/modules équivalents pris en charge.
2. Gardez stables les identifiants d’API, chemins d’événements et identifiants de ressources stockés ; traduisez leurs libellés visibles.
3. Utilisez l’interpolation pour le texte variable et les utilitaires d’internationalisation partagés pour les valeurs affichées.
4. Pour les descripteurs de plugins, utilisez le contrat de texte localisé existant `{default,i18key}` et les catalogues fusionnés par l’hôte. Ne rendez jamais en HTML des chaînes traduites ou fournies par un plugin.

## Préserver la sémantique des commandes {#preserve-control-semantics}

Préférez `BaseTextField`, `BaseFormRow`, `BaseDialog`, les onglets et les composants d’état. Un texte indicatif ne remplace pas un libellé accessible. Les boutons d’icône nécessitent un nom localisé. La validation doit identifier le champ et exposer son état invalide ; ne communiquez pas une erreur uniquement par la couleur. Préservez le retour du focus des boîtes de dialogue, le comportement d’Échap, la sélection clavier et les contours de focus visibles.

```sh
bun scripts/task.ts test web i18n
bun scripts/task.ts test web catalogs
bun scripts/task.ts test web BaseDialog
bun scripts/task.ts frontend-lint
```

Pour l’examen manuel, parcourez tout le parcours avec Tab, ouvrez/fermez la boîte de dialogue, changez de langue pendant son ouverture et répétez à faible largeur. Les actions d’état apparaissant au survol doivent aussi apparaître au focus clavier et rester disponibles au toucher.

## Résultat attendu et état {#expected-result-and-status}

Le typage des catalogues et les contrôles de parité détectent les clés manquantes ; les composants partagés fournissent des comportements clavier et ARIA implémentés. Ils ne constituent pas une certification globale d’accessibilité de tous les écrans composés. Rankings reste masqué même si des chaînes traduites existent.

## Récupération {#recovery}

Si du texte non traduit apparaît, vérifiez le chargement différé des catalogues et l’orthographe exacte des clés avant d’ajouter des replis anglais dans les composants. Si le focus disparaît, examinez la commande recevant les attributs transférés et la cible de retour de la boîte de dialogue. L’[implémentation i18n](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/i18n/i18n.ts) et les tests de composants définissent le comportement actuel ; les [composants](components/index.md) expliquent les points d’entrée partagés.
