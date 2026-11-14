---
title: "Diagnostiquer une opération échouée"
docId: "user.preferences.troubleshooting"
translationKey: "user.preferences.troubleshooting"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "40d8c5706363baa28e4114397696c8c2adb320843271a00efb3772be0322d5e2"
translationRevision: "c9765cbd7e1784cbe7de04640e0de66ab0ed723aaf8933226353489559379fd2"
order: 30
---

# Diagnostiquer une opération échouée {#diagnose-a-failed-operation}

## Objectif et prérequis {#goal-and-prerequisites}

Identifier le composant en échec et recueillir un rapport utile sans déclencher des effets à répétition. Il faut la version de l’application, l’heure approximative de l’échec et l’opération tentée. Gardez une [sauvegarde](backups.md) avant toute réinitialisation ou remplacement de données.

## Examiner les éléments les plus proches {#inspect-the-nearest-evidence}

1. Lisez l’erreur affichée à l’endroit de l’échec. Utilisez son action d’état diagnostique ou ouvrez **Diagnostics** pour consulter les entrées associées.
2. Filtrez par source, niveau ou texte. Les sources utiles incluent LIVE, surveillance, plugins, compte, synchronisation, mises à jour, transport, catalogue, installation et collaboration.
3. Sélectionnez l’entrée concernée et conservez son heure, son code, les identifiants de requête/entité et une courte description. Développez les détails lorsqu’ils sont disponibles.
4. Copiez les diagnostics filtrés ou exportez le rapport texte. Si le presse-papiers ou l’exportation échoue, utilisez le texte de repli affiché. Examinez-le avant partage ; retirez les jetons privés, cookies, liens d’invitation et contenus personnels.
5. Incluez une courte reproduction : page, créateur/profil/plugin sélectionné, action effectuée, résultat attendu et résultat réel. Précisez si cela s’est produit avant ou après une mise à jour.

## Isoler les échecs courants {#isolate-common-failures}

Pour un flux vide, effacez la recherche et les filtres, reprenez le défilement et confirmez la [connexion](../live/connections.md). Un défilement en pause n’est pas une déconnexion. Un indicateur de perte d’affichage ne prouve pas que les points ou statistiques ont perdu le même événement.

Pour une automatisation sans exécution, vérifiez le profil actif, l’activation de l’événement/action, le déclencheur, les chemins des champs de filtre et le délai. Examinez l’historique avant de modifier les effets. **Test** peut exécuter des actions audio, HTTP, points et plugins.

Pour un échec de plugin, lisez l’erreur de tâche d’installation ou de connexion. Pour Cloud, vérifiez l’état du compte et le domaine concerné plutôt que de remplacer un cookie TikTok. Pour les incrustations, examinez l’état de la passerelle et la configuration de source. Pour les mises à jour, utilisez les [conseils de mise à jour manuelle](../updates.md) lorsque l’installation est indisponible.

## Résultat attendu et état {#expected-result-and-status}

Le filtrage, la copie et l’exportation texte des diagnostics sont disponibles. Une application empaquetée a aussi besoin de ses ressources web et de l’environnement d’exécution de la plateforme. Sous Linux, vérifiez l’environnement GTK/WebKitGTK pris en charge et la session X11/XWayland ; sous Windows, vérifiez WebView2. Consultez les [prérequis](../requirements.md) pour le périmètre d’installation.

## Récupération {#recovery}

Réessayez après correction de la cause précise. Pour les conflits de révision, rechargez et examinez l’état actuellement enregistré. Pour une commande distante non confirmée, consultez son journal/état avant d’en envoyer une autre : un délai d’attente dépassé peut suivre un effet réussi. Si le démarrage échoue, signalez le paquet, la version et le journal de démarrage plutôt que de supprimer le répertoire de données.

<DemoFrame scenario="diagnostics" locale="fr" tour="diagnostics" title="Démo : diagnostics" />

## Détails de fonctionnement {#operational-details}

Diagnostics est un journal UI en mémoire, limité à 2 000 entrées ou 2 MiB. Les répétitions en cinq secondes sont regroupées. La vue montre d’abord 100 entrées ; l’export contient toutes les entrées filtrées, mais **Effacer** supprime le journal entier. Exportez avant de fermer, redémarrer, effacer, changer de compte Cloud connu ou vous déconnecter : ces changements vident cet historique.

Sans fenêtre, cherchez le journal natif `tiktools.log` (`controller.log` en mode contrôleur) dans `%LOCALAPPDATA%/TikTools/logs` sous Windows ou `${XDG_DATA_HOME:-~/.local/share}/TikTools/logs` sous Linux. `TIKTOOLS_HOME` change la racine et `TIKTOOLS_LOG_DIR` remplace le répertoire des journaux. Si leur initialisation échoue, aucun fichier n’est garanti ; conservez l’erreur du terminal/démarrage disponible. La rédaction UI n’est pas garantie pour les journaux natifs : relisez avant partage.
