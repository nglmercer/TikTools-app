---
title: "Installer sous Windows ou Linux"
docId: "user.start.installation"
translationKey: "user.start.installation"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6f8673490bebb3447474a70d914d06102de87a9ef2f3f3c51da897d0d2cc8c1c"
translationRevision: "9936278f12631a92f9ebdd64f3c11f93327924c2786e3367a32537728bfba954"
order: 20
---

# Installer sous Windows ou Linux {#install-windows-or-linux}

## Objectif et prérequis {#goal-and-prerequisites}

Installer un paquet de bureau publié sans configurer de compilateur. Vérifiez les [prérequis](requirements.md), fermez toute instance de TikTools en cours et téléchargez le paquet depuis les [versions GitHub](https://github.com/nglmercer/TikTools-app/releases) du projet.

## Choisir un paquet {#choose-a-package}

| Paquet | Utilisation |
| --- | --- |
| Windows `installer.exe` | Installation NSIS par utilisateur ; WebView2 est nécessaire. |
| Linux `installer.AppImage` | Image exécutable portable pour un bureau Linux compatible. |
| Linux `installer.deb` | Installation avec un gestionnaire de paquets Debian/Ubuntu. |
| Windows ZIP / Linux TAR.GZ | Répertoire portable contenant l’exécutable et ses ressources. |

Vérifiez si la version est stable ou s’il s’agit d’une préversion. Les préversions servent au débogage et ne garantissent pas automatiquement la compatibilité en production. Utilisez le fichier `SHA256SUMS.txt` de la version pour vérifier les octets téléchargés avant de remplacer une installation existante.

## Windows {#windows}

1. Téléchargez l’installateur x86_64 et vérifiez sa somme de contrôle.
2. Exécutez l’installateur avec le compte utilisateur qui utilisera TikTools. Installez WebView2 si cela est demandé.
3. Lancez TikTools. Ouvrez **Settings → General** pour choisir la langue de l’interface.
4. Pour un ZIP portable, extrayez tout le répertoire. Conservez `web/` et les autres ressources du paquet à côté de l’exécutable ; ne lancez pas seulement un EXE copié ailleurs.

## Linux {#linux}

1. Téléchargez l’AppImage, le DEB ou l’archive portable appropriée et vérifiez sa somme de contrôle.
2. Pour l’AppImage, autorisez son exécution dans le gestionnaire de fichiers ou lancez `chmod +x` sur le fichier téléchargé, puis ouvrez-le.
3. Pour le DEB, utilisez le gestionnaire de paquets du système afin de résoudre les dépendances d’exécution.
4. Pour le TAR.GZ, extrayez tout le répertoire et lancez son exécutable de bureau en conservant les ressources du paquet.

## Résultat attendu {#expected-result}

TikTools s’ouvre hors ligne. Les données sont stockées séparément de l’installation, normalement dans `%LOCALAPPDATA%/TikTools` sous Windows ou `~/.local/share/TikTools` sous Linux.

## Récupération {#recovery}

Si le paquet échoue avant l’apparition d’une fenêtre, consultez les diagnostics et les erreurs de bibliothèques manquantes avant de modifier les paramètres de compte ou de migration. Sauvegardez le répertoire de données lorsque l’application est arrêtée avant d’essayer une autre version. Réinstaller un exécutable plus ancien n’annule pas les migrations de base de données. Poursuivez avec [votre premier LIVE](first-live.md) ou les [mises à jour](updates.md).

## Détails de fonctionnement {#operational-details}

Si aucune fenêtre ne s’ouvre, consultez le journal natif `tiktools.log` (`controller.log` en mode contrôleur) : Windows `%LOCALAPPDATA%/TikTools/logs`, Linux `${XDG_DATA_HOME:-~/.local/share}/TikTools/logs`, sauf modification par `TIKTOOLS_HOME` ou `TIKTOOLS_LOG_DIR`. L’initialisation peut échouer avant la création du fichier. Conservez l’erreur du terminal et consultez [conservation et confidentialité](preferences/troubleshooting.md#operational-details) avant partage.
