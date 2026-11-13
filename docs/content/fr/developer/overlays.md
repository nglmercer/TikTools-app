---
title: "SDK des widgets, passerelle et frontières des incrustations"
docId: "developer.overlays"
translationKey: "developer.overlays"
locale: fr
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "99d3351859ddb3a3304133f062f68b8e3130416947fc40433dc9c6bffaf85f6a"
translationRevision: "2211002bf9220b3092af524f9f29298a14f76cb68647bab383e335489e8692b8"
order: 170
---

# SDK des widgets, passerelle et frontières des incrustations {#widget-sdk-gateway-and-overlay-boundaries}

## Objectif et prérequis {#goal-and-prerequisites}

Créer un widget partageant son rendu entre l’aperçu local et la sortie OBS. Utilisez la chaîne Vue installée et comprenez les [événements](events.md). L’aperçu local ne nécessite ni passerelle ni identifiant de compte. La sortie de widgets en direct nécessite le plugin event-gateway installé séparément et son identifiant limité aux widgets.

## Ajouter un modèle {#add-a-template}

1. Définissez un moteur de rendu dans `WidgetStage` ; dimensionnez-le relativement à son parent plutôt qu’à la fenêtre du navigateur.
2. Utilisez `defineWidget` de `src/widgets/sdk/template.ts`. Fournissez son schéma, de nouvelles enveloppes d’exemple et une fabrique `create(search, clock)`.
3. Renvoyez un contrôleur implémentant `handleEnvelope`, `clear` et `dispose`, ainsi que le rendu. Utilisez l’horloge fournie pour que les aperçus puissent maintenir les échantillons et les tests avancer le temps.
4. Enregistrez-le dans `sdk/templates.ts`, puis ajoutez l’entrée autonome, la configuration de compilation, la liste de routes autorisées de la passerelle et l’entrée d’empaquetage. L’enregistrement seul n’expose pas d’URL publique.
5. Utilisez le même `WidgetHost.vue` dans l’éditeur et le bundle autonome ; n’intégrez pas le HTML de la passerelle pour un aperçu local.

```sh
bun scripts/task.ts widgets
bun scripts/task.ts test bun widgets
```

Gardez les jetons de texte, valeurs de conception par défaut et champs modifiables dans le schéma du modèle. Les conceptions se sérialisent en JSON et les URL de widgets copiées portent un instantané validé ; un enregistrement ultérieur exige de copier une URL actualisée dans OBS.

## Protocole de passerelle {#gateway-protocol}

`src/widgets/shared/gateway-client.ts` possède l’authentification, les abonnements, le battement de vie, la reconnexion et le signalement de lacunes. Les widgets consomment des enveloppes métier canoniques, pas TikTok directement. Le point de terminaison des widgets est `/ws/widgets` ; il diffère de l’interface complète du client externe de passerelle. Analysez les enveloppes avec l’analyseur partagé et préservez les frontières des identifiants plutôt que de passer un jeton complet de passerelle par la WebView principale.

## Résultat attendu et état {#expected-result-and-status}

Les aperçus locaux intégrés et le SDK des widgets sont implémentés. L’édition d’incrustation Cloud et le code source `cloud-overlay` existent, mais la validation OBS réelle, la couverture des illustrations/sources en direct et l’application de paquets gardent des limites. Overlay Changes est un travail ultérieur non fusionné. Rankings reste masqué dans la navigation normale malgré l’existence du code du widget de classement.

## Récupération {#recovery}

Pour un aperçu local vide, examinez la durée de vie du contrôleur, les valeurs par défaut du schéma et le dimensionnement de la scène avant d’affaiblir la politique d’origine. Pour la sortie en direct, examinez la santé de la passerelle, l’identifiant de widget et les lacunes explicites. Un test Chromium sans interface ne prouve pas la validation d’une source navigateur OBS. Le [SDK des widgets](https://github.com/nglmercer/TikTools-app/tree/remake/src/widgets/sdk) et les tests du protocole partagé sont les références actuelles d’implémentation.

## Détails de fonctionnement {#operational-details}

La suite unitaire utilise des fixtures ; Chromium avec gateway réel exige assets construits et dépôt externe via `TIKTOOLS_PLUGINS_REPO`. Sans source, elle est ignorée : signalez ce skip. Lancez `bun x --no-install playwright test --config src/widgets/playwright.gateway.config.ts` pour cette frontière. Vérifiez ensuite OBS réel, clé widget, livraison et reprise LIVE ; Chromium headless ne prouve pas l’acceptation OBS/native.
