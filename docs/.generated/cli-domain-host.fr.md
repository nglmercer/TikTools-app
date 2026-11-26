| Commande | Résumé |
| --- | --- |
| `tiktools host --stdio [--events]` | host sans interface sur stdio ; diffuse les événements avec --events |
| `tiktools host --ipc` | host sans interface en IPC local ; refuse quand un host est déjà en cours |
| `tiktools rpc <method> [params-json] &#124; rpc --stdio [--events]` | passage direct non typé vers toute méthode du host |
| `tiktools rpc --stdio [--events]` | boucle de requête/réponse NDJSON sur stdio |
