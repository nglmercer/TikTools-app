| Commande | Résumé |
| --- | --- |
| `tiktools publisher list` | lister les espaces d'éditeur |
| `tiktools publisher create <namespace>` | créer un espace d'éditeur |
| `tiktools publisher delete <namespace-or-publisher-id>` | supprimer un espace ou un éditeur **destructif** |
| `tiktools publisher keys <publisher-id>` | lister les clés de signature d'un éditeur |
| `tiktools publisher enroll <publisher-id> [--key-id <id>]` | inscrire une clé d'éditeur ; lit le mot de passe du backend depuis stdin |
| `tiktools publisher upload <publisher-id> <path>` | téléverser un paquet de plugin |
| `tiktools publisher submit <upload-id> <publisher-key-id> <plugin-id> <version> [--target any]` | soumettre un téléversement pour révision |
| `tiktools publisher release <release-id>` | créer une release |
| `tiktools publisher publish <release-id>` | publier une release |
