---
title: "Prérequis"
docId: "user.start.requirements"
translationKey: "user.start.requirements"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "f3bc30b94983b95c7ce6ffc7fe8b3707aa175cadd2156f26f16b8b77be83ea6b"
translationRevision: "f3bc30b94983b95c7ce6ffc7fe8b3707aa175cadd2156f26f16b8b77be83ea6b"
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
