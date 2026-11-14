---
title: "Périmètre de Termux et du mode sans interface graphique"
docId: "user.start.termux"
translationKey: "user.start.termux"
locale: fr
audience: "user"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "40a55076e0cff0230664728c6091cbec0b41649f9e69f50da50c741de337c2cb"
translationRevision: "7ffe76c431ed3a7ece216030df9bfc7efbbe6fa4f4572d3f8ec457026a4b4130"
order: 50
---

# Périmètre de Termux et du mode sans interface graphique {#termux-and-headless-scope}

## Objectif et prérequis {#goal-and-prerequisites}

Comprendre quelles parties de TikTools fonctionnent sans WebView de bureau. Cet usage avancé n’est pas une version Android de l’installateur Windows/Linux. Le projet dispose d’une CLI Rust native pour Termux et d’un hôte sans interface graphique ; l’obtention d’un binaire compatible ou sa compilation relève de la configuration de développement.

## Utiliser un seul hôte {#use-one-host}

1. Démarrez un hôte sans interface graphique avec `tiktools host --ipc` dans l’environnement où la CLI a été installée.
2. Utilisez le même répertoire d’application dans tous les terminaux.
3. Depuis un autre terminal, utilisez `tiktools system info`, `tiktools live status` ou `tiktools api discover` pour examiner cet hôte.
4. Arrêtez-le proprement avec la commande d’arrêt documentée ou Ctrl-C. Ne supprimez pas les verrous de propriété pendant son exécution.

Les commandes CLI ordinaires se connectent à l’hôte existant. `--standalone` crée explicitement un cœur distinct pour une seule invocation ; ce n’est pas le mode de contrôle par défaut d’un hôte déjà démarré.

## Aperçu dans un navigateur local {#local-browser-preview}

L’adaptateur facultatif `local-web` peut servir l’interface Vue à un navigateur après un appairage local. Il s’agit d’un aperçu de développement avec ses propres exigences de compilation et de cycle de vie. Il ne fait pas d’Android une cible WebView de bureau prise en charge et n’accorde pas à une personne distante l’accès à l’hôte.

## Résultat attendu {#expected-result}

La CLI et le bureau partagent les concepts de contrôle, la persistance SQLite et le traitement natif des événements. Les plugins dépendant de la plateforme, l’audio, les raccourcis globaux, les fenêtres natives et les coffres d’identifiants peuvent être indisponibles. La persistance native des profils distants comporte actuellement des restrictions explicites sur Android.

## Récupération {#recovery}

Utilisez le répertoire privé de l’application Termux plutôt que le stockage partagé Android, qui peut empêcher l’exécution et les sockets Unix. Gardez le même répertoire d’application et vérifiez que l’hôte est actif avant de déboguer un échec IPC. Si Android arrête le processus, redémarrez-le avec le même profil et examinez les diagnostics de récupération. La démo de documentation ne reproduit pas le cycle de vie Android.

Le [guide de développement](../developer/index.md) contient la chaîne d’outils, les indicateurs de fonctionnalités et les procédures de vérification sur appareil. L’installation des binaires de bureau reste décrite dans le [guide Windows/Linux](installation.md).

## Détails de fonctionnement {#operational-details}

Pour le navigateur local, construisez les assets avec index.html et la CLI avec `local-web`. Retirez les variables héritées et utilisez `TIKTOOLS_HOME` jetable ; démarrez d’abord l’hôte IPC. Dans un autre terminal avec la même racine, lancez l’adaptateur, ouvrez le lien loopback ou saisissez le code de six chiffres à usage unique (cinq minutes). Arrêtez séparément adaptateur et hôte. Loopback/pairing ne fournissent pas d’accès distant/LAN. Termux exige toolchain Android natif et assets web ; un binaire Linux desktop n’est pas Android.

```sh
bun scripts/task.ts frontend-build
bun scripts/task.ts rust build -p tiktools-cli --features local-web --locked
export TIKTOOLS_HOME=/absolute/path/disposable-home
target/debug/tiktools host --ipc
```

```sh
export TIKTOOLS_HOME=/absolute/path/disposable-home
target/debug/tiktools web --assets /absolute/path/to/TikTools/dist/web --port 4174
```
