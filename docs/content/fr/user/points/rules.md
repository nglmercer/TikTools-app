---
title: "Configurer les règles de points"
docId: "user.points.rules"
translationKey: "user.points.rules"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "89e0189daf4193853d17ec9e576a4ff9973cdfc5df06d0b27fc59296d9ee66de"
translationRevision: "e57738cda53ced5da0c5a8c0992b85a4f80a1f950e0676a1a6d4f9a83554a7d2"
order: 10
---

# Configurer les règles de points {#configure-point-rules}

## Objectif et prérequis {#goal-and-prerequisites}

Choisissez comment l’activité entrante rapporte votre monnaie locale, puis vérifiez une attribution simple. TikTools doit être lancé et vous devez connaître un créateur pour une vérification LIVE ultérieure. Les règles de points et les actions d’automatisation sont distinctes : une règle peut attribuer des points même sans automatisation créée.

<DocCallout title="Testez une source à la fois" tone="tip">

Commencez uniquement avec le chat, à un point par message. Le solde attendu est alors facile à vérifier avant d’ajouter les cadeaux, mentions J’aime ou bonus d’abonné. Il s’agit de points locaux de l’application, pas d’une monnaie TikTok ou d’une promesse de paiement.

</DocCallout>

## Définir la politique d’attribution {#set-the-earning-policy}

<DocSteps>

1. **Nommez la monnaie.** Ouvrez **Points**, trouvez **Règles de points** et choisissez un **Nom de la monnaie** reconnaissable, comme `Étoiles`.
2. **Choisissez les sources.** Chaque source a son taux et son interrupteur : Pièces de cadeaux, Messages du chat, J’aime, Partages, Abonnements et Arrivées. Pour la première vérification, activez uniquement **Messages du chat** à `1` point. Laissez **Bonus d’abonné** à `0`.
3. **Réglez la progression.** Définissez **Points par niveau**, par exemple `100`. L’éditeur impose un minimum de `10` ; les taux ne peuvent pas être négatifs.
4. **Enregistrez la politique.** Sélectionnez **Enregistrer les règles** et attendez la confirmation. **Annuler** restaure la configuration enregistrée pendant l’édition.

</DocSteps>

<DemoFrame scenario="points" locale="fr" panel="points" title="Essayer les règles de points et les soldes des spectateurs" />

Connectez-vous via [Connexions](../live/connections.md), observez un nouveau message dans le [flux](../live/feed.md), puis utilisez **Actualiser le classement** et recherchez ce spectateur.

Les taux des cadeaux et mentions J’aime utilisent les quantités reçues. Par exemple, une contribution de `10` mentions J’aime à `0.1` point chacune donne `1` point avant tout bonus applicable. Une ligne affichée dans le flux ne correspond pas forcément à une unité de contribution. Changer les taux définit la politique des attributions suivantes ; cela ne recalcule pas le passé.

## Résultat attendu et état {#expected-result-and-status}

Les règles locales, soldes des spectateurs et commandes manuelles d’ajout/retrait sont disponibles. Avec la configuration isolée du chat, un nouveau message accepté doit augmenter d’un point le solde de son auteur. Les niveaux suivent la valeur configurée de points par niveau.

<DocCallout title="Comparez le solde obtenu" tone="info">

Une automatisation attribuant aussi des points peut ajouter une seconde attribution. Vérifiez les sources activées et les automatisations actives si un spectateur reçoit davantage que le point attendu pour un message isolé.

</DocCallout>

## Récupération {#recovery}

Vérifiez d’abord l’interrupteur de la source, le taux enregistré et l’identifiant du spectateur. Les commandes de réinitialisation sont verrouillées pendant les connexions LIVE actives ou occupées. Déconnectez-vous avant une réinitialisation prévue, lisez sa confirmation et conservez une [sauvegarde avec l’application fermée](../preferences/backups.md) avant d’effacer les soldes accumulés.

## Détails de fonctionnement {#operational-details}

<DocCallout title="Comprenez la portée de la réinitialisation" tone="warning">

Le menu d’une ligne remet un spectateur à 0 point et au niveau 1 et ajoute un marqueur de réinitialisation. **Réinitialiser tous les points** touche tous les spectateurs enregistrés dans cette installation, tous créateurs confondus : soldes à 0, niveaux à 1 et suppression de l’historique des transactions de points. Les fiches des spectateurs et les compteurs cumulés restent présents ; le flux et les Statistiques ne sont pas effacés. Déconnectez le LIVE, sauvegardez l’application fermée et vérifiez la portée avant de confirmer.

</DocCallout>

Les ajustements manuels positifs appliquent le bonus d’abonné enregistré : +100 avec un bonus de 50 % accorde +150 à un abonné. Un ajustement de −100 n’applique pas ce bonus ; le solde reste au minimum à zéro. Vérifiez le solde final plutôt que de supposer que la variation saisie est exacte. La recherche du tableau porte seulement sur son instantané des 100 premiers ; une absence ne prouve pas l’absence de fiche enregistrée.
