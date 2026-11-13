---
title: "Préparer la collaboration et l’accès des équipes"
docId: "user.cloud.collaboration"
translationKey: "user.cloud.collaboration"
locale: fr
audience: "user"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "7bff4e49d94a8ee7e13b7a64bc8b66bec9090371bb6cefe0689ef63563424f8a"
translationRevision: "7bff4e49d94a8ee7e13b7a64bc8b66bec9090371bb6cefe0689ef63563424f8a"
order: 30
---

# Préparer la collaboration et l’accès des équipes {#prepare-collaboration-and-team-access}

## Objectif et prérequis {#goal-and-prerequisites}

Accorder à un autre opérateur un rôle défini en conservant le contrôle du propriétaire. Il faut un hôte de bureau capable et un choix délibéré entre collaboration locale/native et Teams lié au compte. Teams nécessite aussi une [connexion Cloud](account.md) et un service déployé compatible. Commencez avec un accès en lecture seule et une configuration hors production.

## Configurer l’accès du propriétaire {#configure-owner-access}

1. Ouvrez **Settings → Collaboration**. Examinez l’état de connexion et activez/configurez l’écouteur ou la passerelle voulus avec leurs commandes.
2. Examinez les origines autorisées et la matrice d’accès avant de créer une invitation. Choisissez uniquement les domaines, opérations et ressources nécessaires à l’opérateur.
3. Créez une invitation et transmettez son lien/jeton privé par un canal de confiance. Une invitation n’est pas une session active approuvée.
4. Examinez les demandes en attente et l’autorisation demandée. Approuvez uniquement l’opérateur attendu, puis confirmez son identité visible et sa connexion.
5. Utilisez l’inspecteur d’accès pour examiner les changements d’autorisation ultérieurs. Révoquez les invitations, identités ou sessions dont l’accès n’est plus nécessaire.

Modifier une définition par délégation, adopter une définition en tant que propriétaire et approuver une exécution déléguée sont des décisions distinctes. Lisez la source enregistrée, les actions référencées et les permissions dans l’examen concerné. Un aperçu de définition seul n’autorise pas un effet.

## Préparer une équipe liée au compte {#prepare-an-account-based-team}

Lorsque **Settings → Teams** est proposé, créez une équipe nommée et sélectionnez-la. Inscrivez **this PC** comme hôte avec un nom d’appareil reconnaissable, puis utilisez **Start serving** et examinez l’état de sa liaison. La création et l’inscription de l’hôte local sont raccordées dans l’interface ; elles sont distinctes de l’invitation d’un coéquipier. Un appareil contrôleur a son propre rôle d’inscription.

Choisissez un destinataire, un rôle et une durée d’invitation, puis examinez les demandes entrantes, membres et appareils. La recherche par pseudonyme nécessite un point de terminaison serveur déployé séparément et peut être indisponible sur le service actuel. Ne supposez pas qu’une recherche échouée signifie que la personne n’a pas de compte, et ne remplacez pas la sélection par un identifiant arbitraire.

## Résultat attendu et état {#expected-result-and-status}

Les commandes d’accès du propriétaire et la création/inscription des équipes dans l’interface existent. Le parcours complet de collaboration distante est expérimental : le fonctionnement LIVE réel entre deux ordinateurs, la recherche de pseudonymes en production, le comportement NAT/relais et tous les parcours de récupération des plateformes/effets ne sont pas établis ici. Une liaison prête ne prouve pas à elle seule l’application des permissions de bout en bout pour votre action.

## Récupération {#recovery}

Pour des autorisations ou révisions d’hôte obsolètes, rechargez et examinez à nouveau. Pour des commandes non confirmées, consultez le résultat enregistré avant de réessayer. Arrêtez le service ou révoquez l’accès en terminant une session. Excluez les jetons d’invitation et identifiants secrets des [rapports de diagnostic](../preferences/troubleshooting.md) publics.

<DemoFrame scenario="collaboration" locale="fr" title="Démo : collaboration" />
