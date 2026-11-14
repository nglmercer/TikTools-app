---
title: "ComboBox et SearchSelect"
docId: "developer.components.selects"
translationKey: "developer.components.selects"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6dc5181ece21f73c6569b5d8e54c896c6df86390333a5240ef0696257e5e56d5"
translationRevision: "d696706e290bcba455fcc08c60da2a141f9bda5eabc6572080b9f3643257343d"
---

# ComboBox et SearchSelect {#combobox-and-searchselect}

## Objectif et prérequis {#purpose}

Choisissez une valeur stable parmi des options libellées. SearchSelect recherche libellés et valeurs ; une valeur sélectionnée inconnue est affichée telle quelle. Utilisez l’espace de travail Vue existant et son fournisseur i18n. L’aperçu monte les vrais composants partagés dans un cadre applicatif isolé ; le thème documentaire ne charge pas le CSS global du bureau.

## Essayer l’exemple {#example}

1. Chargez l’aperçu ou ouvrez la démo complète.
2. Modifiez le sélecteur d’état pour examiner les présentations de chargement, vide, erreur, désactivée, texte long et mobile selon leur pertinence. Un état sans rendu distinct n’ajoute pas de comportement artificiel au composant.
3. Utilisez Tab, Entrée, Échap et les flèches selon la commande. Vérifiez son nom accessible, le focus visible et le retour du focus à la fermeture d’une fenêtre contextuelle.
4. Modifiez une valeur ou invoquez son action et examinez la sélection ou la sortie résultante. L’actualisation ou Reset restaure l’état de la démo.

<DemoFrame scenario="settings" locale="fr" route="/components/selects" title="Exemple : ComboBox et SearchSelect" />

## API {#api}

Les props, valeurs par défaut, événements, slots et déclarations de modèle suivants sont générés depuis la source du composant. Les types importés restent liés à cette source au lieu d’être dupliqués manuellement. Les attributs non déclarés suivent le contrat de transfert du composant ; examinez sa source avant d’attacher un événement natif à une enveloppe.

<!--@include: ../../../../.generated/components/selects.fr.md-->

## Source exécutable {#source}

Ce fichier exact est à la fois compilé pour l’aperçu et inclus comme extrait. Son petit état appartient à l’exemple. La logique de compte, de plugin et d’hôte reste dans features/entities plutôt que dans l’interface partagée.

<<< ../../../../examples/selects.vue

## Accessibilité et récupération {#accessibility}

Utilisez un libellé visible ou aria-label, préservez le fonctionnement au clavier et évitez de vous fier uniquement à la couleur. Testez à faible largeur et avec de longs libellés traduits. Pour les formulaires, liez les controlAttrs de la ligne au véritable champ pour que les erreurs et l’état requis restent associés. Une commande désactivée ne peut pas expliquer seule un échec ; affichez un état adjacent et une action de récupération.

Si un aperçu échoue, fermez-le et rouvrez-le, puis utilisez Reset pour sa propre instance de démo. Examinez l’API statique et la source même lorsque la démo est indisponible. Les boîtes de dialogue natives, services externes et intégrations système nécessitent des tests de bureau distincts. [Revenir au guide des composants](index.md).

## Détails de fonctionnement {#operational-details}

Fournissez l’option sélectionnée pendant le chargement pour afficher son libellé ; sinon la valeur brute apparaît. `search` émet la requête pour une recherche distante facultative du propriétaire, avec filtrage interne libellé/valeur. Flèches/Home/End parcourent les options actives ; Entrée choisit la ligne. Avec `allowCustom`, la confirmation rogne le texte ; Échap annule. Testez ID non chargé, options désactivées et valeur personnalisée.

## Détails de fonctionnement {#keyboard-contract}

Consultez [clavier, exceptions du dialogue et noms/tooltip](../i18n-accessibility.md#operational-details). Vérifiez focus initial/retour dans le flux complet, y compris dialogues non fermables et éditeurs démontés.
