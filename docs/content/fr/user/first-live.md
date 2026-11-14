---
title: "Première connexion LIVE"
docId: "user.start.first-live"
translationKey: "user.start.first-live"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fff247a8ec59baa1ebf6f4ad0019f456f1f48cfdd95ab624007bda0f7f208c96"
translationRevision: "090edc41ce906e586902a3f29adeedc52db5de950bbb2c237333b936a7621e41"
order: 30
---

# Première connexion LIVE {#first-live-connection}

## Objectif et prérequis {#goal-and-prerequisites}

Connectez un créateur, recevez un événement et examinez ses détails avant d’activer les automatisations. Pour un véritable LIVE, il vous faut une installation de bureau fonctionnelle, un accès à Internet et un créateur actuellement en direct.

<DocCallout title="Entraînez-vous avant votre diffusion" tone="tip">

Les exemples ci-dessous utilisent les vraies commandes avec des données fictives. Vous pouvez connecter `demo_creator` sans compte TikTok. Une démo réussie ne valide pas une véritable session TikTok. La connexion Cloud est distincte et n’authentifie pas TikTok.

</DocCallout>

## Se connecter {#connect}

<DocSteps>

1. **Choisissez le créateur.** Ouvrez **Connexions** ou utilisez **Connecter** dans la barre d’état inférieure. Saisissez l’identifiant du créateur ; le préfixe `@` est facultatif. Utilisez son identifiant plutôt que son nom affiché.
2. **Choisissez les options de connexion.** Laissez **Utiliser un cookie de session** désactivé lorsque l’accès invité est possible. Si la salle exige votre session autorisée, activez cette commande et saisissez le cookie uniquement dans son champ masqué. Laissez **Surveiller et connecter automatiquement** désactivé pour cette première connexion directe.
3. **Établissez la connexion.** Appuyez sur **Connecter** et attendez que l’état se stabilise. Essayez le panneau ci-dessous avec `demo_creator` et observez l’indicateur de connexion.

</DocSteps>

<DemoFrame scenario="connection" locale="fr" panel="connection" title="Essayer le panneau de connexion LIVE" />

## Résultat attendu {#expected-result}

L’état indique une connexion établie. Ouvrez **Activité en direct**, puis attendez un événement ou demandez un message de chat anodin. Vérifiez le créateur et l’horodatage, puis sélectionnez l’événement pour examiner ses détails. Sélectionnez le nom ou l’avatar d’un spectateur pour consulter son profil suivi.

<DemoFrame scenario="feed" locale="fr" panel="feed" title="Examiner des événements LIVE fictifs" />

Une salle inactive peut légitimement ne produire aucun nouvel événement. Les points dépendent des règles de points activées et les résultats des automatisations dépendent des actions et événements configurés. Recevoir un événement constitue la première vérification ; cela n’active pas automatiquement ces fonctionnalités.

## Surveillance et reconnexion {#monitoring-and-reconnecting}

Après une connexion directe réussie, **Surveiller et connecter automatiquement** peut attendre que le créateur passe en direct. Son état distingue l’attente, la vérification et l’activité de connexion. Arrêtez la surveillance pour modifier un champ de créateur verrouillé. La reconnexion au même créateur conserve le flux actuel ; changer de créateur ouvre un autre contexte de flux.

## Récupération {#recovery}

Si le créateur est hors ligne, confirmez que la salle est en direct avant de réessayer. En cas d’erreur d’authentification ou de limitation de débit, consultez **Diagnostics** et respectez la condition de nouvelle tentative indiquée ; des clics répétés ne réparent pas une session distante expirée.

<DocCallout title="Gardez vos cookies de session privés" tone="warning">

Si un cookie est nécessaire, obtenez une nouvelle session autorisée et excluez sa valeur des captures d’écran, journaux et demandes d’assistance. Il reste en mémoire et ne constitue pas un identifiant Cloud réutilisable. Déconnectez-vous avant de tester un autre créateur.

</DocCallout>
