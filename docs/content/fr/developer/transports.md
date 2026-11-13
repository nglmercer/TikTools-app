---
title: "Transports actuels et restrictions de plateforme"
docId: "developer.transports"
translationKey: "developer.transports"
locale: fr
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "293f8c00709f3796e5d6bab335df654aef5d9d379c19d622934bf1f8a95591c0"
translationRevision: "293f8c00709f3796e5d6bab335df654aef5d9d379c19d622934bf1f8a95591c0"
order: 200
---

# Transports actuels et restrictions de plateforme {#current-transports-and-platform-restrictions}

## Objectif et prérequis {#goal-and-prerequisites}

Choisir le transport qui existe réellement pour la compilation et la plateforme ciblées. Lisez l’[API de contrôle](control-api.md) et la [collaboration](collaboration.md). Un transport porte des opérations sérialisées ; il ne doit pas introduire un second cœur ou accorder une autorité à lui seul.

## Choisir la frontière {#choose-the-boundary}

| Parcours | Implémentation actuelle | Limite importante |
| --- | --- | --- |
| WebView de bureau | IPC Wry et client de contrôle partagé | La durée de vie de l’interface/des threads appartient au bureau |
| CLI locale | Socket Unix ou tube nommé Windows ; hôte stdio explicite | Même répertoire d’application et même propriétaire |
| Navigateur local | Adaptateur natif de bouclage et `browser-transport.ts` | Hôte natif appairé, pas de simulacre ni de serveur distant public |
| Distant direct | Passerelle HTTPS/WSS authentifiée et transports SDK/navigateur | TLS, hello, autorisations et conditions de fonctionnalités |
| Pair natif | `tiktools-peer`, flux iroh/QUIC chiffré | L’identité du pair ne confère pas l’autorité RPC |
| WebRTC natif | `tiktools-rtc`, signalisation signée et canaux de données | Les vérifications de négociation, autorisation et route restent nécessaires |

L’interface TypeScript de transport se trouve dans `src/web/shared/api/control-transport.ts`. La configuration de pair natif n’active pas implicitement des relais tiers publics ou la découverte. WebRTC est natif Rust (`webrtc` 0.21.1), sans dépendance à `RTCPeerConnection` du navigateur dans l’interface de bureau.

## Examiner la route sélectionnée {#inspect-the-selected-route}

Découvrez les méthodes et capacités prises en charge par l’hôte connecté avant d’appairer ou d’afficher une action de contrôleur :

```sh
tiktools --json system info
tiktools api discover
bun scripts/task.ts test web browser-transport
bun scripts/task.ts test web remote-browser-transport
```

`tiktools-rtc/src/route.rs` observe la paire de candidats ICE sélectionnée pour classer la route comme directe ou relayée. Un état prêt seul ne prouve pas une route directe. L’infrastructure de signalisation/relais Cloud est distincte de l’autorisation métier et doit conserver les vérifications natives d’autorisation.

## Résultat attendu et état {#expected-result-and-status}

L’IPC local et le transport de bureau sont des parcours établis. Le pair natif, WebRTC et l’intégration contrôleur disposent de code et de tests ciblés, mais le déploiement et la validation multi-PC restent expérimentaux. Les anciens textes affirmant l’absence d’implémentation pair/relais sont historiques. L’IPC local Android fonctionne ; les profils distants persistants n’ont pas de coffre d’identifiants sélectionné et les extensions natives/WebViews de bureau y restent indisponibles.

## Récupération {#recovery}

Traitez une déconnexion comme une possible incertitude pour les opérations acceptées. La reconnexion doit invalider les projections privées, renégocier l’identité et reprendre les flux bornés ou en obtenir un instantané. N’affaiblissez jamais les vérifications de certificat et ne vous repliez pas silencieusement vers un propriétaire local après un échec distant. L’[implémentation RTC](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-rtc/src) et le graphe réel des fonctionnalités priment sur les anciens plans de transport.
