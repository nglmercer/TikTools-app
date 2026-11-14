---
title: "Installer sous Windows ou Linux"
docId: "user.start.installation"
translationKey: "user.start.installation"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "ed9e7c1bcb88d991ed3d8ea8a399aa5b5e9ca8cfa58de580f8ca757344d26ca2"
translationRevision: "6f06a7138258ead608eb65fbb4a937e944217da9a3f4f20804ac0236825ab5f1"
order: 20
---

# Installer sous Windows ou Linux {#install-windows-or-linux}

## Objectif et prérequis {#goal-and-prerequisites}

Installez un paquet de bureau publié sans configurer de compilateur. Téléchargez-le depuis les [versions GitHub](https://github.com/nglmercer/TikTools-app/releases) du projet, puis suivez les instructions correspondant à votre système d’exploitation.

<DocRequirements title="Avant de commencer">

- Vérifiez les [prérequis système](requirements.md) et choisissez un paquet compatible avec votre ordinateur.
- Fermez toute instance de TikTools en cours avant de remplacer l’application.
- Lisez les notes de version et conservez son fichier `SHA256SUMS.txt` pour vérifier le téléchargement.
- Avant d’essayer une autre version, sauvegardez le répertoire de données avec l’application arrêtée.

</DocRequirements>

## Choisir un paquet {#choose-a-package}

<DocTabs label="Choisissez votre système">

<template #windows>

Choisissez **`installer.exe`** pour x86_64 afin d’effectuer une installation NSIS par utilisateur. L’application de bureau nécessite **WebView2** ; installez-le si l’installateur le demande.

Utilisez le paquet **ZIP** pour obtenir un répertoire portable. Extrayez toute l’archive et conservez l’exécutable, `web/` et toutes les autres ressources du paquet ensemble.

[Suivre les instructions Windows](#windows)

</template>

<template #linux>

Choisissez **`installer.AppImage`** pour une image exécutable portable sur un bureau Linux compatible, **`installer.deb`** pour une installation avec le gestionnaire de paquets Debian/Ubuntu, ou **TAR.GZ** pour un répertoire portable contenant l’exécutable et ses ressources.

L’AppImage nécessite une autorisation d’exécution. Le DEB utilise votre gestionnaire de paquets pour résoudre les dépendances d’exécution. Extrayez tout le répertoire TAR.GZ et conservez toutes les ressources du paquet ensemble.

[Suivre les instructions Linux](#linux)

</template>

</DocTabs>

Vérifiez si la version est stable ou s’il s’agit d’une préversion. Les préversions servent au débogage et ne garantissent pas automatiquement la compatibilité en production. Utilisez le fichier `SHA256SUMS.txt` de la version pour vérifier les octets téléchargés avant de remplacer une installation existante.

## Windows {#windows}

<DocSteps>

1. Téléchargez l’installateur x86_64 et vérifiez sa somme de contrôle.
2. Exécutez l’installateur avec le compte utilisateur qui utilisera TikTools. Installez WebView2 si cela est demandé.
3. Lancez TikTools. Ouvrez **Paramètres → Général** pour choisir la langue de l’interface.
4. Pour un ZIP portable, extrayez tout le répertoire. Conservez `web/` et les autres ressources du paquet à côté de l’exécutable ; ne lancez pas seulement un EXE copié ailleurs.

</DocSteps>

## Linux {#linux}

<DocSteps>

1. Téléchargez l’AppImage, le DEB ou l’archive portable appropriée et vérifiez sa somme de contrôle.
2. Pour l’AppImage, autorisez son exécution dans le gestionnaire de fichiers ou lancez `chmod +x` sur le fichier téléchargé, puis ouvrez-le.
3. Pour le DEB, utilisez le gestionnaire de paquets du système afin de résoudre les dépendances d’exécution.
4. Pour le TAR.GZ, extrayez tout le répertoire et lancez son exécutable de bureau en conservant les ressources du paquet.

</DocSteps>

## Résultat attendu {#expected-result}

TikTools s’ouvre hors ligne. Les données sont stockées séparément de l’installation, normalement dans `%LOCALAPPDATA%/TikTools` sous Windows ou `~/.local/share/TikTools` sous Linux.

## Récupération {#recovery}

Si le paquet échoue avant l’apparition d’une fenêtre, consultez les diagnostics et les erreurs de bibliothèques manquantes avant de modifier les paramètres de compte ou de migration. Sauvegardez le répertoire de données lorsque l’application est arrêtée avant d’essayer une autre version. Réinstaller un exécutable plus ancien n’annule pas les migrations de base de données. Poursuivez avec [votre premier LIVE](first-live.md) ou les [mises à jour](updates.md).

## Détails de fonctionnement {#operational-details}

Si aucune fenêtre ne s’ouvre, consultez le journal natif `tiktools.log` (`controller.log` en mode contrôleur) : Windows `%LOCALAPPDATA%/TikTools/logs`, Linux `${XDG_DATA_HOME:-~/.local/share}/TikTools/logs`, sauf modification par `TIKTOOLS_HOME` ou `TIKTOOLS_LOG_DIR`. L’initialisation peut échouer avant la création du fichier. Conservez l’erreur du terminal et consultez [conservation et confidentialité](preferences/troubleshooting.md#operational-details) avant partage.
