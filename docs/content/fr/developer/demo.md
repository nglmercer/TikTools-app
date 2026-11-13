---
title: "Architecture et limites de la démo navigateur"
docId: "developer.demo"
translationKey: "developer.demo"
locale: fr
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "2e7f0ab271cce7a8c6e78d9d73c38ae0da022cefe2db104b1001a8f45cf8754c"
translationRevision: "30c8f1c472c13a9bb6eec5da73ecd22c0d6e103ba76f4ec59bebf028320f3f1c"
order: 140
---

# Architecture et limites de la démo navigateur {#browser-demo-architecture-and-limits}

## Objectif et prérequis {#goal-and-prerequisites}

Explorer l’interface réelle de l’application avec des données synthétiques dans un navigateur. Utilisez Bun 1.4.4 et l’installation verrouillée du dépôt. L’entrée explicite de démo fonctionne indépendamment de Rust, des comptes et d’un hôte actif. Son runtime et ses vérifications de validation navigateur font partie de ce dépôt.

## Compiler et examiner {#build-and-inspect}

```sh
bun scripts/task.ts docs check
bun scripts/task.ts demo dev
bun scripts/task.ts demo build
bun scripts/task.ts site build
bun scripts/task.ts site preview
```

Pour un aperçu local non commité, utilisez `TIKTOOLS_SITE_ALLOW_DIRTY=1 bun scripts/task.ts site build`. Son manifeste est marqué comme modifié et la publication le rejette. L’assemblage final exige que les deux bundles indiquent le même SHA Git propre et la même base ; recompilez après le commit. Lancez un serveur de développement ou d’aperçu uniquement lorsque le travail navigateur local est souhaité. `docs` possède VitePress et `demo` le bundle applicatif navigateur ; `site build` les assemble en un site statique. L’espace documentaire conserve sa dépendance VitePress séparément de la chaîne Vite 8 de l’application. Les commandes ne doivent ni installer silencieusement des dépendances ni invoquer Cargo.

## Préserver la frontière applicative {#preserve-the-application-boundary}

`src/web/app/mount-workspace.ts` fournit la fabrique partagée application/Pinia/routeur ; l’importer ne monte rien et ne choisit pas de transport. L’initialisation explicite de démo fournit `src/web/demo/runtime/transport.ts` à la frontière partagée `ControlTransport`/client de contrôle. Les fixtures initiales pures se trouvent dans `demo/fixtures/base-host.ts`, indépendamment de Playwright. `shared/lib/app-storage.ts` et `demo/storage.ts` fournissent des adaptateurs de stockage isolés pour que les réinitialisations ne puissent pas effacer les préférences natives ou une autre instance de démo. Les pages, widgets et fonctionnalités doivent utiliser leurs stores d’entité existants et les appels `src/web/shared/api`. Ne créez pas une seconde application imitée, n’injectez pas de fausses API natives au démarrage natif ordinaire et ne laissez pas les requêtes de démo atteindre un véritable hôte.

`demo/scenarios.ts` valide 11 identifiants de scénario et les routes/langues/visites autorisées. Les exemples ciblés acceptent aussi `embed=connection|feed|automation|points|overlays`. Chaque panneau intégré exige la route correspondante et ne peut pas lancer de visite ; les combinaisons invalides sont rejetées par la validation. L’entrée intégrée masque la navigation de l’espace de travail et la barre complète de la démo, tout en conservant les véritables composants de page, les stores et le runtime simulé. `runtime/host.ts` enregistre des gestionnaires de méthodes explicites ; la découverte liste ces gestionnaires, les identifiants de requête sont repris dans `rpc-response` et les effets non pris en charge renvoient immédiatement une erreur. L’hôte initialise des enregistrements fictifs et avance une horloge fixe d’un pas par Next event. Play et Next event connectent d’abord le créateur fictif via la RPC simulée si nécessaire, puis livrent un événement immédiat. Play continue toutes les deux secondes ; Pause arrête la production. La barre d’outils affiche l’état de lecture et le nombre d’événements. Le flux s’arrête à 300 pas et conserve au maximum 60 événements de flux et 60 exécutions. Les files de réponses et les enregistrements stockés ont des limites distinctes.

