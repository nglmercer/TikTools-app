---
title: "Prérequis"
docId: "user.start.requirements"
translationKey: "user.start.requirements"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "c68c149552a943cf0f03393c5dac91bd62abf9c50f791d7b0757599754f4804e"
translationRevision: "ee008aeff5df0b3917848ed29fc346703f1984d672c0fae300f902205fd724bd"
order: 10
---

# Prérequis {#requirements}

## Objectif {#goal}

Choisir un paquet de bureau pris en charge et identifier les services nécessaires à votre usage avant l’installation.

## Ce dont vous avez besoin {#what-you-need}

- Un ordinateur Windows x86_64 ou Linux x86_64 pour les paquets de bureau publiés. Le processus de publication actuel compile ces deux cibles. Il ne publie ni version de bureau macOS ni APK Android.
- Une session graphique fonctionnelle. Windows nécessite le runtime WebView2 de Microsoft. Linux nécessite un environnement GTK/WebKitGTK compatible ; le DEB déclare ses paquets d’exécution et l’AppImage inclut des utilitaires multimédias vérifiés.
- Un accès réseau à TikTok pour une véritable connexion LIVE. Une salle hors ligne, une session invitée expirée ou une limitation du service distant ne se corrigent pas en modifiant une automatisation.
- Un espace de stockage hors du répertoire d’installation pour les données SQLite, plugins et journaux. L’application détermine un répertoire de données par utilisateur ; déplacer l’exécutable ne déplace pas ces données.

Un compte Cloud est nécessaire pour les opérations de catalogue ou de collaboration liées au compte, pas pour tous les éditeurs locaux. OBS n’est nécessaire que pour afficher un widget dans une diffusion. Les intégrations audio/TTS peuvent nécessiter un plugin installé séparément et son propre fournisseur ou service.

## Vérifier l’environnement {#check-the-environment}

1. Choisissez le paquet correspondant à votre système d’exploitation et à l’architecture x86_64.
2. Sous Windows, installez WebView2 si l’installateur signale son absence. Sous Linux, installez les bibliothèques d’exécution manquantes avec le gestionnaire de paquets de votre distribution.
3. Vérifiez que la fenêtre de l’application s’ouvre avant de vous connecter à TikTok ou d’ajouter des plugins.
4. Poursuivez avec l’[installation](installation.md), puis [votre premier LIVE](first-live.md).

## Résultat attendu {#expected-result}

L’application s’ouvre et sa navigation principale fonctionne hors ligne. Les intégrations facultatives peuvent rester indisponibles tant que leur plugin ou la capacité correspondante de l’hôte manque.

## Récupération {#recovery}

En cas de fenêtre vide ou de fermeture précoce, recueillez les diagnostics de démarrage et la version exacte du paquet. AppImage et DEB sont deux formats d’installation différents ; aucun n’est un exécutable Android. Consultez le [périmètre Termux et sans interface graphique](termux.md) pour cet environnement. La compilation depuis les sources comporte des prérequis supplémentaires dans le guide de développement.

## Détails de fonctionnement {#operational-details}

Si aucune fenêtre ne s’ouvre, consultez le journal natif `tiktools.log` (`controller.log` en mode contrôleur) : Windows `%LOCALAPPDATA%/TikTools/logs`, Linux `${XDG_DATA_HOME:-~/.local/share}/TikTools/logs`, sauf modification par `TIKTOOLS_HOME` ou `TIKTOOLS_LOG_DIR`. L’initialisation peut échouer avant la création du fichier. Conservez l’erreur du terminal et consultez [conservation et confidentialité](preferences/troubleshooting.md#operational-details) avant partage.
