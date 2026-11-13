---
title: "Préparer des incrustations pour OBS"
docId: "user.overlays.index"
translationKey: "user.overlays.index"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "3141f778f3bf0e5c9e01415122e7863e9342c266a57bee9811d7a6b75b2c842b"
translationRevision: "c8223076fba0cf25b93837f2a62ce346feac639d1f45d0a5522a471eb2f86ea2"
order: 10
---

# Préparer des incrustations pour OBS {#prepare-overlays-for-obs}

## Objectif et prérequis {#goal-and-prerequisites}

Concevez un widget lisible, examinez son aperçu et connectez sa source navigateur à OBS. Il faut TikTools en cours d’exécution, la capacité de passerelle d’incrustation et OBS sur le même ordinateur pour l’adresse de bouclage par défaut. La sortie en direct nécessite aussi l’événement entrant concerné ou une source de classement configurée.

<DocCallout title="Vérifiez d’abord le design dans l’aperçu" tone="tip">

Utilisez l’éditeur d’exemple pour apprendre les commandes avant de configurer une diffusion. Son aperçu fictif aide à évaluer l’apparence ; il ne prouve ni la diffusion LIVE réelle ni la réception dans OBS.

</DocCallout>

## Concevoir et prévisualiser {#design-and-preview}

<DocSteps>

1. **Choisissez un widget.** Ouvrez **Incrustations**, lisez l’état de la passerelle et choisissez abonnement, cadeau, chat, partage, abonnement payant, mentions J’aime ou classement.
2. **Rendez-le lisible.** Modifiez les calques, textes visibles, couleurs, espacements et options d’avatar dans l’inspecteur de conception. Vérifiez le contraste et la taille du texte à votre résolution réelle de diffusion.
3. **Examinez l’aperçu.** Utilisez **Rejouer l’aperçu** et vérifiez la disposition. Les commandes **Annuler** et **Rétablir** concernent les modifications locales ; sélectionnez ensuite **Enregistrer le design**. Changer de type de widget avec des modifications non enregistrées demande de les abandonner.

</DocSteps>

<DemoFrame scenario="overlays" locale="fr" panel="overlays" title="Essayer les commandes de design des incrustations" />

<DocSteps>

1. **Copiez la source.** Dans le panneau **URL de la source navigateur**, utilisez **Copier l’URL** pour obtenir l’URL utilisable. L’URL affichée masque son identifiant secret ; gardez l’URL copiée privée.
2. **Ajoutez-la à OBS.** Créez une source navigateur OBS et collez l’URL copiée. Gardez TikTools actif et validez un événement pertinent avant de vous y fier pendant une diffusion.

</DocSteps>

La passerelle par défaut est `127.0.0.1:17452`. Une URL de bouclage désigne l’ordinateur exécutant la source navigateur ; la copier sur un autre ordinateur ne permet donc pas automatiquement d’atteindre cet hôte. Les widgets de classement nécessitent aussi un identifiant de classement sélectionné ; consultez l’[état des classements](../points/rankings.md).

## Utiliser la bibliothèque Cloud {#use-the-cloud-library}

Lorsque l’hôte expose les capacités d’incrustation Cloud, ouvrez la bibliothèque depuis Incrustations. Parcourez les conceptions locales ou **Mes overlays**, utilisez l’éditeur pour créer une incrustation Cloud, ou ouvrez un identifiant existant et révisez-le. L’éditeur Cloud prend en charge le texte, les champs en direct, les illustrations et les dimensions du canevas ; les téléversements d’images suivent un parcours distinct. L’enregistrement local d’une conception et sa création/révision Cloud sont des opérations distinctes.

## Résultat attendu et état {#expected-result-and-status}

Les conceptions locales, commandes de passerelle et parcours d’éditeur/bibliothèque Cloud conditionnés aux capacités sont disponibles. Aucun adaptateur **Changes** partagé pour les incrustations n’est disponible dans cette version. Validez le design dans OBS et confirmez la sortie LIVE réelle sur le système d’exploitation prévu.

## Récupération {#recovery}

En l’absence de sortie, vérifiez l’état de la passerelle, les ressources, les identifiants secrets, l’URL de source et le type d’événement sélectionné. Pour un conflit de révision Cloud, rechargez la révision actuelle avant de modifier à nouveau. Signalez les erreurs de passerelle via les [diagnostics](../preferences/troubleshooting.md).

<DocCallout title="L’URL de source contient un identifiant secret" tone="warning">

Ne publiez jamais son jeton dans des captures d’écran ou des demandes d’assistance. Utilisez l’identifiant propre au widget de l’URL générée et vérifiez la réception avec un événement anodin sur la source OBS prévue.

</DocCallout>

## Détails de fonctionnement {#operational-details}

Installez le plugin Event Gateway séparément, activez-le et vérifiez son exécution avant d’ajouter des sources OBS LIVE. Les états guident la récupération : missing → installer ; disabled → activer/vérifier la disponibilité ; stopped → démarrer ; credentialUnavailable → redémarrer une fois ; assetsMissing → réparer/reconstruire les ressources de widgets empaquetées ; starting → réessayer brièvement ; unreachable → vérifier le port et le service de bouclage. Un état prêt ou un aperçu local ne prouvent ni un événement LIVE réel ni la réception dans OBS.
