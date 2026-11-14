---
title: "Agents et SDK Rust typé"
docId: "developer.agents"
translationKey: "developer.agents"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fe3c629f94372134069e2f5316f807214666ce435d135cb45c0eba72054c5839"
translationRevision: "fe3c629f94372134069e2f5316f807214666ce435d135cb45c0eba72054c5839"
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

## Teams headless pour les agents {#headless-teams}

Compilez le CLI avec le support hôte ou utilisez le CLI de même version inclus dans la distribution portable. Lancez le propriétaire comme processus persistant avec un `TIKTOOLS_HOME` absolu et isolé. Les commandes d’un autre terminal doivent utiliser ce même répertoire.

```sh
cargo build -p tiktools-cli --release --locked --features remote-control
TIKTOOLS_HOME=/absolute/test-owner target/release/tiktools host --ipc
```

`cloud login` authentifie le runtime propriétaire. `remote cloud login` authentifie le contexte controller, dont les métadonnées de compte et les clés de périphérique restent séparées. Les deux utilisent le véritable transfert par navigateur ; la fenêtre du bureau est inutile. `--non-interactive` ne lance jamais de connexion et renvoie `authentication_required` si les identifiants manquent.

Le propriétaire crée l’équipe, inscrit son périphérique hôte et démarre le service. Utilisez `teams people search NICKNAME` pour découvrir l’ID du destinataire et créer une invitation qui lui est liée. Le jeton est écrit une seule fois dans un nouveau fichier privé ; stdout contient uniquement les métadonnées. Approuvez explicitement la demande et configurez les droits natifs de l’agent avant la connexion.

```sh
tiktools --json teams create --name "Studio" --confirm
tiktools --json teams host enroll --team TEAM_ID --label "Host" --confirm
tiktools --json teams host start --wait --timeout 30 --confirm
tiktools --json teams invitations create --team TEAM_ID --recipient RECIPIENT_ID --role observer --output-file /private/invite --confirm
```

Pour un accès réutilisable, utilisez `remote cloud connect --host ID --profile agent --kind agent --wait --non-interactive --timeout 30` après approbation. `teams list --joined` consulte les appartenances du controller ; `teams list --owned` les équipes du propriétaire. `teams doctor` inspecte le propriétaire ; `teams doctor --context controller` inspecte l’authentification et les appartenances du controller. `remote check agent` vérifie une véritable connexion.

```sh
tiktools --json teams join --profile agent --kind agent --invite-stdin --non-interactive < /private/invite
tiktools --json teams requests list --team TEAM_ID
tiktools --json teams requests approve --team TEAM_ID --request REQUEST_ID --confirm
tiktools --json teams grants list --team TEAM_ID
tiktools --json teams grants update --team TEAM_ID --grant GRANT_ID --expected-revision REVISION --role custom --policy @agent-policy.json --confirm
tiktools remote serve --profile agent --stdio --events --timeout 30
```

`agent-policy.json`:

<<< ../../../examples/agent-policy.json

```json
{"jsonrpc":"2.0","id":"agent-1","method":"live.status","params":{}}
```

Le client persistant accepte une requête JSON-RPC par ligne et conserve les IDs. Les écritures exigent `confirm: true` ; les écritures durables nécessitent aussi leurs métadonnées documentées de commande et d’hôte. EOF, terminaison ou fermeture de connexion libèrent le client. La taille et la durée des requêtes sont bornées ; une écriture expirée peut néanmoins avoir été validée.

Le test d’acceptation utilise un CLI compilé, un véritable backend jetable, des comptes synthétiques et des processus séparés propriétaire/controller. Fournissez un checkout RemakeBackend contenant la fixture CLI et ses outils pgrust installés. Les binaires de développement acceptent l’origine isolée au runtime. Les releases conservent leur origine de compilation : compilez une release de staging pour un port TLS loopback fixe et passez `--cloud-port PORT`. Les binaires liés à la production sont refusés avant authentification. Aucun workflow distant n’est lancé.

```sh
bun scripts/task.ts test headless-teams --binary /absolute/path/tiktools --backend /absolute/path/RemakeBackend --report /tmp/headless-teams.json
```

Les nouveaux paquets acceptent aussi `./TikTools.AppImage --cli --build-info` et `./TikTools.AppImage --cli --json teams doctor`. Cette entrée agit avant l’initialisation graphique ; les AppImages déjà publiées ne reçoivent pas automatiquement cette fonction.

La réutilisation d’un profil controller entre processus CLI nécessite un magasin d’identifiants du système disponible.
