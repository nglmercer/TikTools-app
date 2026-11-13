---
title: "Trouver les préférences et les commandes clavier"
docId: "user.preferences.index"
translationKey: "user.preferences.index"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6790ab7171a90bda1df882c0f88626a5875da68bf4f624968b0e54815842030f"
translationRevision: "fd118aea8302443fc3b89b7fc4c9a7b63de7133ed2049d3bfadad4034bc84745"
order: 10
---

# Trouver les préférences et les commandes clavier {#find-preferences-and-keyboard-controls}

## Objectif et prérequis {#goal-and-prerequisites}

Définir la langue de l’application, trouver le bon espace de configuration et utiliser confortablement les commandes clavier prises en charge. La fenêtre de bureau doit être ouverte ; l’accès réseau est nécessaire uniquement pour le service configuré.

## Trouver le paramètre concerné {#find-the-relevant-setting}

1. Ouvrez **Settings → General** pour choisir la langue de l’interface. Les choix actuels sont l’anglais (`en`), l’espagnol (`es`), le portugais (`pt`), le français (`fr`), le chinois simplifié (`zh`) et le japonais (`ja`). Choisir une langue ne modifie pas les identifiants d’événements TikTok ni les noms d’API des plugins.
2. Examinez les commandes de mise à jour dans General : canal, vérifications automatiques, notifications et vérification manuelle lorsqu’elle est prise en charge. Les vérifications automatiques sont facultatives. La disponibilité des mises à jour signées dépend de la configuration de signature publiée de la compilation.
3. Utilisez **Account** pour l’identité Cloud et ses diagnostics. Utilisez **Collaboration** pour configurer l’accès du propriétaire, et **Teams** lorsque l’hôte expose cette capacité.
4. Ouvrez **Plugins** pour les préférences communes aux plugins et les volets Settings/Connection de chaque plugin. Les identifiants du service d’un plugin n’appartiennent pas aux paramètres General.
5. Ouvrez **Automations → Variables** pour les valeurs d’exécution partagées, **Points** pour les règles d’attribution et **Overlays** pour la conception des widgets. Ces espaces de configuration sont volontairement placés à côté des fonctionnalités concernées.

L’installation automatique est expérimentale et désactivée par les conditions de compilation du processus de publication actuel. Une préférence visible de vérification automatique ne promet pas une installation automatique. Utilisez la [procédure de mise à jour](../updates.md) documentée et faites une [sauvegarde](backups.md) avant tout changement de version.

## Utiliser les commandes clavier prises en charge {#use-supported-keyboard-controls}

Utilisez Tab et Shift+Tab pour déplacer le focus entre les commandes ordinaires, et Entrée/Espace pour activer les boutons ciblés. Dans l’inspecteur de conception d’incrustation, Ctrl/Meta+Z annule et Ctrl/Meta+Shift+Z ou Ctrl/Meta+Y rétablit. Les champs texte conservent leur propre annulation. Ce sont des raccourcis d’éditeur, pas des raccourcis globaux d’automatisation.

Échap ferme le volet de navigation étroit si une autre commande ne l’a pas consommé. Les boîtes de dialogue et éditeurs peuvent gérer Échap eux-mêmes, y compris avec une demande concernant les modifications non enregistrées. L’automatisation par raccourcis globaux nécessite un plugin compatible installé et les capacités annoncées de son moteur.

## Résultat attendu, état et récupération {#expected-result-status-and-recovery}

Ces paramètres et interactions clavier locales sont disponibles ; aucune validation complète d’accessibilité ou de plateforme n’est affirmée. Préférez des boutons libellés, des tailles de fenêtre lisibles et le défilement du flux en pause lorsque le mouvement distrait. Si un paramètre semble absent, vérifiez sa page propriétaire et la capacité de l’hôte avant de réinitialiser les données. Consultez le [dépannage](troubleshooting.md) pour les commandes indisponibles et la [FAQ](faq.md) pour les distinctions courantes.

<DemoFrame scenario="settings" locale="fr" title="Démo : paramètres" />
