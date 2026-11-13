---
title: "Réutiliser et examiner les variables d’exécution"
docId: "user.automations.variables"
translationKey: "user.automations.variables"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "20ba9e681b63ffa9aff2ff5d868738d242fd4eab4d520d9e08656760ee8361ba"
translationRevision: "20ba9e681b63ffa9aff2ff5d868738d242fd4eab4d520d9e08656760ee8361ba"
order: 40
---

# Réutiliser et examiner les variables d’exécution {#reuse-and-review-runtime-variables}

## Objectif et prérequis {#goal-and-prerequisites}

Modifier une valeur partagée une seule fois et la réutiliser dans plusieurs actions. Il faut **Automations → Variables** et un hôte prenant en charge les opérations de variables concernées. L’édition typée et les opérations Cloud avec examen apparaissent selon les capacités de l’hôte ; enregistrer localement ne nécessite aucun téléversement.

## Enregistrer une valeur locale {#save-a-local-value}

1. Ajoutez une variable nommée `showLabel`. Les clés commencent par une lettre ou un trait de soulignement et peuvent contenir lettres, chiffres, points, traits de soulignement et traits d’union, jusqu’à `64` caractères. Évitez les doublons.
2. Lorsque l’édition typée est disponible, choisissez le texte et saisissez `Émission du soir`. Les autres types pris en charge sont nombre, booléen, liste et secret. Les listes utilisent un tableau JSON de chaînes, tel que `["rouge", "bleu"]`.
3. Sélectionnez **Save**. Un conflit de révision signifie que la collection enregistrée a changé ; rechargez-la et comparez votre modification prévue à celle-ci.
4. Utilisez la valeur enregistrée dans un champ d’action acceptant les modèles :

```text
{{ globals.showLabel }}: {{ event.user.uniqueId }}
```

5. Testez une action limitée au journal pour examiner le texte rendu. Modifiez `showLabel`, enregistrez à nouveau et lancez un nouveau test. Les définitions d’action existantes peuvent réutiliser la valeur actualisée sans réimporter de modèle.

Les `params` du modèle sont substitués lors de son application ; les `globals` sont lues à l’exécution. Une clé d’exécution manquante produit un texte vide : vérifiez donc son orthographe avant de l’utiliser dans une destination ou un identifiant. Utilisez les suggestions de variables du champ pour choisir les chemins d’événement adaptés à son déclencheur.

## Examiner séparément les changements Cloud {#review-cloud-changes-separately}

La commande **Changes** de Variables dispose d’opérations natives avec examen d’envoi, récupération, application/adoption et retrait lorsqu’elles sont prises en charge. Enregistrez d’abord les modifications locales, sélectionnez une comparaison, examinez son type et sa valeur, puis choisissez explicitement la direction voulue. Récupérer une proposition Cloud est distinct de son application locale. Retirer une valeur partagée est distinct de la suppression d’une ligne locale.

Les valeurs secrètes ou bloquées sont masquées dans les comparaisons et ne peuvent pas être envoyées par ce parcours avec examen. Ne transformez jamais un secret en texte ordinaire pour contourner ce blocage.

## Résultat attendu, état et récupération {#expected-result-status-and-recovery}

Les variables locales et le partage avec examen conditionné aux capacités sont disponibles ; enregistrer localement ne téléverse jamais automatiquement. Les clés invalides, valeurs typées invalides et révisions obsolètes bloquent l’enregistrement. Corrigez le champ signalé ou rechargez avant de réessayer. Pour les erreurs de compte, consultez le [compte Cloud](../cloud/account.md) ; pour les comparaisons, consultez [Changes](../cloud/changes.md).
