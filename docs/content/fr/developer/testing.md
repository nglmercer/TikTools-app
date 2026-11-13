---
title: "Tests et limites des preuves"
docId: "developer.testing"
translationKey: "developer.testing"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "455bcdbf94d1bff28bd50dcb993cdff59dd000214e25c49bb194c9b3124e8e59"
translationRevision: "455bcdbf94d1bff28bd50dcb993cdff59dd000214e25c49bb194c9b3124e8e59"
order: 210
---

# Tests et limites des preuves {#tests-and-evidence-boundaries}

## Objectif et prérequis {#goal-and-prerequisites}

Choisir des vérifications capables d’établir le comportement modifié. Utilisez Bun 1.4.4, une installation conforme au fichier de verrouillage et Rust uniquement pour les frontières natives concernées. Isolez les tests des hôtes, identifiants et comptes de production. Une fixture navigateur réussie constitue une preuve utile pour sa frontière, pas pour toutes les intégrations sous-jacentes.

## Choisir une suite ciblée {#choose-a-focused-suite}

```sh
bun scripts/task.ts test
bun scripts/task.ts test web control-client
bun scripts/task.ts test bun automation
bun scripts/task.ts test rust tiktools-core batched_likes --features persistence,native-tiktok
bun scripts/task.ts rust test -p tiktools-client --test parity --locked
bun scripts/task.ts contracts --check
```

La suite rapide par défaut réunit les tests unitaires Bun et les suites Vitest web/widgets. Le lanceur ciblé valide les arguments et rejette l’absence de filtre de test Rust plutôt que d’accepter zéro correspondance. Utilisez le passage direct `rust` pour les options de cible Cargo dépassant la grammaire du lanceur ciblé.

## Vérifier la frontière réelle {#verify-the-actual-boundary}

Les tests de composants Vue utilisent le faux client de contrôle partagé. Les tests Playwright principaux exercent la même SPA avec un faux hôte `window.ipc` installé avant navigation. Les RPC inconnues échouent explicitement et les erreurs de console/page sont vérifiées. Ils utilisent Chromium, pas Wry. Les suites navigateur de l’interface des plugins se trouvent dans le dépôt externe de plugins. Les suites de passerelle des widgets testent le protocole/client partagé et les ressources compilées.

Pour un travail navigateur autorisé après compilation des ressources, utilisez `bun scripts/task.ts test e2e`. Cette commande lance un navigateur ; elle est donc volontairement séparée de l’édition documentaire et de l’examen du code sans interface. Les tests de fenêtre native, source navigateur OBS, téléphone, coffre d’identifiants système, remplacement par mise à jour et deux PC nécessitent leurs environnements réels et des preuves explicites.

## Durée d’exécution et coût CI {#execution-time-and-ci-cost}

Commencez localement avec un filtre de fichier/nom ou `bun scripts/task.ts test --changed`. Une exécution complète de l’interface isole tous les fichiers ; Vitest utilise `vmThreads` pour réutiliser l’environnement DOM par worker plutôt que partager l’état applicatif entre fichiers.

La CI compare toute la plage du push ou de la pull request. Les chemins documentaires reconnus omettent les tâches natives ; les changements Vue/démo exécutent le contrôle d’interface. Les sources natives, l’infrastructure de compilation, les manifestes de dépendances, chemins inconnus ou comparaison indisponible conservent tous les contrôles existants. `scripts/ci-scope.mjs` définit la politique de sélection testée. La CI manuelle propose `frontend`, `native` ou `full` ; les appelants réutilisables valident tout par défaut. L’état CI final rejette les tâches sélectionnées échouées ou annulées.

Les caches Rust sont séparés par rôle, système, chaîne d’outils, fichier de verrouillage et révision source. La CI conserve les informations de débogage de table de lignes et désactive les sessions incrémentales pour réduire le volume du cache. Les tests SQLite/profils sériels restent sériels jusqu’à l’isolation indépendante de leurs ressources partagées. Ne retirez pas cette protection pour obtenir un meilleur temps.

Le processus du site est manuel : modifier ou pousser la documentation ne lance pas une seconde compilation distante. La récupération des versions est également manuelle : par défaut elle vérifie uniquement Linux sans publier. Les contrôles des deux plateformes et un choix explicite de publication sont requis pour publier une version. Ne lancez et ne répétez jamais de travail sur un exécuteur payant simplement pour confirmer une modification Markdown.

## Résultat attendu et état {#expected-result-and-status}

Un rapport de modification nomme la suite réussie et son périmètre. Exécutez les vérifications de lint/types/compilation de l’interface et les contrôles CI requis pour les changements d’implémentation ; le contenu documentaire utilise `bun scripts/task.ts docs check` et `docs test`. Ne modifiez pas les dépendances simplement pour qu’une commande de test télécharge un outil plus récent.

## Récupération {#recovery}

Si un filtre ne correspond à rien, corrigez-le plutôt que d’annoncer une réussite. Si une fixture omet une RPC nouvellement requise, étendez son contrat explicite plutôt que de faire réussir les appels inconnus. Rendez visibles les validations manuelles incertaines ou ignorées. Le [lanceur de tests](https://github.com/nglmercer/TikTools-app/blob/remake/scripts/lib/test-runner.ts) définit la syntaxe actuelle ; la [performance](performance.md) explique les preuves de charge reproductibles.
