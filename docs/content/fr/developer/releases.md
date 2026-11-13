---
title: "Versions, paquets et conditions de mise à jour"
docId: "developer.releases"
translationKey: "developer.releases"
locale: fr
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a606fb0f2723757e4fc3c25afb2c6b9f259bfdb5e988dd504b3f35d7d6a9f5fe"
translationRevision: "a606fb0f2723757e4fc3c25afb2c6b9f259bfdb5e988dd504b3f35d7d6a9f5fe"
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

## Examiner la confiance du système de mise à jour {#review-updater-trust}

`tiktools-app-update` utilise des métadonnées de canal signées et des octets de paquet signés. La clé publique de confiance, l’activation du flux et les ressources vérifiées publiées sont des exigences indépendantes. Les sommes SHA256 des paquets seules n’activent pas le système de mise à jour. Gardez les clés de mise à jour applicative distinctes des clés d’éditeurs de plugins.

L’hôte limite téléchargements et redirections, et le transfert d’installation doit attendre l’arrêt sûr du cœur/de la WebView. L’installation AppImage nécessite en plus `TIKTOOLS_APP_UPDATE_INSTALL_ENABLED=1` à la compilation. NSIS nécessite sa propre condition `TIKTOOLS_APP_UPDATE_NSIS_ENABLED=1`. L’existence de code et d’un test unitaire de signature ne justifie pas d’activer ces conditions dans les compilations publiées.

## Résultat attendu et état {#expected-result-and-status}

L’empaquetage portable et les outils d’installateurs de plateforme sont implémentés. Les vérifications/téléchargements dépendent du provisionnement ; le remplacement automatique et le redémarrage restent expérimentaux. Les dispositions Linux inconnues/extraites passent à l’installation manuelle. Un installateur NSIS lancé ne permet pas à l’ancien processus qui quitte d’annoncer sa réussite.

## Vérifier et récupérer {#verify-and-recover}

Exécutez `bun scripts/task.ts rust test -p tiktools-app-update --locked` pour les fixtures locales de protocole/signature. La validation réelle de mise à niveau, d’échec, de préservation des profils et de redémarrage nécessite des installations jetables prises en charge. Gardez disponible la ressource vérifiée précédente ; une publication interrompue de flux/fichier annexe doit refuser toute opération. Ne réactivez pas le transfert tant que l’arrêt des plugins reste incertain. Le [processus de publication](https://github.com/nglmercer/TikTools-app/blob/remake/.github/workflows/release.yml) et la source du système de mise à jour définissent les conditions actuelles ; les plans historiques de publication ne sont pas une liste de déploiement.
