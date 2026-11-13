---
title: "Cycle de vie, propriété et nettoyage"
docId: "developer.lifecycle"
translationKey: "developer.lifecycle"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "92e2164c28be287f8b5d5c96d787a106bb011e1939ff490e4e69737c87bb9716"
translationRevision: "92e2164c28be287f8b5d5c96d787a106bb011e1939ff490e4e69737c87bb9716"
order: 50
---

# Cycle de vie, propriété et nettoyage {#lifecycle-ownership-and-cleanup}

## Objectif et prérequis {#goal-and-prerequisites}

Faire libérer au montage, à la reconnexion et à l’arrêt les ressources qu’ils possèdent. Comprenez l’[architecture](architecture.md) et utilisez un `TIKTOOLS_HOME` isolé pour les expériences natives de cycle de vie. Un désabonnement de l’interface et l’arrêt d’un hôte résolvent des problèmes de propriété différents.

## Suivre la durée de vie du bureau {#follow-the-desktop-lifetime}

`crates/tiktools-desktop/src/app.rs` implémente le gestionnaire d’application Winit. La création de fenêtre et de WebView se fait sur le thread de la boucle d’événements. La fenêtre commence masquée et apparaît après l’envoi de `frontend-ready` par Vue. La fermeture masque la fenêtre lorsqu’une icône de notification existe ; sinon elle déclenche l’arrêt. Quit demande au cœur d’arrêter ses services avant la sortie de la boucle d’événements.

`crates/tiktools-core/src/shutdown.rs` ferme rapidement l’admission au démarrage des plugins, arrête la coordination et la surveillance en arrière-plan, déconnecte le LIVE et vide le travail possédé. Les appels d’arrêt répétés attendent la même fin. Abandonner une future ne peut ni arrêter du code natif arbitraire ni annuler un effet externe.

## Posséder explicitement les ressources de l’interface {#own-frontend-resources-explicitly}

Utilisez le modèle déjà suivi par `use-app-runtime.ts` :

```ts
import { onUnmounted } from 'vue';
import { useControlClient } from '@/shared/api/use-control-client.ts';

const control = useControlClient();
const stop = control.onTopic('points.changed', () => {
  // Actualiser ici la présentation validée du store propriétaire.
});
onUnmounted(stop);
```

Le corps du rappel appartient à l’entité propriétaire de la vue. Une fonctionnalité doit libérer ses abonnements, temporisateurs, trames d’animation et portées de requête, tandis que l’initialisation de l’application possède le rattachement et détachement du transport partagé. Un changement de modèle de widget doit libérer le contrôleur précédent plutôt que de laisser tourner ses temporisateurs d’alerte.

## Résultat attendu et vérification {#expected-result-and-verification}

Quitter une page et y revenir ne multiplie pas les rappels. Une présentation masquée arrête les rendus inutiles sans suspendre la comptabilisation faisant autorité. La reconnexion signale les lacunes et recharge les instantanés ; elle ne rejoue pas les effets métier. Exécutez `bun scripts/task.ts test web cloud-lifecycle` et `bun scripts/task.ts test web request-scope` pour des exemples ciblés.

## Récupération et limites {#recovery-and-limits}

Un démarrage QuickJS dépassant le délai peut rester incertain ; le gestionnaire conserve l’identité de son worker et bloque son remplacement au lieu de prétendre que l’arrêt a réussi. Certains parcours de contrôle synchrones du cycle de vie des plugins peuvent encore bloquer leur appelant. Traitez-les comme des limites d’implémentation, surtout lors du transfert à l’outil de mise à jour. Arrêtez proprement l’hôte propriétaire plutôt que de supprimer son verrou actif. La [source de l’arrêt](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/shutdown.rs) et les tests d’exécution sont des preuves plus solides qu’un schéma historique de cycle de vie.
