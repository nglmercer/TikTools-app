---
title: "Configurer les règles de points"
docId: "user.points.rules"
translationKey: "user.points.rules"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "4e162cd76cb973f9215898e757b4765c429f1bc225df0903b5b40b1e3ad6fb9b"
translationRevision: "d24410f45e056534e95ca6be9182882400e40a7a9852af93a4ac7fb06e34f7cd"
order: 10
---

# Configurer les règles de points {#configure-point-rules}

## Objectif et prérequis {#goal-and-prerequisites}

Choisir comment l’activité entrante rapporte votre monnaie locale, puis vérifier une attribution simple. TikTools doit être lancé et vous devez connaître un créateur pour une vérification LIVE ultérieure. Les règles de points et les actions d’automatisation sont distinctes : une règle peut attribuer des points même sans automatisation créée.

## Définir la politique d’attribution {#set-the-earning-policy}

1. Ouvrez **Points** et trouvez le panneau des règles. Choisissez un nom de monnaie reconnaissable, comme `Étoiles`.
2. Activez uniquement les sources souhaitées : pièces des cadeaux, discussions, mentions J’aime, partages, abonnements ou arrivées. Chaque source possède son taux et son interrupteur d’activation.
3. Pour une première vérification simple, activez les discussions à `1` point et désactivez les autres sources. Laissez le bonus d’abonné à `0` pour établir la référence.
4. Définissez les points par niveau, par exemple `100`. L’éditeur impose un minimum de `10` ; les taux ne peuvent pas être négatifs.
5. Sélectionnez **Save** et attendez la confirmation. **Discard** restaure la configuration enregistrée pendant l’édition.
6. Connectez-vous via [Connections](../live/connections.md), observez un nouveau message dans le [flux](../live/feed.md), puis actualisez le classement et recherchez ce spectateur.

Les taux des cadeaux et mentions J’aime utilisent les quantités reçues. Par exemple, une contribution de `10` mentions J’aime à `0.1` point chacune donne `1` point avant tout bonus applicable. Une ligne affichée dans le flux ne correspond pas forcément à une unité de contribution. Changer les taux définit la politique des attributions suivantes ; cela ne recalcule pas le passé.

## Résultat attendu et état {#expected-result-and-status}

Les règles locales, soldes des spectateurs et commandes manuelles d’ajout/retrait sont disponibles. Avec la configuration isolée des discussions, un nouveau message accepté doit augmenter d’un point le solde de son auteur. Les niveaux suivent la valeur configurée de points par niveau. Il s’agit de points locaux de l’application, pas d’une monnaie TikTok ou d’une promesse de paiement.

## Récupération {#recovery}

Vérifiez d’abord l’interrupteur de la source, le taux enregistré et l’identifiant du spectateur. Une automatisation attribuant aussi des points peut ajouter une seconde attribution. Les commandes de réinitialisation sont verrouillées pendant les connexions LIVE actives ou occupées ; déconnectez-vous avant une réinitialisation prévue et lisez sa confirmation. Conservez une [sauvegarde avec l’application fermée](../preferences/backups.md) avant d’effacer les soldes accumulés.

<DemoFrame scenario="points" locale="fr" tour="points" title="Démo : points" />

## Détails de fonctionnement {#operational-details}

Le menu d’une ligne remet un spectateur à 0 point et au niveau 1 et ajoute un marqueur. **Tout réinitialiser** touche tous les spectateurs enregistrés dans cette installation, tous créateurs confondus : soldes à 0, niveaux à 1 et suppression de l’historique des transactions de points. Les fiches et compteurs cumulés restent présents. Le fil et Analytics ne sont pas effacés. Déconnectez le LIVE, sauvegardez l’application fermée et vérifiez la portée avant confirmation.

Les ajustements manuels positifs appliquent le bonus d’abonné enregistré : +100 avec un bonus de 50% accorde +150 à un abonné. Retirer 100 n’applique pas ce bonus ; le solde reste au minimum à zéro. Vérifiez le solde final. La recherche du tableau porte seulement sur son instantané des 100 premiers ; une absence ne prouve pas l’absence de fiche.
