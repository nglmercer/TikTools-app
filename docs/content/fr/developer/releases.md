---
title: "Versions, paquets et conditions de mise à jour"
docId: "developer.releases"
translationKey: "developer.releases"
locale: fr
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a0c1a45f5279fec0b2fae29f40f47090d0600237815b293e9600e28221ec969b"
translationRevision: "b9fa808f309db16f85065938bab674cfe15e9b6a5bc1ab88b612e746e509b9e4"
order: 220
---

# Versions, paquets et conditions de mise à jour {#releases-packages-and-updater-gates}

## Objectif et prérequis {#goal-and-prerequisites}

Préparer une ressource de version pouvant être examinée et distinguer les téléchargements vérifiés de l’installation automatique. Ce parcours de maintenance nécessite les chaînes d’outils appropriées aux plateformes et du matériel de signature protégé. Les contributeurs ordinaires peuvent examiner l’empaquetage et les contrôles de version sans publier ni créer d’étiquette de version.

## Préparer des ressources cohérentes {#prepare-consistent-artifacts}

`package.json`, le `Cargo.toml` de l’espace de travail et `Cargo.lock` partagent la version applicative. Utilisez le lanceur de tâches plutôt que de modifier une seule version indépendamment :

```sh
bun scripts/task.ts version check
bun scripts/task.ts frontend-build
bun scripts/task.ts rust build -p tiktools-desktop --release --locked
```

Un changement de version intentionnel du mainteneur utilise `bun scripts/task.ts version bump v4.2.3` avec la prochaine version sélectionnée. La compilation complète crée aussi les ressources des widgets. L’empaquetage de publication place l’exécutable natif à côté de `web/index.html` et publie des sommes de contrôle. Les processus d’installateurs construisent en plus NSIS, DEB et AppImage et vérifient leurs ressources contre la préparation portable. Les paquets de plugins restent des installations distinctes à l’exécution.


La documentation publique affiche les cartes de version à partir de `docs/experience/releases.json`. Actualisez explicitement cet instantané après une publication :

```sh
bun scripts/refresh-docs-releases.ts
```

Cette commande lit les versions publiées via l’API GitHub, conserve les fichiers effectivement téléversés et enregistre la date de vérification affichée sur les cartes. Les compilations utilisent l’instantané versionné hors ligne ; elles ne récupèrent pas les données de version sur GitHub. Examinez les modifications de l’instantané, notamment les noms de fichiers par plateforme, les canaux et les URL de téléchargement, avant de le commiter. Une étiquette de code source seule ne doit pas créer de lien d’installateur, et l’instantané n’active ni ne valide l’installation par le système de mise à jour.

## Examiner la confiance du système de mise à jour {#review-updater-trust}

`tiktools-app-update` utilise des métadonnées de canal signées et des octets de paquet signés. La clé publique de confiance, l’activation du flux et les ressources vérifiées publiées sont des exigences indépendantes. Les sommes SHA256 des paquets seules n’activent pas le système de mise à jour. Gardez les clés de mise à jour applicative distinctes des clés d’éditeurs de plugins.

L’hôte limite téléchargements et redirections, et le transfert d’installation doit attendre l’arrêt sûr du cœur/de la WebView. L’installation AppImage nécessite en plus `TIKTOOLS_APP_UPDATE_INSTALL_ENABLED=1` à la compilation. NSIS nécessite sa propre condition `TIKTOOLS_APP_UPDATE_NSIS_ENABLED=1`. L’existence de code et d’un test unitaire de signature ne justifie pas d’activer ces conditions dans les compilations publiées.

## Résultat attendu et état {#expected-result-and-status}

L’empaquetage portable et les outils d’installateurs de plateforme sont implémentés. Les vérifications/téléchargements dépendent du provisionnement ; le remplacement automatique et le redémarrage restent expérimentaux. Les dispositions Linux inconnues/extraites passent à l’installation manuelle. Un installateur NSIS lancé ne permet pas à l’ancien processus qui quitte d’annoncer sa réussite.

