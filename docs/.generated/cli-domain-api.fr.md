| Commande | Résumé |
| --- | --- |
| `tiktools api discover [--format json&#124;human]` | lister toutes les méthodes en direct du registre du host |
| `tiktools api schema <method> [--format json&#124;human]` | description, drapeaux et schémas d'une méthode |
| `tiktools api call <method> [params-json] [--timeout SEC] [--dry-run] [--confirm] [--redact&#124;--secrets-visible] [--explain] [--format json&#124;human] [--expected-revision N] [--command-id ID --host-instance-id ID] [--include-meta]` | valider et exécuter une méthode ; les destructives exigent --confirm |
| `tiktools api events [--topics t,...] [--timeout SEC] [--max-events N]` | diffuser les événements de domaine NDJSON |
| `tiktools api event get <name> [--type kind]... [--author text] [--limit N]` | rechercher dans le journal borné du feed |
| `tiktools api verify [--coverage&#124;--smoke] [--sandbox] [--timeout SEC]` | vérifier la surface de contrôle contre le host |
