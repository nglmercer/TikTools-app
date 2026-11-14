---
title: "Clients distants et récupération"
docId: "developer.remote-client"
translationKey: "developer.remote-client"
locale: fr
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "3fa6b05b6c19548fa64d45b3114bfd8080f3f0ed510f835c4d18297d9ccd6cae"
translationRevision: "69d12ca6768a7e299399e92d57279dce46cf8830a7396e51e6b74ce8a98c363c"
order: 90
---

# Clients distants et récupération {#remote-clients-and-recovery}

## Objectif et prérequis {#goal-and-prerequisites}

Connecter un contrôleur à un hôte sans créer de cœur LIVE local. Il faut un hôte dont les fonctions distantes concernées sont activées, une invitation approuvée par le propriétaire, une origine HTTPS accessible et un coffre d’identifiants sécurisé pris en charge. Lisez la [collaboration](collaboration.md) pour les autorisations et les [transports](transports.md) pour les routes natives.

## Appairer et examiner {#pair-and-inspect}

Ces commandes décrivent un environnement de test manuel ; remplacez l’origine d’exemple par celle de l’hôte configuré :

```sh
tiktools remote inspect --url https://host.example
tiktools remote pair --url https://host.example --profile live-desk
tiktools --remote-profile live-desk api discover
tiktools --remote-profile live-desk live status
tiktools --remote-profile live-desk workflow list
```

Pour un hôte autosigné, comparez son empreinte avec le propriétaire par un canal indépendant et passez `--fingerprint` pendant l’appairage. L’inspection n’envoie aucune requête métier authentifiée et n’approuve pas la confiance. Les valeurs d’invitation et d’identifiants sont saisies via les invites prises en charge, pas comme secrets en ligne de commande.

Les fichiers de profil contiennent l’identité, la politique de confiance et une référence opaque au stockage sécurisé. Linux utilise Secret Service, macOS Keychain et Windows Credential Manager. Un stockage indisponible échoue explicitement ; Android n’a pas de moteur sélectionné pour les profils persistants. Le SDK peut utiliser des identifiants en mémoire fournis explicitement. `remote forget` supprime les identifiants et métadonnées locaux mais ne révoque pas la session de l’hôte.

## Préserver l’identité des commandes {#preserve-command-identity}

Lisez une ressource avec ses métadonnées avant une mutation prise en charge. Conservez sa révision de base, l’instance d’hôte et l’identifiant de commande durable. Si la connexion disparaît après soumission, interrogez `collaboration.commands.get` avec cet identifiant original. Un reçu est historique ; actualisez séparément la ressource. La désactivation/suppression de workflows, les ajustements de points et les brouillons d’automatisation désactivés appartenant à l’auteur ont des contrats et autorisations différents.

## Résultat attendu et état {#expected-result-and-status}

Le bureau natif dispose d’une entrée isolée `tiktools-desktop --controller` et de journaux liés au profil. La couverture du contrôleur reste expérimentale. L’appairage, la connexion de transport ou un indicateur de fonctionnalité large n’autorise pas des modifications arbitraires. Les lectures d’automatisation distantes exposent des résumés fermés, jamais la source modifiable ; l’activation/déclenchement distant et l’adoption locale par le propriétaire restent distincts.

## Récupérer {#recover}

En cas de lacune de séquence, utilisez le parcours négocié d’instantané/reprise d’abonnement et effacez les projections privées lors d’un changement d’identité. Ne rejouez pas automatiquement les mutations métier après reconnexion. Un certificat modifié exige un examen explicite de confiance. L’[implémentation des profils](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-client/src/remote/profiles/mod.rs) documente les baux de profils et l’incertitude de renouvellement ; la validation réelle multi-PC et des coffres système doit être rapportée séparément des tests de bouclage.

## Détails de fonctionnement {#operational-details}

Appliquez chaque événement fiable à l’instantané/projection correspondant avant `acknowledge` ; confirmer avant perd la preuve de récupération. Reprenez seulement avec les mêmes instance, abonnement, session et permissions. Lacune, contexte changé, curseur refusé ou fermeture du navigateur après gap imposent de nouveaux instantanés autorisés puis un abonnement. Ne rejouez pas de mutations depuis l’historique. Utilisez la [recette métadonnées/ID original](control-api.md#operational-details) pour les écritures incertaines.

L’`event` local contient `{topic,data?}` ; une variante unitaire peut omettre les données. À distance, `event.params` est ReliableEvent avec séquence textuelle, ID, version de ressource et acteur. `event.gap` est une méthode distincte. Ces enveloppes sont illustratives, sans replay durable des effets perdus.

```json
{"jsonrpc":"2.0","method":"event","params":{"topic":"live.status","data":{}}}
{"jsonrpc":"2.0","method":"event","params":{"topic":"workflow.changed","data":{},"hostInstanceId":"h1","subscriptionId":"s1","sequence":"7","eventId":"e7","resource":{"kind":"workflow","id":"docs-example","scope":"host","revision":"2"},"actor":{"principalId":"owner"}}}
{"jsonrpc":"2.0","method":"event.gap","params":{"lost":3,"resync":true}}
```

Le schéma SDK suivant exige remote-profiles. Les callbacks de projection appartiennent à l’application et terminent avant acknowledge. Voir [RecoveryStream](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-client/src/remote/recovery.rs) pour baseline, curseur et reprise typés.

```rust
use tiktools_client::remote::{RecoveryMode, RecoveryStream, StreamDomain};
use std::collections::BTreeSet;

let mut stream = RecoveryStream::start(remote, BTreeSet::from([StreamDomain::Workflows])).await?;
replace_snapshot(stream.baseline().expect("complete authorized baseline"))?;
loop {
    match stream.next_event().await {
        Ok(event) => {
            apply_to_projection(&event)?;
            stream.acknowledge(&event)?;
        }
        Err(_) => {
            if stream.reconnect().await? == RecoveryMode::Snapshot {
                replace_snapshot(stream.baseline().expect("fresh authorized baseline"))?;
            }
        }
    }
}
```
