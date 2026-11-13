---
title: "Contribuer avec des changements ciblés"
docId: "developer.contributing"
translationKey: "developer.contributing"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "2e0522178ebee69ea5eb20fec3c0ec586f13950c51d658cb3a21843536b2f9f9"
translationRevision: "d2a85dfdb0cb39bd82e5bdce7ba0bb2669a6943d130bb10c6ba95f2496786947"
order: 230
---

# Contribuer avec des changements ciblés {#contributing-focused-changes}

## Objectif et prérequis {#goal-and-prerequisites}

Préparer une modification qu’un autre contributeur peut examiner et vérifier. Lisez le [CONTRIBUTING.md](https://github.com/nglmercer/TikTools-app/blob/remake/CONTRIBUTING.md) du dépôt, les fichiers `AGENTS.md` applicables et la [configuration](setup.md) avant de modifier. Utilisez Bun 1.4.4, Rust 1.95 ou ultérieur si nécessaire, les fichiers de verrouillage du dépôt et une branche basée sur la cible prévue.

## Définir et implémenter le changement {#define-and-implement-the-change}

1. Examinez `git status` et préservez le travail des autres. Excluez du correctif les ressources générées, identifiants et bases locales.
2. Suivez le propriétaire actuel du code : cœur Rust pour les services, modules Control API pour les contrats, client typé pour la couverture SDK et [plan des couches de l’interface](frontend.md) pour la présentation.
3. Pour changer un contrat, mettez à jour sa définition canonique et régénérez ensemble les sorties dérivées. Ne maintenez pas manuellement un autre tableau complet de schémas RPC ou d’événements.
4. Gardez les implémentations propres aux plugins dans leurs paquets externes ; le code générique de l’hôte doit découvrir les manifestes plutôt qu’importer un plugin nommé.
5. Expliquez le déclencheur concret et le comportement résultant, y compris les parcours incomplets ou expérimentaux.

## Valider de manière proportionnée {#validate-proportionately}

```sh
bun scripts/task.ts frontend-lint
bun scripts/task.ts frontend-test
bun scripts/task.ts frontend-build
bun scripts/task.ts contracts --check
```

Les changements d’implémentation native nécessitent aussi formatage, lint et tests Rust applicables ainsi que les contrôles CI requis. Une modification uniquement documentaire doit utiliser les vérifications documentaires et examiner ses commandes/liens sources plutôt que revendiquer une validation native à l’exécution. Le lanceur de tâches du projet définit les points d’entrée pris en charge ; utilisez le passage direct Cargo pour les options de cible natives spécifiques.

## Résultat attendu et état {#expected-result-and-status}

La revue contient des modifications source ciblées, des preuves de validation utiles et des limites honnêtes. La politique du dépôt vise les pull requests vers `remake` ; les étiquettes de version sont contrôlées par les mainteneurs. Les instructions de contribution sont disponibles, mais la protection des branches et la configuration des contrôles distants relèvent des administrateurs, pas de garanties fournies par une page Markdown.

## Récupérer {#recover}

Si la validation échoue à cause d’une dépendance ou bibliothèque de plateforme manquante, signalez et réparez cet environnement plutôt que d’affaiblir une vérification. Si les fichiers générés divergent, régénérez-les depuis leur source et examinez les différences. Ne publiez jamais cookies de session, clés, identifiants ou captures d’événements non expurgées. Si la source et une ancienne conception divergent, décrivez l’écart et utilisez l’implémentation actuelle pour les conseils publics. N’écrasez pas les fichiers en attente d’un autre propriétaire pour donner une apparence cohérente à un vaste nettoyage.

## Détails de fonctionnement {#operational-details}

Les sceaux sont distincts : `sourceRevision` hache le corps anglais actuel, `translationRevision` le corps localisé de cet article (sans frontmatter ni espaces extérieurs). Après revue source/traductions, lancez `bun docs/scripts/record-review.ts developer/example.md`. Toute édition ultérieure invalide le sceau correspondant. Cela détecte la dérive, pas l’identité humaine ni l’équivalence sémantique : comparez commandes, IDs RPC/environnement, clés payload, limites, activation/récupération et négations. Traduisez toute la prose en gardant les instructions techniques.

Pour une migration divisée, notez chaque sujet/titre original et sa destination concrète dans la carte, avec fragment pour une section. Le checker valide fichiers/fragments et octets historiques ; la revue éditoriale établit encore la couverture thématique. Ne modifiez pas originaux/provenance historiques pour faire réussir un contrôle.
