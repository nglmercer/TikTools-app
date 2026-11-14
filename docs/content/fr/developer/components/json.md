---
title: "Vue JSON et différences"
docId: "developer.components.json"
translationKey: "developer.components.json"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "f9d8877d583d78c94a8a46a2beb25bc623dd606706f219eef87285c0a249d6a8"
translationRevision: "68f02c99743c8c56c62788363018b1cb71d2b7b8e07e31977a9fbcb6bcdfc75f"
---

# Vue JSON et différences {#json-view-and-diff}

## Objectif et prérequis {#purpose}

Examinez du JSON borné sous forme de code ou d’arbre et comparez les changements. Les valeurs sont échappées ; les grands documents sont tronqués avec un avis visible. Utilisez l’espace de travail Vue existant et son fournisseur i18n. L’aperçu monte les vrais composants partagés dans un cadre applicatif isolé ; le thème documentaire ne charge pas le CSS global du bureau.

## Essayer l’exemple {#example}

1. Chargez l’aperçu ou ouvrez la démo complète.
2. Modifiez le sélecteur d’état pour examiner les présentations de chargement, vide, erreur, désactivée, texte long et mobile selon leur pertinence. Un état sans rendu distinct n’ajoute pas de comportement artificiel au composant.
3. Utilisez Tab, Entrée, Échap et les flèches selon la commande. Vérifiez son nom accessible, le focus visible et le retour du focus à la fermeture d’une fenêtre contextuelle.
4. Modifiez une valeur ou invoquez son action et examinez la sélection ou la sortie résultante. L’actualisation ou Reset restaure l’état de la démo.

<DemoFrame scenario="settings" locale="fr" route="/components/json" title="Exemple : Vue JSON et différences" />

## API {#api}

Les props, valeurs par défaut, événements, slots et déclarations de modèle suivants sont générés depuis la source du composant. Les types importés restent liés à cette source au lieu d’être dupliqués manuellement. Les attributs non déclarés suivent le contrat de transfert du composant ; examinez sa source avant d’attacher un événement natif à une enveloppe.

<!--@include: ../../../../.generated/components/json.fr.md-->

## Source exécutable {#source}

Ce fichier exact est à la fois compilé pour l’aperçu et inclus comme extrait. Son petit état appartient à l’exemple. La logique de compte, de plugin et d’hôte reste dans features/entities plutôt que dans l’interface partagée.

<<< ../../../../examples/json.vue

## Accessibilité et récupération {#accessibility}

Utilisez un libellé visible ou aria-label, préservez le fonctionnement au clavier et évitez de vous fier uniquement à la couleur. Testez à faible largeur et avec de longs libellés traduits. Pour les formulaires, liez les controlAttrs de la ligne au véritable champ pour que les erreurs et l’état requis restent associés. Une commande désactivée ne peut pas expliquer seule un échec ; affichez un état adjacent et une action de récupération.

Si un aperçu échoue, fermez-le et rouvrez-le, puis utilisez Reset pour sa propre instance de démo. Examinez l’API statique et la source même lorsque la démo est indisponible. Les boîtes de dialogue natives, services externes et intégrations système nécessitent des tests de bureau distincts. [Revenir au guide des composants](index.md).
