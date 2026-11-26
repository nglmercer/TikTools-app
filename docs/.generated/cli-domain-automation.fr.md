| Commande | Résumé |
| --- | --- |
| `tiktools automation list [--kind event&#124;action&#124;all]` | lister les règles d'automatisation |
| `tiktools automation context [--event-type t]` | contexte de modèle disponible pour les règles |
| `tiktools automation get <id> [--kind event&#124;action]` | lire une règle |
| `tiktools automation create --record json [--kind event&#124;action]` | créer une règle |
| `tiktools automation update <id> --record json [--kind event&#124;action]` | mettre à jour une règle |
| `tiktools automation delete <id> [--kind event&#124;action]` | supprimer une règle **destructif** |
| `tiktools automation enable <id> [--kind event&#124;action]` | activer une règle |
| `tiktools automation disable <id> [--kind event&#124;action]` | désactiver une règle |
| `tiktools automation test (--id <id> &#124; --record json) [--kind event&#124;action] [--trigger t] [--event json]` | tester une règle à blanc contre un événement d'exemple |
| `tiktools automation fire --trigger <type> [--event json] [--record json]` | déclencher un déclencheur manuellement |
| `tiktools automation moderation-penalty --points <delta>` | appliquer une pénalité de modération en points |
