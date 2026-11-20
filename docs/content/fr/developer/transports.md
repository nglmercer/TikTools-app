---
title: "Transports actuels et restrictions de plateforme"
docId: "developer.transports"
translationKey: "developer.transports"
locale: fr
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6a5fcd19211bdd279a0696cde31c26cc7444a128e50c3ca752489b25acb7cd9a"
translationRevision: "ba5fe61d36b0954d333cff3b3700fa2901de9349978c051ae84816cbfac1054c"
order: 200
---

# Transports actuels et restrictions de plateforme {#current-transports-and-platform-restrictions}

## Objectif et prérequis {#goal-and-prerequisites}

Choisir le transport qui existe réellement pour la compilation et la plateforme ciblées. Lisez l’[API de contrôle](control-api.md) et la [collaboration](collaboration.md). Un transport porte des opérations sérialisées ; il ne doit pas introduire un second cœur ou accorder une autorité à lui seul.

## Choisir la frontière {#choose-the-boundary}

| Parcours | Implémentation actuelle | Limite importante |
| --- | --- | --- |
| WebView bureau | IPC Wry et client de contrôle partagé | La durée de vie de l’interface et des threads appartient au bureau |
| CLI locale | Socket Unix ou canal nommé Windows ; hôte stdio explicite | Même répertoire et même propriétaire de l’application |
| Navigateur local | Adaptateur loopback natif et `browser-transport.ts` | Hôte natif appairé, pas un serveur distant public |
| Passerelle HTTPS/WSS authentifiée | Transports SDK/navigateur avec TLS, hello, autorisations et feature gates | Les commandes distantes exigent une autorisation explicite |
| Plugin de télécommande installable | `tiktools.remote-control` sur un relais WSS avec `remote.controller.ticket` | Propriété du plugin ; aucun code distant n’est compilé dans l’hôte |

L’interface TypeScript se trouve dans `src/web/shared/api/control-transport.ts`. La télécommande est un plugin installable, et non une partie du binaire hôte : chaque commande passe par `HostCallPolicy` (`remote.command.dispatch`, `remote.pair.list`, `remote.grant.list`, `remote.grant.revoke`, `remote.controller.ticket`). Cette compilation ne contient ni transport pair/RTC natif ni lien hôte Cloud Teams.

## Inspecter la route sélectionnée {#inspect-the-selected-route}

Découvrez les méthodes et capacités prises en charge par l’hôte connecté avant l’appairage ou l’affichage d’une action de contrôle :

```sh
tiktools --json system info
tiktools api discover
bun scripts/task.ts test web browser-transport
```

L’appairage et les autorisations appartiennent au plugin : le ticket de contrôleur est de courte durée et à usage unique, et `remote.grant.list` / `remote.grant.revoke` font foi. Une connexion relais prouve seulement le transport, jamais l’autorité RPC.

## Résultat attendu et statut {#expected-result-and-status}

L’IPC local et le transport bureau sont des chemins établis. La télécommande fonctionne via le plugin installable et un relais authentifié ; ce n’est pas un second runtime hôte. Les codes de connexion sont relayés vers les appareils connectés avec une durée de vie de cinq minutes, avec OAuth en repli lorsqu’aucun compte n’est configuré. L’IPC local Android fonctionne avec les outils instrumentés `mobile` (`info` / `exec` / `tty`) ; les addons natifs et WebViews de bureau y restent indisponibles.

## Récupération {#recovery}

Considérez la déconnexion comme une incertitude possible pour les opérations acceptées. La reconnexion doit invalider les projections privées, renégocier l’identité et reprendre ou figer les flux bornés. Ne réduisez jamais les vérifications de certificat et ne revenez pas silencieusement à un propriétaire local après un échec distant. Les tickets expirés et les autorisations révoquées doivent échouer en fermeture.

## Détails opérationnels {#operational-details}

Gardez les autorisations au minimum privilège et à durée limitée ; vérifiez-les à chaque reconnexion. Diagnostiquez séparément l’identité, l’expiration du ticket, la portée de l’autorisation, l’accessibilité du relais et la taille des trames. Les trames trop grandes échouent à la validation du cadrage plutôt que de devenir des RPC partielles. Testez les sessions modifiées ou révoquées avec des fixtures avant une acceptation réelle sur deux appareils.
