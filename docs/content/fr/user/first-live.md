---
title: "Première connexion LIVE"
docId: "user.start.first-live"
translationKey: "user.start.first-live"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fb7101f2010cd57478f9f6b2ea9cbb8c0a6401975fe4b29b311c41b13c1a5185"
translationRevision: "88a00db1305c8516e6587e0f42a3ebd41f074639358bef5b49b4f4ca9ef86727"
order: 30
---

# Première connexion LIVE {#first-live-connection}

## Objectif et prérequis {#goal-and-prerequisites}

Recevoir les événements d’un créateur avant d’activer des automatisations. Il vous faut une installation de bureau fonctionnelle, un accès réseau et un créateur actuellement en direct. La connexion Cloud est distincte et n’authentifie pas TikTok.

## Se connecter {#connect}

1. Ouvrez **Connections** ou l’action **Connect** dans la barre d’état inférieure.
2. Saisissez l’identifiant du créateur. Le préfixe `@` est facultatif. Vérifiez l’orthographe plutôt que d’utiliser le nom affiché.
3. Laissez **Use Cookie** désactivé lorsque l’accès invité est possible. Si la salle exige votre session autorisée, activez cette commande et fournissez le cookie uniquement dans son champ masqué.
4. Laissez la surveillance désactivée pour cette première connexion directe. Appuyez sur **Connect** et attendez la stabilisation de l’état de connexion.
5. Ouvrez **Live feed**. Demandez un message de discussion anodin ou attendez un événement réel. Vérifiez son créateur et son horodatage.
6. Sélectionnez un événement pour examiner ses détails. Sélectionnez le nom ou l’avatar d’un spectateur pour consulter son profil suivi.

## Résultat attendu {#expected-result}

L’état indique une connexion établie et les événements entrants apparaissent dans le flux. Une salle inactive peut légitimement ne produire aucun nouvel événement. Les points dépendent des règles de points activées, et les résultats des automatisations dépendent des actions et événements configurés.

## Surveillance et reconnexion {#monitoring-and-reconnecting}

Après une connexion directe réussie, **Monitor and auto-connect** peut attendre que le créateur passe en direct. Son état distingue l’attente, la vérification et l’activité de connexion. Arrêtez la surveillance pour modifier un champ de créateur verrouillé. La reconnexion au même créateur conserve le flux actuel ; changer de créateur ouvre un autre contexte de flux.

## Récupération {#recovery}

Si le créateur est hors ligne, confirmez que la salle est en direct avant de réessayer. En cas d’erreur d’authentification ou de limitation de débit, consultez **Diagnostics** et respectez la condition de nouvelle tentative indiquée ; des clics répétés ne réparent pas une session distante expirée. Si un cookie est nécessaire, obtenez une nouvelle session autorisée et excluez sa valeur des captures d’écran, journaux et demandes d’assistance. Il reste en mémoire et ne constitue pas un identifiant Cloud réutilisable.

Déconnectez-vous avant de tester un autre créateur. La démo permet d’apprendre ces commandes avec des événements fictifs, mais sa réussite ne valide pas une session TikTok.

<DemoFrame scenario="connection" locale="fr" tour="connection" title="Démo : connexion" />
