---
title: "Lire et examiner le flux LIVE"
docId: "user.live.feed"
translationKey: "user.live.feed"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "8589ccb9d2f5cc646ae0bf9ac5164b90ba7cc73e4bcc819535fc59e0752fe30d"
translationRevision: "8589ccb9d2f5cc646ae0bf9ac5164b90ba7cc73e4bcc819535fc59e0752fe30d"
order: 20
---

# Lire et examiner le flux LIVE {#read-and-inspect-the-live-feed}

## Objectif et prérequis {#goal-and-prerequisites}

Trouver un événement, examiner ses champs et distinguer un affichage en pause d’une salle déconnectée. Commencez avec une [connexion LIVE](connections.md), ou utilisez les événements déjà présents dans le flux actuel. Le flux est une vue de travail bornée, pas un enregistrement complet d’une diffusion.

## Examiner l’activité {#inspect-activity}

1. Ouvrez **Live feed** et confirmez le créateur actif dans l’état de connexion.
2. Choisissez un filtre d’événements : tous, discussions, cadeaux, mentions J’aime, abonnements, arrivées ou autres activités sociales. Combinez-le avec la recherche pour trouver un auteur, un message, un nom de cadeau ou un identifiant exact de cadeau.
3. Sélectionnez **Pause** pour arrêter le défilement automatique pendant la lecture. Les événements peuvent continuer à arriver et le compteur de non lus augmenter. Cette commande ne déconnecte pas TikTok et ne suspend pas les automatisations.
4. Sélectionnez un événement pour ouvrir son inspecteur. Lisez son type et les détails disponibles avant de choisir les champs d’automatisation. Tous les événements ne contiennent pas de texte de discussion, d’informations de cadeau ou d’identité de spectateur.
5. Sélectionnez le nom ou l’avatar d’un spectateur pour examiner son profil suivi. Utilisez la commande de l’inspecteur si le panneau de détails est masqué.
6. Sélectionnez **Resume** ou le bouton d’accès aux événements non lus pour revenir à l’activité entrante. Effacez la recherche et choisissez tous les événements pour vérifier que de l’activité arrive.

Utilisez autant que possible les commandes libellées avec le focus clavier. Suspendre le défilement aide lorsque le mouvement gêne la lecture. Un avatar absent est remplacé par des initiales ; une image n’est pas nécessaire pour identifier un spectateur.

## Résultat attendu et état {#expected-result-and-status}

Le filtrage et l’inspection sont disponibles. Lors d’un trafic important, l’interface peut afficher une fenêtre de rendu limitée et signaler des événements d’interface abandonnés. Ces indicateurs concernent l’affichage ; n’utilisez pas le nombre de lignes visibles comme total comptable. [Analytics](../points/statistics.md) interroge séparément les agrégats persistants.

## Récupération {#recovery}

Si le flux est vide, retirez les filtres, reprenez le défilement et vérifiez l’état de connexion. Une salle calme peut fonctionner normalement. La reconnexion au même créateur conserve le contexte ; changer de créateur le modifie. **Clear** vide le flux de travail sans réinitialiser les points ou les statistiques. Examinez les indicateurs de perte via les [diagnostics](../preferences/troubleshooting.md), sans supposer que toutes les lignes manquantes à l’affichage sont des événements distants perdus.

<DemoFrame scenario="feed" locale="fr" title="Démo : flux" />
