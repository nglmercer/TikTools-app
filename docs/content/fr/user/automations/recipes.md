---
title: "Adapter des recettes pratiques d’automatisation"
docId: "user.automations.recipes"
translationKey: "user.automations.recipes"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "1486a39572d9a4151216f867dbe4dd5c448905678d962a58faedf0685e2978fb"
translationRevision: "b5cee9e2afeb9f6390b1f0cc315d9340f2efb7db352f5bfecd3e688296404400"
order: 30
---

# Adapter des recettes pratiques d’automatisation {#adapt-practical-automation-recipes}

## Objectif et prérequis {#goal-and-prerequisites}

Transformer une [règle de journalisation](first-rule.md) fonctionnelle en réactions précisément ciblées. Il faut un profil actif et des champs d’événement observés. Commencez avec des actions `core.log` ; remplacez-les par un autre effet seulement après avoir confirmé la correspondance et la fréquence.

## Reconnaître une commande de discussion {#match-a-chat-command}

Créez un événement `tiktok.chat` avec `event.data.comment` égal (`eq`) à `!bonjour`. Journalisez :

```text
Bonjour {{ event.user.uniqueId }}
```

Utilisez un délai par utilisateur de `30` secondes pour qu’un spectateur ne puisse pas le déclencher continuellement, tout en laissant aux autres une possibilité indépendante. L’égalité exacte convient à une commande ; une correspondance par contenu accepte aussi les messages avec du texte autour. Choisissez délibérément.

## Enregistrer un cadeau répondant au critère {#record-a-qualifying-gift}

Créez un événement `tiktok.gift` avec `event.data.diamondCount` supérieur ou égal (`gte`) à `10`. Journalisez :

```text
Cadeau {{ event.data.giftName }} de {{ event.user.uniqueId }}
```

Examinez d’abord les détails des cadeaux réels. La valeur en diamants, le nombre de répétitions et l’indicateur de fin d’un cadeau sont des champs différents. Si vous souhaitez réagir uniquement à la fin, vérifiez que la charge utile entrante fournit `event.data.repeatEnd`, puis utilisez la condition booléenne vraie. Un filtre sur le nom seul ne garantit pas une exécution par série terminée.

## Limiter une réaction partagée {#bound-a-shared-reaction}

Créez un événement `tiktok.follow` avec un délai global de `10` secondes. Reliez une courte action de journalisation. Si deux spectateurs s’abonnent dans cet intervalle, le délai global peut supprimer la réaction suivante. Utilisez plutôt une portée par utilisateur lorsque chacun doit disposer de sa propre autorisation.

Pour plusieurs actions, ordonnez la liste et conservez le mode exécutant toutes les actions lorsque chaque étape est nécessaire. Le mode aléatoire choisit une action ; il n’exécute pas toutes les étapes dans un ordre aléatoire. Ajoutez `core.delay` uniquement lorsque les étapes suivantes doivent attendre.

## Résultat attendu et état {#expected-result-and-status}

Ces déclencheurs, filtres de champs et commandes de délai sont disponibles dans le moteur local. Une exécution doit correspondre à un événement accepté satisfaisant les critères, sous réserve du délai et de l’activation. La validation propre au fournisseur nécessite toujours votre vérification LIVE réelle.

## Récupération {#recovery}

Si une recette échoue, revenez à une seule action de journalisation, examinez la charge utile et simplifiez temporairement les conditions. Ne désactivez pas les limites sur une diffusion chargée pendant l’enquête. Les tests audio, points, HTTP et plugins peuvent produire des effets réels. Consultez la [configuration audio](../plugins/audio.md), les [variables](variables.md) et les [diagnostics d’exécution](../preferences/troubleshooting.md) avant d’ajouter ces dépendances.

Essayez interactivement avant de toucher à votre vraie configuration :

<DemoFrame scenario="automation" locale="fr" panel="automation" title="Essayez l'espace des automatisations" />

<TourFrame id="automationEventChat" locale="fr" title="Guide interactif : Événements de chat et commandes" />

<TourFrame id="automationEventGift" locale="fr" title="Guide interactif : Événements de cadeau" />

<TourFrame id="automationEventSocial" locale="fr" title="Guide interactif : Événements sociaux" />

<TourFrame id="automationEventRoom" locale="fr" title="Guide interactif : Événements de salle" />

<TourFrame id="automationActionHttp" locale="fr" title="Guide interactif : Actions webhook (HTTP)" />

<TourFrame id="automationActionVars" locale="fr" title="Guide interactif : Variables dans les actions" />
