---
title: "Se connecter à son compte Cloud"
docId: "user.cloud.account"
translationKey: "user.cloud.account"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a35f6efcbba515b60b6f0c3d3bda6fd0224a993585776873fd02afa4e7202a61"
translationRevision: "a35f6efcbba515b60b6f0c3d3bda6fd0224a993585776873fd02afa4e7202a61"
order: 10
---

# Se connecter à son compte Cloud {#sign-in-to-your-cloud-account}

## Objectif et prérequis {#goal-and-prerequisites}

Connecter l’application de bureau à son service Cloud configuré pour les opérations de catalogue et de collaboration liées au compte. Il faut un accès réseau, un navigateur système et un coffre d’identifiants du système d’exploitation fonctionnel. L’édition locale reste utile sans Cloud ; une session Cloud n’authentifie pas votre connexion TikTok LIVE.

## Se connecter et vérifier l’identité {#sign-in-and-verify-identity}

1. Ouvrez **Settings → Account**. Lisez l’état actuel avant de démarrer une nouvelle connexion.
2. Sélectionnez **Sign in**. Le bureau démarre un récepteur de rappel local et ouvre la page de connexion configurée dans le navigateur système.
3. Terminez le parcours de connexion du service et revenez à TikTools. Si le navigateur ne s’est pas ouvert, utilisez l’URL de connexion affichée par l’application pour cette tentative. Traitez cette URL comme privée et ne la publiez pas.
4. Attendez que l’état du compte indique une connexion établie. Confirmez le nom ou l’adresse électronique affichés avant d’ouvrir un catalogue privé ou une équipe.
5. Ouvrez la section des diagnostics de Account si l’état signale une erreur. Utilisez **Cancel** pour abandonner une tentative inachevée plutôt que de laisser plusieurs tentatives actives.
6. Après utilisation d’un ordinateur partagé, sélectionnez **Sign out** et vérifiez le changement d’état du compte.

L’hôte natif gère le jeton de session. Il stocke les identifiants via le coffre du système d’exploitation et ne conserve dans l’interface qu’un état non secret. Aucun repli vers un fichier en clair n’existe lorsque le stockage sécurisé est indisponible. Un dossier de données copié ne constitue pas une session connectée portable.

## Résultat attendu et état {#expected-result-and-status}

La connexion native, la restauration et la déconnexion sont disponibles dans les hôtes compilés avec l’authentification Cloud. Un compte configuré peut être restauré au redémarrage et revalidé auprès du service. Un hôte non configuré ou non pris en charge ne se corrige pas en saisissant un autre cookie TikTok.

La disponibilité de la connexion au service et la validation réelle du fournisseur/de la plateforme dépendent du service déployé et de votre machine. Une validation locale ne prouve pas que tous les parcours de connexion en production fonctionnent.

## Récupération {#recovery}

Pour une tentative annulée ou expirée, lancez une nouvelle connexion après avoir fermé l’ancienne. En cas d’erreur réseau ou de service, conservez le code de diagnostic et réessayez après rétablissement. Pour une erreur de coffre d’identifiants, rétablissez le stockage sécurisé du système ; ne collez pas de jeton porteur dans les paramètres ou un message d’assistance. Consultez [Changes](changes.md) pour la synchronisation explicite et la [collaboration](collaboration.md) pour les autorisations d’accès distinctes.
