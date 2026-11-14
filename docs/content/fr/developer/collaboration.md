---
title: "Autorité et authentification de collaboration"
docId: "developer.collaboration"
translationKey: "developer.collaboration"
locale: fr
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fb71673a581e923e3c02ae401a46937b007f9919690ca020d22c27b370b6c61d"
translationRevision: "12ac7b010630ad128b026385ded2f897811b484b9ab46c73b84462048134d842"
order: 190
---

# Autorité et authentification de collaboration {#collaboration-authority-and-authentication}

## Objectif et prérequis {#goal-and-prerequisites}

Permettre à un appareil approuvé d’examiner ou d’effectuer une opération précisément délimitée sur un hôte existant. Utilisez un profil propriétaire de test, une compilation avec prise en charge distante et un transport configuré. La connexion au compte, l’appartenance à une équipe Cloud, la propriété locale et une session déléguée directe sont des autorités distinctes.

## Établir la chaîne de confiance {#establish-the-trust-chain}

L’administration `collaboration.*` du propriétaire local possède configuration, invitations, demandes en attente, principaux, autorisations, sessions et audit. Découvrez les schémas avant de configurer un écouteur ou de créer des identifiants :

```sh
tiktools api schema collaboration.configure
tiktools api schema collaboration.invites.create
tiktools api call collaboration.status '{}' --format json
```

Une invitation lance l’appairage ; elle n’accorde pas à elle seule l’autorité métier. L’approbation du propriétaire crée la session/les autorisations appropriées. HTTPS direct exige des certificats validés ou une confiance native explicitement épinglée. Les frontières des jetons porteurs natifs et des cookies navigateur diffèrent. Le `collaboration.hello` initial lie protocole, instance d’hôte, principal, session, révision d’autorisation, capacités et limites négociées.

## Autoriser chaque opération {#authorize-each-operation}

Une méthode distante prise en charge doit passer ses vérifications de liste autorisée, capacité, restriction de ressource et session actuelle. La désactivation/suppression de workflow utilise des métadonnées durables de comparaison et modification atomiques. Les ajustements de points sont additifs et possèdent leurs propres limites d’autorisation. Les lectures d’automatisation distantes sont des résumés fermés ; les brouillons désactivés appartenant à leur auteur n’acquièrent pas l’autorité exécutable du propriétaire simplement en étant enregistrés.

L’examen/adoption par le propriétaire et l’approbation vérifiée lient les définitions, dépendances et provenances actuelles. Un manifeste d’aperçu rapporte des données d’inspection, pas un jeton d’exécution. Revalidez les autorisations et liaisons de ressources aux frontières d’exécution plutôt que de traiter l’identité de transport comme une permission permanente.

## Résultat attendu et état {#expected-result-and-status}

La collaboration native directe implémente des lectures délimitées, certaines mutations durables et une récupération bornée. Les équipes/relais Cloud ont en plus leurs propres adaptateurs ; le parcours de lecture du relais Cloud documenté ne doit pas être présenté comme une modération complète ou l’exécution de brouillons. Les effets délégués, écritures métier et validations multi-PC restants demeurent expérimentaux. Consultez les [clients distants](remote-client.md) et [Cloud](cloud.md) pour le parcours sélectionné.

## Récupérer et vérifier {#recover-and-verify}

À la révocation ou au changement d’identité, arrêtez les abonnements et supprimez les projections privées. Utilisez `collaboration.subscribe`, `snapshot` et `resume` selon les domaines négociés et la validité du curseur. Résolvez l’identifiant de commande original après un accusé incertain. Ne réessayez pas une suppression avec un nouvel identifiant et ne restaurez pas une autorité depuis un ancien reçu. La [source d’autorisation](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-collaboration/src) et la politique du registre définissent la frontière ; réussir TLS seul n’est pas un test d’autorisation.

## Détails de fonctionnement {#operational-details}

Séquence Teams : enregistrez l’appareil, émettez/rejoignez une invitation ou demande, approuvez puis faites confirmer par l’hôte la révision actuelle de permission native. `pendingHost` indique cette dernière étape. Une nouvelle révision invalide la confirmation ; rechargez si expectedRevision est périmée. L’acceptation native exige instantané récent, hôte/appareil actifs, permission active et confirmedRevision égale à revision. L’UI expose panneaux hôte/jonction et approbations disponibles ; la confirmation native est un contrat API, pas un bouton garanti partout.

Conversion héritée fermée : observer lit live/workflows/points/compétitions/automatisation/résultats ; moderator ajoute ajustement de points, toggles workflow/automatisation et gestion des compétitions ; automationEditor ajoute édition aux lectures. Aucun preset n’implique scripts arbitraires, effets externes ou délégation réutilisable. `nativeGrant` explicite exige `custom` ; limitez ressources, effets et delta. Client natif : `desktop` ou `agent` seulement. Une politique non supportée échoue sans élargir les droits. Diagnostiquez pairing, confirmation, révision et transport séparément.
