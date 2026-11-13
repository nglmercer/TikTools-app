---
title: "Utiliser TikTools"
docId: "user.overview"
translationKey: "user.overview"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "72ce37218d4f3341fccd306bd0b1b4039ec6e1fd3dcfeb16f9096f43576c861d"
translationRevision: "72ce37218d4f3341fccd306bd0b1b4039ec6e1fd3dcfeb16f9096f43576c861d"
order: 0
---

# Utiliser TikTools {#use-tiktools}

TikTools connecte un ordinateur Windows ou Linux à un TikTok LIVE, affiche ses événements, conserve les points des spectateurs et exécute des règles sur votre ordinateur. L’interface de bureau et l’hôte sans interface graphique facultatif partagent le même cœur Rust. Un compte Cloud est distinct de la connexion TikTok.

## Choisir votre première tâche {#choose-your-first-task}

1. Vérifiez les [prérequis](requirements.md) et [installez un binaire publié](installation.md). Rust, Cargo et Bun ne sont pas nécessaires pour utiliser la version de bureau.
2. [Connectez-vous à un LIVE](first-live.md), en commençant sans cookie TikTok lorsque la salle autorise l’accès invité.
3. Utilisez **Live feed** pour examiner les événements. **Points** conserve les totaux des spectateurs ; **Analytics** affiche les statistiques. **Automations** relie des déclencheurs aux actions configurées.
4. Installez des intégrations facultatives depuis **Plugins**. Les paquets de plugins externes sont distribués séparément de l’application.
5. Utilisez **Overlays** pour préparer une source navigateur pour OBS. L’intégration de passerelle est nécessaire pour diffuser les widgets en direct.

## Fonctionnalités disponibles et expérimentales {#available-and-experimental-features}

La navigation principale présente Live feed, Points, Analytics, Connections, Automations, Plugins, Overlays et Settings. Les classements et compétitions disposent d’un moteur natif et d’une route directe, mais leur page de gestion reste masquée dans la navigation habituelle. Ne supposez pas que tous les prototypes décrits dans un ancien document de conception sont des fonctionnalités publiées.

Les changements Cloud des plugins et les Variables examinées utilisent des commandes explicites par section. Automations effectue actuellement une comparaison avec une copie de référence locale ; ce bouton ne publie pas de profils sur Cloud. Les transports de collaboration et certains transferts vers les installateurs restent expérimentaux. Une commande désactivée indique généralement une capacité absente de l’hôte ou un prérequis non satisfait.

## En cas d’échec {#when-something-fails}

Ouvrez **Diagnostics**, conservez la source de l’erreur et l’identifiant de requête, et notez la version installée. Évitez d’inclure des cookies, jetons d’invitation ou valeurs de variables privées. Commencez par la procédure de récupération de chaque guide, puis consultez les guides d’assistance. La démo interactive utilise des données fictives et ne permet pas de vérifier votre compte TikTok, OBS ou votre réseau.

Pour développer le code source, consultez la [documentation de développement](../developer/index.md).