## Vérifier et récupérer {#verify-and-recover}

Exécutez `bun scripts/task.ts rust test -p tiktools-app-update --locked` pour les fixtures locales de protocole/signature. La validation réelle de mise à niveau, d’échec, de préservation des profils et de redémarrage nécessite des installations jetables prises en charge. Gardez disponible la ressource vérifiée précédente ; une publication interrompue de flux/fichier annexe doit refuser toute opération. Ne réactivez pas le transfert tant que l’arrêt des plugins reste incertain. Le [processus de publication](https://github.com/nglmercer/TikTools-app/blob/remake/.github/workflows/release.yml) et la source du système de mise à jour définissent les conditions actuelles ; les plans historiques de publication ne sont pas une liste de déploiement.

## Détails de fonctionnement {#operational-details}

Le paquet portable local exige mêmes versions source/manifestes/binaire, widgets/web construits et exécutable release. L’exemple utilise la version actuelle ; changez le tag seulement lors d’un changement intentionnel. `package` valide binaire/layout et écrit `release/TikTools-<version>-<platform>.tar.gz` (Linux) ou `.zip` (Windows). `RELEASE_BINARY` choisit un autre binaire. Sous Windows utilisez les affectations PowerShell et `windows-x86_64`. Les installateurs exigent dépendances plateforme et cargo-packager épinglé du workflow, puis `bun scripts/package-installers.ts` avec mêmes tag/plateforme. Portable ne produit pas NSIS/DEB/AppImage.

Conjonction compilée : checks exigent `TIKTOOLS_APP_UPDATE_FEED_ENABLED=1` ET `TIKTOOLS_APP_UPDATE_PUBLIC_KEY` valide ; AppImage ajoute `TIKTOOLS_APP_UPDATE_INSTALL_ENABLED=1` ET une source reconnue ; NSIS ajoute encore `TIKTOOLS_APP_UPDATE_NSIS_ENABLED=1`. Valeurs de compilation. Le workflow transforme `TIKTOOLS_UPDATE_PUBLIC_KEY` du dépôt en clé/feed. Sans elle, assets/checksums peuvent être publiés sans feed signé. Avec elle : clé/mot de passe protégés, signature du manifeste exact/paquet, assets publiés vérifiés et promotion authentifiée. Signature updater, signature OS et clé éditeur plugin sont distinctes.

Les tests ordinaires omettent l’acceptation réelle AppImage. Les deux tests ignorés exigent une vraie image jetable annonçant 4.0.0 (assertion actuelle), FUSE et signer cargo-packager ; le remplacement exige un écran. Définissez `TIKTOOLS_TEST_APPIMAGE`, listez puis exécutez explicitement `preflight_runs_a_real_appimage` et `packager_replaces_a_running_disposable_appimage_without_touching_its_profile` avec `-- --ignored --nocapture`. Un arbre extrait ne remplace pas le montage. Observez remplacement, version, redémarrage et profil. NSIS exige une acceptation après sortie ; l’ancien processus ne certifie pas le succès.

```sh
bun scripts/task.ts widgets
bun scripts/task.ts frontend-build
bun scripts/task.ts rust build -p tiktools-desktop --release --locked
RELEASE_TAG=v4.2.2 RELEASE_PLATFORM=linux-x86_64 bun scripts/task.ts package
```

```powershell
$env:RELEASE_TAG = 'v4.2.2'
$env:RELEASE_PLATFORM = 'windows-x86_64'
bun scripts/task.ts package
```

```sh
TIKTOOLS_TEST_APPIMAGE=/absolute/path/disposable-v4.0.0.AppImage bun scripts/task.ts rust test -p tiktools-app-update --locked preflight_runs_a_real_appimage -- --ignored --nocapture
```
