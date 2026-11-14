---
title: "Internationalisation et accessibilité"
docId: "developer.i18n-accessibility"
translationKey: "developer.i18n-accessibility"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "2c74fcefd1d28353b1135c547b2e3e6ae852af8b683350011732d10c364f9323"
translationRevision: "41485a8df0080d774338fa0108074bc4db296a320e297d992e15302286b66542"
order: 120
---

# Internationalisation et accessibilité {#internationalization-and-accessibility}

## Objectif et prérequis {#goal-and-prerequisites}

Ajouter un écran qui reste compréhensible dans les langues prises en charge et utilisable au clavier, au toucher et avec les technologies d’assistance. Lisez les [couches de l’interface](frontend.md) et utilisez les composants partagés existants. Les langues de l’application à l’exécution et les éditions publiées de la documentation sont des systèmes distincts.

## Ajouter du contenu localisé {#add-localized-content}

Le catalogue de l’application se trouve dans `src/web/shared/i18n/`. L’anglais est la langue de repli typée ; les autres catalogues pris en charge sont chargés à la demande. `activateLocale` charge les messages choisis et met à jour `document.documentElement.lang`. Enregistrez la préférence de langue via le store de langue existant plutôt qu’une autre clé de stockage local.

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

## Détails de fonctionnement {#operational-details}

Détection : `tiktok-live-locale` sauvegardé → `navigator.languages` → anglais. `normalizeLocale` gère régions et `jp` → `ja`. Codes/régions : `en/en-US`, `es/es-ES`, `pt/pt-BR`, `fr/fr-FR`, `zh/zh-CN`, `ja/ja-JP` ; utilisez formatCount/formatDate d’`intl.ts`. `activateLocale` charge messages/lang ; `useLocaleStore().setLocale` persiste aussi et prépare les préférences au mieux. Utilisez le store dans les réglages. Anglais en fallback ; zh/ja chargent les messages hôte. Nouveau langage : supported-locales, loaders, catalogues/hôte, régions, tests préférences/clés/placeholders/pluriels/compilation et routes docs/site. Préservez IDs/API et paramètres.

Utilisez tokens espace/type/contrôle/focus/couches. Tout pointeur grossier ou largeur sous 700px porte les contrôles à 44px ; navigation sous 900px et lignes sous 600px sont indépendantes. Mouvement réduit annule les durées. Testez le tactile aussi sur écran large. L’app est sombre globalement ; la variante claire est réservée au JSON.

Clavier : onglets flèches/Home/End avec sélection automatique ; selects flèches/Home/End/Entrée et Échap annule ; menus rendent le focus. Dialogues en showModal natif, mais `closable=false` bloque Échap et fermer démonte le contenu. Gardez les brouillons chez le propriétaire et vérifiez focus initial/retour. Bouton icône exige `label` ; `hint`/tooltip explique sans nommer. Tooltip attend 500ms au survol/focus, ferme sur Échap et respecte le top layer. Vérifiez les flux clavier/assistance réels ; les unit tests ne certifient pas un écran entier.
