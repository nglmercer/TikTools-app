---
title: "Créer votre première règle de journalisation"
docId: "user.automations.first-rule"
translationKey: "user.automations.first-rule"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fc9bfc55b0fa1e4d41cdbdf75ec6c865707d53b3f8cf823871a97a0b7c9cfff2"
translationRevision: "fc9bfc55b0fa1e4d41cdbdf75ec6c865707d53b3f8cf823871a97a0b7c9cfff2"
order: 20
---

# Créer votre première règle de journalisation {#create-your-first-logging-rule}

## Objectif et prérequis {#goal-and-prerequisites}

Vérifier le parcours de l’événement à l’action avec une ligne de journal locale. Il faut un hôte de bureau actif et un accès à **Automations**. Aucun compte Cloud, service audio ou point de terminaison externe n’est nécessaire. Utilisez des messages fictifs ou non sensibles : la journalisation enregistre le message rendu.

## Créer l’action {#create-the-action}

1. Ouvrez **Automations → Actions** dans le profil souhaité.
2. Créez une action de type **Write to the log** (`core.log`) nommée `Vérification du chat`.
3. Définissez son champ Message avec le modèle suivant, puis enregistrez et activez cette action :

```text
Vérification du chat : {{ event.user.uniqueId }} a dit {{ event.data.comment }}
```

Ce sont les chemins normalisés actuels des discussions. `event.data.comment` est le message ; `event.user.uniqueId` est l’identifiant du spectateur. Le nom affiché utilise un autre champ.

## Créer l’événement {#create-the-event}

1. Ouvrez **Events**, créez `Événement de vérification du chat` et sélectionnez `tiktok.chat` comme déclencheur.
2. Reliez uniquement `Vérification du chat`. Conservez le mode normal exécutant toutes les actions.
3. Laissez les filtres vides pour le premier test sur échantillon. Définissez un délai global de `10` secondes pour limiter ensuite le trafic réel.
4. Enregistrez et examinez tout aperçu de modification. Avec uniquement l’action de journalisation liée, utilisez **Test** et examinez l’exécution et le texte du journal renvoyés. L’hôte peut fournir un échantillon ; son résultat ne prouve pas une connexion réelle à TikTok.
5. Ajoutez une condition sur `event.data.comment`, opérateur égal (`eq`), valeur `!bonjour`. Enregistrez à nouveau et activez explicitement l’événement.
6. Connectez-vous à un [LIVE](../live/connections.md) que vous êtes autorisé à tester. Observez un nouveau message `!bonjour` dans le [flux](../live/feed.md), puis vérifiez l’historique des automatisations une fois le délai écoulé.

## Résultat attendu et état {#expected-result-and-status}

L’action de journalisation locale et le déclencheur de discussion sont disponibles. L’échantillon doit produire un résultat lisible dans le journal ; un vrai message correspondant doit produire une exécution avec l’identifiant et le message réels. Les autres textes doivent échouer à la condition d’égalité exacte.

## Récupération {#recovery}

Si un échantillon ne correspond plus après l’ajout du filtre, son commentaire fourni peut différer de `!bonjour`. Vérifiez les champs de l’événement réel, le profil actif et les états d’activation plutôt que de déclencher à répétition. Les tests peuvent exécuter des effets ; limiter cette règle au journal rend son effet prévisible. Désactivez l’événement après utilisation ou poursuivez avec les [recettes](recipes.md).

<DemoFrame scenario="automation" locale="fr" tour="automation" title="Démo : automatisation" />
