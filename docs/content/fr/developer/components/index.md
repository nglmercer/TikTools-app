---
title: "Composants d’interface partagés et composition"
docId: "developer.components.index"
translationKey: "developer.components.index"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "4c70fe5256ba711f6f8fd6c3e8d305c4a44bdd6ba06092a846015e6f1898744f"
translationRevision: "4c70fe5256ba711f6f8fd6c3e8d305c4a44bdd6ba06092a846015e6f1898744f"
order: 290
---

# Composants d’interface partagés et composition {#shared-ui-components-and-composition}

## Objectif et prérequis {#goal-and-prerequisites}

Composer une fonctionnalité à partir du kit d’interface existant en préservant interaction et localisation. Utilisez la chaîne d’outils installée de l’espace Vue et lisez le [plan de l’interface](../frontend.md). Cette page présente le kit ; la source, les props, les tests et la référence dédiée de famille de chaque composant restent l’autorité pour son API exacte.

## Trouver la bonne primitive {#find-the-correct-primitive}

`src/web/shared/ui/` contient boutons, champs texte/date/couleur, zones de texte, cases à cocher, interrupteurs, listes combinées, onglets, tableaux, formulaires, boîtes de dialogue, états, alertes et commandes structurelles. Les jetons de conception se trouvent dans `src/web/app/styles/tokens.css` ; les styles de base et adaptatifs composent l’application. Utilisez les jetons pour les espacements, couleurs, typographie et focus plutôt que de créer un second thème dans chaque fonctionnalité.

1. Trouvez une fonctionnalité existante avec l’interaction souhaitée.
2. Lisez `defineProps`, les slots, les valeurs émises/de modèle et le test de la primitive avant de l’envelopper.
3. Gardez validation métier, état enregistré et appels hôte dans leur fonctionnalité/entité propriétaire.
4. Transmettez les libellés accessibles et attributs de commande via le contrat de transfert pris en charge par la primitive.

Un composant script-setup peut utiliser la véritable API des boutons :

```vue
<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
const { t } = useI18n();
</script>

<template>
  <BaseButton variant="primary" type="submit">{{ t('common.apply') }}</BaseButton>
</template>
```

Le formulaire contenant possède le comportement de soumission et la validation. `BaseButton` prend en charge les variantes `default`, `primary`, `danger` et `ghost` ; une couleur destructive ne constitue pas en elle-même un parcours de confirmation.

## Résultat attendu et état {#expected-result-and-status}

Les primitives partagées offrent des comportements clavier, focus et ARIA cohérents et implémentés. La composition nécessite toujours un examen : les textes indicatifs ne sont pas des libellés, les commandes à icône seule nécessitent un nom, les états désactivés doivent refléter la propriété actuelle de l’opération et les erreurs asynchrones doivent rester visibles. La route de développement du kit d’interface est une démonstration, pas une fonctionnalité produit distincte ni un test de validation serveur.

## Vérifier et récupérer {#verify-and-recover}

Exécutez `bun scripts/task.ts test web BaseButton`, le test ciblé du composant modifié et `bun scripts/task.ts frontend-lint`. Vérifiez la faible largeur, la navigation clavier et les libellés traduits. Si un attribut arrive sur une enveloppe plutôt que sur le champ, examinez `shared/lib/use-split-attrs.ts` et le test de la commande avant de contourner la primitive. Consultez l’[accessibilité](../i18n-accessibility.md) et la [source de l’interface](https://github.com/nglmercer/TikTools-app/tree/remake/src/web/shared/ui) pour adapter un exemple de référence de famille.

## Références interactives des familles {#families}

- [Boutons et actions](buttons.md)
- [Menus](menus.md)
- [TextField et SpinBox](fields.md)
- [ComboBox et SearchSelect](selects.md)
- [Interrupteurs](switches.md)
- [Formulaires](forms.md)
- [Onglets et barres d’outils](tabs.md)
- [Listes et tableaux](collections.md)
- [Filtres et sélection](filters.md)
- [Alertes, états vides et états d’opération](feedback.md)
- [Panneaux et boîtes de dialogue](panels.md)
- [FloatingPanel](floating.md)
- [Splitter et MasterDetail](layouts.md)
- [Vue JSON et différences](json.md)
- [Importation et examen](import-review.md)
- [Modèles d’examen Cloud](cloud-patterns.md)
- [Modèles d’incrustation](overlay-patterns.md)
