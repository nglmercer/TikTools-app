---
title: "Préparer des incrustations pour OBS"
docId: "user.overlays.index"
translationKey: "user.overlays.index"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6062b2d3d3248eaa9ab1c8f73082c8d4a6bbe81d199f8a45b6c8c28dea617d46"
translationRevision: "5b995ec3be41ce58c3cd9d1a45dc72b06406d367a8f326165e8b526303ecaea3"
order: 10
---

# Préparer des incrustations pour OBS {#prepare-overlays-for-obs}

## Objectif et prérequis {#goal-and-prerequisites}

Concevoir un widget lisible et connecter sa source navigateur locale à OBS. Il faut TikTools en cours d’exécution, la capacité de passerelle d’incrustation et OBS sur le même ordinateur pour l’adresse de bouclage par défaut. La sortie en direct nécessite aussi l’événement entrant concerné ou une source de classement configurée.

## Concevoir et prévisualiser {#design-and-preview}

1. Ouvrez **Overlays** et lisez l’état de la passerelle. Choisissez abonnement, cadeau, discussion, partage, souscription, mentions J’aime ou classement.
2. Modifiez les calques, textes visibles, couleurs, espacements et options d’avatar dans l’inspecteur de conception. Utilisez un contraste suffisant et une taille lisible à votre résolution réelle de diffusion.
3. Rejouez l’aperçu pour examiner la disposition. Son contenu aide à évaluer la conception ; il ne prouve pas la diffusion LIVE.
4. Utilisez les commandes Undo/Redo pour les modifications locales, puis **Save**. Changer de type de widget avec des modifications non enregistrées demande de les abandonner.
5. Dans le panneau d’aperçu/source, copiez l’URL de source. L’URL affichée masque son identifiant secret ; utilisez la commande de copie pour obtenir l’URL utilisable. Gardez-la privée.
6. Ajoutez une source navigateur OBS et collez l’URL copiée. Gardez TikTools actif et validez un événement pertinent avant de vous y fier pendant une diffusion.

La passerelle par défaut est `127.0.0.1:17452`. Une URL de bouclage désigne l’ordinateur exécutant la source navigateur ; la copier sur un autre ordinateur ne permet donc pas automatiquement d’atteindre cet hôte. Les widgets de classement nécessitent aussi un identifiant de classement sélectionné ; consultez l’[état des classements](../points/rankings.md).

## Utiliser la bibliothèque Cloud {#use-the-cloud-library}

Lorsque l’hôte expose les capacités d’incrustation Cloud, ouvrez la bibliothèque depuis Overlays. Parcourez les conceptions locales ou **My overlays**, utilisez l’éditeur pour créer une incrustation Cloud, ou ouvrez un identifiant existant et révisez-le. L’éditeur Cloud prend en charge le texte, les champs en direct, les illustrations et les dimensions du canevas ; les téléversements d’images suivent un parcours distinct. L’enregistrement local d’une conception et sa création/révision Cloud sont des opérations distinctes.

## Résultat attendu et état {#expected-result-and-status}

Les conceptions locales, commandes de passerelle et parcours d’éditeur/bibliothèque Cloud conditionnés aux capacités sont disponibles. Aucun adaptateur **Changes** partagé pour les incrustations n’est disponible dans cette version. OBS, le LIVE réel et chaque système d’exploitation nécessitent toujours une validation dans votre environnement.

## Récupération {#recovery}

En l’absence de sortie, vérifiez l’état de la passerelle, les ressources, les identifiants secrets, l’URL de source et le type d’événement sélectionné. Pour un conflit de révision Cloud, rechargez la révision actuelle avant de modifier à nouveau. Ne publiez jamais le jeton de source dans des captures d’écran ; signalez l’erreur de passerelle via les [diagnostics](../preferences/troubleshooting.md).

<DemoFrame scenario="overlays" locale="fr" tour="overlay" title="Démo : incrustations" />

## Détails de fonctionnement {#operational-details}

Installez Event Gateway séparément, activez-le et vérifiez son exécution avant les sources OBS LIVE. États : missing → installer ; disabled → activer/vérifier disponibilité ; stopped → démarrer ; credentialUnavailable → redémarrer une fois ; assetsMissing → réparer/reconstruire les assets empaquetés ; starting → réessayer brièvement ; unreachable → vérifier port/service loopback. Utilisez la clé widget de l’URL générée. Ready ou aperçu local ne prouvent ni événement LIVE ni réception OBS : testez sans effet nuisible sur la source prévue.
