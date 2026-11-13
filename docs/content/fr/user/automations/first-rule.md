---
title: "Créer votre première règle de journalisation"
docId: "user.automations.first-rule"
translationKey: "user.automations.first-rule"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "13b4b7b3cb8eba95257ec9738934813266414f504eec8b43090b73e74ad50c10"
translationRevision: "2e729493ef97d5d0a234d34af7407b4b0ed93783f1c80b406fbb24a0f9649b60"
order: 20
---

# Créer votre première règle de journalisation {#create-your-first-logging-rule}

## Objectif et prérequis {#goal-and-prerequisites}

Transformez un événement de chat en une ligne de journal locale et lisible. Il faut un hôte de bureau actif et un accès à **Automatisations**. Aucun compte Cloud, service audio ou point de terminaison externe n’est nécessaire.

<DocCallout title="Commencez par un résultat local et visible" tone="tip">

Limitez la première règle au journal pour pouvoir examiner son résultat. Utilisez des messages fictifs ou non sensibles : la journalisation enregistre le message rendu. L’exemple interactif utilise un espace de travail préparé que vous pouvez réinitialiser.

</DocCallout>

## Créer l’action {#create-the-action}

<DocSteps>

1. **Ouvrez l’éditeur d’action.** Accédez à **Automatisations → Actions** dans le profil souhaité.
2. **Nommez l’action.** Créez une action de type **Write to the log** (`core.log`) nommée `Vérification du chat`.
3. **Définissez le message.** Saisissez le modèle ci-dessous dans le champ **Message**, puis enregistrez et activez cette action.

</DocSteps>

```text
Vérification du chat : {{ event.user.uniqueId }} a dit {{ event.data.comment }}
```

`event.data.comment` est le message de chat normalisé ; `event.user.uniqueId` est l’identifiant du spectateur. Le nom affiché utilise un autre champ.

<DemoFrame scenario="automation" locale="fr" panel="automation" title="Explorer l’éditeur d’automatisations" />

## Créer l’événement {#create-the-event}

<DocSteps>

1. **Choisissez le déclencheur.** Ouvrez **Événements**, créez `Événement de vérification du chat` et sélectionnez `tiktok.chat`.
2. **Reliez l’action.** Ajoutez uniquement `Vérification du chat` et conservez le mode normal exécutant toutes les actions.
3. **Établissez une référence.** Laissez les filtres vides pour le premier test sur échantillon. Définissez un délai global de `10` secondes pour limiter ensuite le trafic réel.
4. **Testez le résultat.** Enregistrez et examinez tout aperçu de modification. Avec uniquement l’action de journalisation liée, utilisez **Lancer un test** et examinez l’exécution et le texte du journal renvoyés. Un échantillon fourni par l’hôte ne prouve pas une connexion réelle à TikTok.
5. **Précisez la règle.** Ajoutez une condition sur `event.data.comment`, opérateur égal (`eq`), valeur `!bonjour`. Enregistrez à nouveau et activez explicitement l’événement.
6. **Vérifiez un vrai message.** Connectez-vous à un [LIVE](../live/connections.md) que vous êtes autorisé à tester. Observez un nouveau message `!bonjour` dans le [flux](../live/feed.md), puis vérifiez l’historique des automatisations une fois le délai écoulé.

</DocSteps>

## Résultat attendu et état {#expected-result-and-status}

L’action de journalisation locale et le déclencheur de chat sont disponibles. L’échantillon doit produire un résultat lisible dans le journal. Un vrai message correspondant doit produire une exécution avec l’identifiant et le message réels ; les autres textes doivent échouer à la condition d’égalité exacte.

<DocCallout title="Vérifiez le parcours complet" tone="info">

Confirmez le profil actif, l’action et l’événement activés, le message correspondant et le délai écoulé. Voir un message dans le flux ne suffit pas à prouver qu’une automatisation s’est exécutée.

</DocCallout>

## Récupération {#recovery}

Si un échantillon ne correspond plus après l’ajout du filtre, son commentaire fourni peut différer de `!bonjour`. Vérifiez les champs de l’événement réel, le profil actif et les états d’activation plutôt que de déclencher à répétition. Les tests peuvent exécuter des effets ; limiter cette règle au journal rend son effet prévisible. Désactivez l’événement après utilisation ou poursuivez avec les [recettes](recipes.md).
