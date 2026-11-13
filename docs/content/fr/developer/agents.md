---
title: "Agents et SDK Rust typé"
docId: "developer.agents"
translationKey: "developer.agents"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "13e7a36b1ef2b8d3b84834135c04c5a51e7d2b4b403af07ae25f25454de70053"
translationRevision: "7ceea706daa97d96f55a438984ec8b94f82853801b1a6be757d225a27b2c41f3"
order: 80
---

# Agents et SDK Rust typé {#agents-and-the-typed-rust-sdk}

## Objectif et prérequis {#goal-and-prerequisites}

Créer des scripts ou agents utilisant le même hôte que le bureau. Disposez d’un hôte isolé actif, d’une CLI `tiktools` correspondante et d’un accès à la [découverte](control-api.md). Le SDK se trouve dans `crates/tiktools-client` ; il expose des moteurs directs, locaux connectés et distants facultatifs sans introduire un autre cœur métier dans les parcours clients ordinaires.

## Commencer par une découverte en lecture seule {#start-with-read-only-discovery}

```sh
target/debug/tiktools --json system info
target/debug/tiktools api discover
target/debug/tiktools api schema live.status
target/debug/tiktools api call live.status '{}' --format json
```

Déterminez les capacités prises en charge et les schémas de paramètres avant de générer une action. Ne déduisez pas la connexion TikTok, la disponibilité d’OBS ou la permission de modifier à partir de `system.info` seul. `system.doctor` distingue les sondes d’exécution et vérifications ignorées des capacités de compilation.

Une fonction Rust asynchrone peut se rattacher au propriétaire existant :

```rust
async fn inspect() -> Result<(), tiktools_client::ClientError> {
    let client = tiktools_client::TikToolsClient::connect().await?;
    let info = client.system_info().await?;
    println!("{info}");
    Ok(())
}
```

L’appelant fournit son environnement Tokio. Les méthodes typées délèguent aux méthodes RPC enregistrées, et le test d’intégration de parité SDK protège la couverture. Utilisez `call_with_meta` ou `call_value_with_meta` lorsqu’un journal de workflow, d’automatisation ou de propriétaire nécessite des versions et reçus de commandes ; les utilitaires limités au domaine conservent volontairement une forme de retour plus simple.

## Soumettre des changements intentionnels {#submit-intentional-changes}

Pour une opération durable, conservez méthode, paramètres canoniques, identifiant de commande original, instance d’hôte et révision lue exigée par la découverte. Enregistrez la proposition de l’utilisateur avant soumission. Un brouillon d’automatisation distant doit rester explicitement désactivé et appartenir à son auteur ; il attend l’examen du propriétaire et n’est pas autorisé à s’exécuter. L’adoption et l’activation par le propriétaire ont des contrats locaux distincts.

## Résultat attendu, état et récupération {#expected-result-status-and-recovery}

Un agent reçoit les mêmes validations, événements et erreurs que les autres clients. L’accès agent est implémenté ; l’autorité dépend du moteur sélectionné et des autorisations actuelles. Un accusé de réception inconnu exige la recherche du résultat original, pas une resoumission automatique avec un nouvel identifiant. Les résumés distants fermés ne permettent pas de reconstruire une définition complète ; conservez donc la source soumise de manière privée.

Exécutez `bun scripts/task.ts rust test -p tiktools-client --test parity --locked` après l’ajout d’une méthode. Le [code source du SDK](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-client/src/lib.rs) et le registre canonique possèdent les signatures ; les exemples d’agents historiques peuvent être en retard sur les nouveaux coordinateurs de récupération du propriétaire.

## Détails de fonctionnement {#operational-details}

Avant un effet, découvrez le schéma, utilisez `--dry-run --explain` et vérifiez CAS, additivité, durabilité et destruction. Suivez la [recette isolée lecture/CAS/résultat original](control-api.md#operational-details) et [instantané/application/acknowledge](remote-client.md#operational-details). L’adaptateur doit préserver métadonnées, charge/ID original et portée de l’appelant ; changer d’ID après timeout peut répéter l’effet. `--confirm` confirme un appel destructif sans élargir l’autorité.
