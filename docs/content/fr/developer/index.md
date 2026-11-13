---
title: "Développer TikTools"
docId: "developer.overview"
translationKey: "developer.overview"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "1ee65a70172f8154eeaf60fb1a3069192769582a4ef264426d0aefaf346e8003"
translationRevision: "1ee65a70172f8154eeaf60fb1a3069192769582a4ef264426d0aefaf346e8003"
order: 0
---

# Développer TikTools {#develop-tiktools}

TikTools conserve un seul AppCore natif derrière des opérations de contrôle typées. Vue affiche les interfaces de bureau, de navigateur local et de contrôleur ; Winit/Wry possède les fenêtres de bureau. La CLI et le SDK utilisent les mêmes opérations. Commencez par la [configuration de développement](setup.md), puis choisissez la boucle la plus courte couvrant votre modification.

## Choisir un point d’entrée {#choose-an-entry-point}

| Travail | Point d’entrée | Ce qu’il permet de vérifier |
| --- | --- | --- |
| Composants et stores Vue | Vite, Vitest et ControlClient injecté | Comportement de l’interface ; aucune validation de fenêtre native. |
| Intégration de bureau | `bun run start` | Intégration WebView et IPC de développement. |
| Services natifs | Tests Cargo ciblés | Contrats du cœur, persistance et comportement des services. |
| Accès navigateur sans interface native | `bun run dev` | Un hôte natif, un adaptateur local et Vite avec appairage. |
| Documentation | `bun scripts/task.ts docs dev` | Markdown et compilation VitePress isolée. |
| Démo publique | Entrée de démo distincte et transport simulé | Scénarios fictifs déterministes, pas les services distants. |

## Responsabilité du code source {#source-ownership}

- `crates/tiktools-core` : services, opérations et SQLite. Il ne dépend pas d’une WebView.
- `crates/tiktools-control-api` : registre, routage JSON-RPC, IPC et découverte.
- `crates/tiktools-desktop` : Winit/Wry, pont du thread d’interface, cycle de vie et ressources empaquetées.
- `src/web/app` : initialisation, routeur et structure de l’interface. `pages` compose `widgets` et `features` ; `entities` possède les stores/contrats métier ; `shared` contient l’interface réutilisable et l’infrastructure.
- `src/automation/contracts/generated` : contrats d’événements versionnés de l’interface, générés depuis Rust. Ne ressaisissez pas ces schémas dans une démo ou un guide.
- `packages` : paquets TypeScript existants du SDK et de l’interface des plugins. Les implémentations des plugins se trouvent dans le dépôt externe de plugins.

## Limites des contributions {#contribution-boundary}

Gardez explicites les points d’entrée natif, navigateur local, contrôleur distant et démo. L’absence d’hôte natif est une erreur dans un vrai point d’entrée ; elle ne doit pas sélectionner silencieusement un simulacre. Libérez écouteurs, abonnements et temporisateurs avec leur portée propriétaire. Préservez les fichiers de verrouillage Rust/Bun et les contrôles CI requis.

L’installation utilisateur relève du [guide d’utilisation](../user/installation.md). Ces vérifications de développement n’établissent pas à elles seules la connectivité réelle TikTok, le rendu OBS, la compatibilité WebView native ou la collaboration entre deux ordinateurs.
