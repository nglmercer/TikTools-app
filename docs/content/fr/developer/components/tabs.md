---
title: "Onglets et barres d’outils"
docId: "developer.components.tabs"
translationKey: "developer.components.tabs"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6341644dd3da8b4d7a5ef9a079a884908997a5a3f8f65618746cdb8d9db81ad8"
translationRevision: "5d8ffb57e09d4ab865b312137a956f74b006f5fd039101f92f1e88b48b779970"
---

# Onglets et barres d’outils {#tabs-and-toolbars}

## Objectif et prérequis {#purpose}

Passez d’une vue liée à une autre sans perdre les brouillons. Donnez des libellés accessibles aux groupes d’onglets et gardez les commandes de barre d’outils accessibles au clavier. Utilisez l’espace de travail Vue existant et son fournisseur i18n. L’aperçu monte les vrais composants partagés dans un cadre applicatif isolé ; le thème documentaire ne charge pas le CSS global du bureau.

## Essayer l’exemple {#example}

1. Chargez l’aperçu ou ouvrez la démo complète.
2. Modifiez le sélecteur d’état pour examiner les présentations de chargement, vide, erreur, désactivée, texte long et mobile selon leur pertinence. Un état sans rendu distinct n’ajoute pas de comportement artificiel au composant.
3. Utilisez Tab, Entrée, Échap et les flèches selon la commande. Vérifiez son nom accessible, le focus visible et le retour du focus à la fermeture d’une fenêtre contextuelle.
4. Modifiez une valeur ou invoquez son action et examinez la sélection ou la sortie résultante. L’actualisation ou Reset restaure l’état de la démo.

<DemoFrame scenario="settings" locale="fr" route="/components/tabs" title="Exemple : Onglets et barres d’outils" />

## API {#api}

Les props, valeurs par défaut, événements, slots et déclarations de modèle suivants sont générés depuis la source du composant. Les types importés restent liés à cette source au lieu d’être dupliqués manuellement. Les attributs non déclarés suivent le contrat de transfert du composant ; examinez sa source avant d’attacher un événement natif à une enveloppe.

<!--@include: ../../../../.generated/components/tabs.fr.md-->

## Source exécutable {#source}

Ce fichier exact est à la fois compilé pour l’aperçu et inclus comme extrait. Son petit état appartient à l’exemple. La logique de compte, de plugin et d’hôte reste dans features/entities plutôt que dans l’interface partagée.

<<< ../../../../examples/tabs.vue

## Accessibilité et récupération {#accessibility}

Utilisez un libellé visible ou aria-label, préservez le fonctionnement au clavier et évitez de vous fier uniquement à la couleur. Testez à faible largeur et avec de longs libellés traduits. Pour les formulaires, liez les controlAttrs de la ligne au véritable champ pour que les erreurs et l’état requis restent associés. Une commande désactivée ne peut pas expliquer seule un échec ; affichez un état adjacent et une action de récupération.

Si un aperçu échoue, fermez-le et rouvrez-le, puis utilisez Reset pour sa propre instance de démo. Examinez l’API statique et la source même lorsque la démo est indisponible. Les boîtes de dialogue natives, services externes et intégrations système nécessitent des tests de bureau distincts. [Revenir au guide des composants](index.md).

## Détails de fonctionnement {#operational-details}

L’exemple garde l’état dans le parent. Le modèle sort trie trois lignes par computed ; l’en-tête change l’ordre sans changer l’ID sélectionné. Les éditeurs gardent leurs brouillons hors de l’enfant avec key : saisir, changer puis revenir les conserve. Onglet/dialogue ne les persistent pas. Choisissez explicitement état parent, contenu monté ou KeepAlive.

## Détails de fonctionnement {#keyboard-contract}

Consultez [clavier, exceptions du dialogue et noms/tooltip](../i18n-accessibility.md#operational-details). Vérifiez focus initial/retour dans le flux complet, y compris dialogues non fermables et éditeurs démontés.
