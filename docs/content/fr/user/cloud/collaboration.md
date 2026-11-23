---
title: "Préparer la collaboration et l’accès des équipes"
docId: "user.cloud.collaboration"
translationKey: "user.cloud.collaboration"
locale: "fr"
audience: "user"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "05f448dda78b05027b84b2c03f08a15eb515aee05af79397a1596d7e40ec5d40"
translationRevision: "6435fb775f3eab73258d699fbd0745f267e3dcfe87af16f62c59e99fcee35661"
order: 30
---

# Préparer la collaboration et l’accès des équipes {#prepare-collaboration-and-team-access}

## Objectif et prérequis {#goal-and-prerequisites}

Accordez à un autre opérateur un rôle défini en conservant le contrôle du propriétaire. L’accès distant est fourni par le plugin installable **Remote Control**, et non par un onglet des préférences : il faut une [connexion Cloud](account.md), un service compatible et le plugin installé pour cet hôte. Commencez avec un accès en lecture seule et une configuration hors production.

## Configurer l’accès distant {#configure-remote-access}

1. Installez et activez `tiktools.remote-control`, puis ouvrez sa page **Remote control**.
2. Créez une équipe et inscrivez **ce PC** comme hôte avec un nom d’appareil reconnaissable ; démarrez la liaison de relais et attendez que son état atteigne `ready`.
3. Créez une invitation et transmettez son jeton à usage unique par un canal de confiance. Une invitation n’est pas une session active approuvée.
4. Examinez la demande en attente et le rôle demandé. N’approuvez que l’opérateur attendu ; l’autorisation reste `pendingHost` jusqu’à ce que la liaison de l’hôte la confirme.
5. Vérifiez régulièrement les autorisations et les appareils. Révoquez une autorisation dont l’accès n’est plus nécessaire.

Modifier une définition par délégation, adopter une définition en tant que propriétaire et approuver une exécution déléguée sont des décisions distinctes. Lisez la source enregistrée, les actions référencées et les permissions dans l’examen concerné. Un aperçu de définition seul n’autorise pas un effet.

## Préparer un appareil contrôleur {#prepare-a-controller-device}

Sur l’appareil contrôleur, connectez-vous avec son propre compte, inscrivez-le comme contrôleur et échangez le jeton d’invitation. Une demande en attente apparaît alors pour le propriétaire. La recherche par pseudonyme nécessite un point de terminaison serveur déployé séparément et peut être indisponible sur le service actuel. Ne supposez pas qu’une recherche échouée signifie que la personne n’a pas de compte, et ne remplacez pas la sélection par un identifiant arbitraire.

## Résultat attendu et état {#expected-result-and-status}

Le parcours entre deux comptes (équipe, liaison de l’hôte, invitation, approbation, commande de lecture, révocation) a été vérifié avec un serveur local. Les onglets Collaboration/Équipes des préférences ont été supprimés ; le service de collaboration natif n’a pas d’interface web dans cette version. Le comportement du relais/NAT en production, la recherche par pseudonyme et le fonctionnement LIVE entre deux PC ne sont pas établis ici. Une liaison prête ne prouve pas à elle seule l’application des permissions de bout en bout pour votre action.

## Récupération {#recovery}

Pour des autorisations obsolètes, révoquez l’accès et vérifiez-le à nouveau après l’actualisation de la liaison. Pour des commandes non confirmées, consultez le résultat enregistré avant de réessayer. Arrêtez la liaison ou révoquez l’accès en terminant une session. Excluez les jetons d’invitation et les identifiants secrets des [rapports de diagnostic](../preferences/troubleshooting.md) publics.

## Détails de fonctionnement {#operational-details}

Chaque commande distante passe par le contrôle de capacités de l’hôte (`remote.command.dispatch`, `remote.pair.list`, `remote.grant.list`, `remote.grant.revoke`, `remote.controller.ticket`) ; le plugin ne reçoit jamais d’API de contrôle ni de jeton complet. Un transport actif ou l’appartenance à une équipe ne donnent pas seuls d’accès natif. Observer privilégie la lecture ; moderator ajoute certains contrôles, automationEditor certaines éditions, custom exige une politique explicite revue. Aucun ne donne des scripts/effets illimités. Consultez [rôles et confirmation](../../developer/collaboration.md#operational-details) et [transports](../../developer/transports.md) pour la frontière actuelle.
