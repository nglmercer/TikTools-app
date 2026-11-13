---
title: "Gérer les connexions LIVE"
docId: "user.live.connections"
translationKey: "user.live.connections"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "37f7274550d3010051ab456da2a2091b650e5e7aaaf30756b6695435bcfc08c7"
translationRevision: "37f7274550d3010051ab456da2a2091b650e5e7aaaf30756b6695435bcfc08c7"
order: 10
---

# Gérer les connexions LIVE {#manage-live-connections}

## Objectif et prérequis {#goal-and-prerequisites}

Se connecter à un créateur précis, comprendre la reconnexion et configurer les serveurs de plugins indépendamment. Il vous faut une [installation de bureau](../installation.md) fonctionnelle, un accès réseau et l’identifiant du créateur. Consultez les [prérequis](../requirements.md) avant de diagnostiquer des problèmes de système d’exploitation.

## Se connecter de manière délibérée {#connect-deliberately}

1. Ouvrez **Connections**. Saisissez l’identifiant, avec ou sans `@` ; un nom affiché n’est pas le même identifiant.
2. Laissez **Use Cookie** désactivé au départ. Si TikTok exige une session autorisée, activez cette option et saisissez le cookie uniquement dans le champ masqué. Les cookies restent en mémoire ; ne les collez pas dans les diagnostics ou les paramètres Cloud.
3. Laissez **Monitor and auto-connect** désactivé pour une tentative directe, puis sélectionnez **Connect**. Lisez l’état et toute erreur avant de réessayer.
4. Consultez [Live feed](feed.md) pour vérifier le créateur et la présence d’un nouvel événement. Une salle connectée mais calme peut ne montrer aucune activité.
5. Utilisez **Reconnect** pour le même créateur ou **Disconnect** avant d’en changer. Pendant une tentative en cours, **Cancel** l’arrête.

Après une connexion directe réussie, activez la surveillance pour attendre qu’un créateur passe en direct. L’attente et la vérification sont distinctes de la connexion établie. Arrêtez la surveillance avant de modifier un identifiant verrouillé. Sélectionner un créateur récent démarre une connexion ; cela ne remplit pas simplement le champ.

L’explorateur propose des salles découvertes, mais les résultats peuvent devenir obsolètes. Actualisez-les avant de supposer qu’une salle est toujours disponible. Les connexions des serveurs de plugins dans le panneau inférieur configurent leurs propres services ; un plugin connecté n’authentifie ni TikTok ni Cloud.

## Résultat attendu et état {#expected-result-and-status}

Les connexions directes, la surveillance et les commandes de connexion des plugins sont disponibles. La reconnexion au même créateur conserve le contexte du flux actuel ; changer de créateur ouvre un autre contexte. L’accès réel à TikTok dépend de la salle et de la session distantes ; un test local réussi ne le garantit pas.

## Récupération {#recovery}

Pour une salle hors ligne, vérifiez que le créateur diffuse en direct. En cas d’erreur de session ou de limitation de débit, respectez la condition de nouvelle tentative indiquée. Cliquer plusieurs fois sur la reconnexion ne renouvelle pas l’authentification. Consultez le [dépannage](../preferences/troubleshooting.md) pour recueillir le composant en échec et l’heure sans exposer d’identifiants.
