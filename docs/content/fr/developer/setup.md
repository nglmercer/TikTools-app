---
title: "Configuration et boucles de développement"
docId: "developer.setup"
translationKey: "developer.setup"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "8842472fa359a7c4b2e33bbd6e84a900b9a6dd71a8207afcf61aa507be7494d3"
translationRevision: "8842472fa359a7c4b2e33bbd6e84a900b9a6dd71a8207afcf61aa507be7494d3"
order: 10
---

# Configuration et boucles de développement {#development-setup-and-loops}

## Objectif et prérequis {#goal-and-prerequisites}

Compiler les sources actuelles de `remake` sans modifier le profil installé de l’utilisateur. Utilisez Bun 1.4.4 indiqué dans `package.json`, Rust 1.95 ou ultérieur et une version de Node compatible avec le Vite verrouillé : `^20.19.0 || >=22.12.0`. La CI utilise actuellement Rust 1.99.1. La compilation des sources est distincte de l’[installation binaire](../user/installation.md).

## Préparer la plateforme {#platform-setup}

Sous Windows, installez MSVC Build Tools, WebView2 et Strawberry Perl natif pour OpenSSL embarqué. L’enveloppe Cargo détecte l’environnement MSVC et Perl natif, tout en préservant les surcharges explicites `OPENSSL_SRC_PERL`/`PERL`. MSYS/Cygwin Perl ne convient pas à ce parcours MSVC. `lld-link` de LLVM est facultatif.

Pour développer le bureau sous Ubuntu 24.04, les dépendances CI incluent `libasound2-dev`, `libappindicator3-dev`, `libgtk-3-dev`, `libwebkit2gtk-4.1-dev`, `libxdo-dev` et `pkg-config`. L’empaquetage AppImage nécessite aussi les métadonnées de développement GStreamer, ses plugins base/good/libav et son scanner. L’outil facultatif `mold` accélère l’édition de liens Linux. Android/Termux utilise une chaîne d’outils Android native plutôt que ces bibliothèques de bureau.

## Installer les dépendances {#install-dependencies}

```sh
git switch remake
bun ci
```

Utilisez les fichiers de verrouillage Bun et Cargo du dépôt. `bun install` est réservé aux changements intentionnels de dépendances. La documentation est un espace de travail explicite : son VitePress stable a une dépendance Vite isolée tandis que l’application conserve Vite 8. N’imposez pas à tous deux la même version majeure de Vite.

## Choisir une boucle {#choose-a-loop}

```sh
bun run start                 # Vite et hôte de développement Wry possédés par ce lancement
bun run dev                   # développement natif dans le navigateur local, sans préparation des plugins de développement
bun scripts/task.ts serve     # Vite seul ; nécessite un vrai hôte/adaptateur pour les opérations natives
bun scripts/task.ts packaged  # chemin des ressources empaquetées, plutôt qu’une URL de développement
```

`start` lance Vite, attend l’URL qui lui est attribuée et exécute le bureau avec `TIKTOOLS_DEV_URL`. Il peut préparer les plugins depuis le dépôt distinct `plugins-example-tiktools`, trouvé via `TIKTOOLS_PLUGINS_REPO` ou le répertoire voisin. Il ne compile pas d’implémentations de plugins présentes dans ce dépôt. Utilisez `TIKTOOLS_SKIP_DEV_PLUGINS=1` pour omettre cette préparation. Aucun hôte n’est remplacé automatiquement ; un propriétaire de bureau existant provoque une erreur de démarrage.

Pour des expériences natives isolées, définissez `TIKTOOLS_HOME` vers un répertoire jetable avant de lancer l’hôte. Utilisez le même répertoire depuis chaque terminal CLI contrôlant cet hôte. Ne supprimez jamais un verrou de propriété actif.

## Valider et récupérer {#validate-and-recover}

```sh
bun scripts/task.ts frontend-lint
bun scripts/task.ts frontend-test
bun scripts/task.ts frontend-build
bun scripts/task.ts rust fmt --all -- --check
bun scripts/task.ts rust-lint
bun scripts/task.ts rust test --workspace --locked -- --test-threads=1
```

Exécutez des tests ciblés pendant les itérations et conservez tous les contrôles CI requis avant fusion. Ne lancez pas plusieurs compilations Cargo sur le même répertoire cible en chronométrant des tests de signature qui invoquent Cargo. Si le démarrage refuse un ancien hôte, arrêtez proprement le processus propriétaire ; si une dépendance d’éditeur de liens ou de WebView manque, réparez la configuration de plateforme avant de changer la logique applicative.