Reset efface uniquement le préfixe courant `tiktools.demo.<instance>.*` et recharge le cadre. Le rechargement réinitialise les singletons de modules, journaux, i18n et diagnostics. Les cadres intégrés reçoivent des identifiants d’instance distincts. Quitter la page démonte Vue, libère les stores, détache le client/transport et supprime les deux temporisateurs. La démo n’installe pas de pont `window.ipc` et n’effectue ni HTTP arbitraire, shell, RCON, exécution de scripts de visiteurs ou installation de mise à jour native. Sa CSP rejette les connexions réseau. Les modèles de journal et la simulation des filtres utilisent des fonctions partagées pures ; ce sont des exemples du contrat de l’interface, pas un remplacement de l’exécution Rust.

L’entrée partagée `startTour(id, {router, capabilities, taskState})` et le `tourRegistry` exporté se trouvent dans `src/web/features/guided-help/index.ts`. Sept visites versionnées utilisent des cibles de page stables et des catalogues séparés pour chaque langue prise en charge. Elles chargent Driver.js uniquement sur demande, limitent les cibles à l’espace de travail, respectent la réduction des animations, restaurent le focus et annulent les attentes bornées lors de la navigation. Fermer une visite ne termine jamais une tâche et n’exécute aucune action applicative. La sélection d’un tutoriel nomme la tâche complète, prépare le scénario correspondant et suspend la lecture avant de commencer. La commande de fermeture de Driver ferme le guide ; la barre d’outils rapporte le résultat. Reset efface la progression versionnée de cette instance.

## Résultat attendu et état {#expected-result-and-status}

La démo illustre la navigation, l’édition locale, la présentation des événements et les états de récupération pris en charge par son modèle de scénario. Elle ne valide pas de vraies sessions TikTok, une source navigateur OBS, le comportement Wry/natif, un serveur de production ou plusieurs PC. Rankings reste masqué ; Automations Changes est uniquement local, Variables avec examen est un vrai travail produit natif, Overlay Changes reste un travail ultérieur non fusionné et les conditions d’installation automatique des mises à jour sont expérimentales.

## Composants et Markdown {#components-and-markdown}

`docs/examples/*.vue` fournit à la fois le catalogue rendu et les extraits sources. Les petits boutons s’affichent directement dans Markdown sans styles globaux du bureau. Les boîtes de dialogue avec état, exemples de disposition et pages applicatives utilisent `DemoFrame`, qui construit des URL validées tenant compte de la base et ne charge un iframe que sur demande. Placez les exemples ciblés à côté des instructions qu’ils illustrent :

```vue
<DemoFrame scenario="connection" locale="fr" panel="connection" title="Essayer le panneau de connexion LIVE" />
```

La propriété `panel` sélectionne le paramètre `embed` et la route correspondante. Avant l’activation, le composant affiche un aperçu léger ; un clic charge l’interface réelle. Chaque cadre reçoit un identifiant d’instance isolé et la réinitialisation n’efface que cette instance. Le lien vers la démo complète retire le mode intégré et ouvre l’espace de travail entier. Conservez les étapes explicatives et les résultats attendus en Markdown pour que l’article reste utile avant le chargement de l’exemple. L’option publique `VITE_TIKTOOLS_PUBLIC_UI_KIT=1` expose le catalogue de composants sans activer tous les outils de développement du bureau.

## Récupération et vérification {#recovery-and-verification}

Réinitialisez un scénario après une lacune ou erreur délibérément simulée. S’il contacte un service externe, arrêtez-le et réparez la frontière de transport. Vérifiez séparément le contenu, la compilation de démo et l’assemblage du site ; une compilation documentaire ne prouve pas la correction native. Les [tests](testing.md) distinguent les preuves de faux hôte, navigateur, Rust et validation manuelle.
