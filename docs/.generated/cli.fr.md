`tiktools [--json] [--standalone] <command> [args]`

### `tiktools plugin` — installer et gérer les plugins

| Commande | Résumé |
| --- | --- |
| `tiktools plugin list` | lister les plugins installés |
| `tiktools plugin get <id>` | lire un plugin |
| `tiktools plugin install <file.plugin> [--replace] &#124; plugin install <id[@version]&#124;url> [--yes] [--no-wait] [--allow-unsigned]` | installer depuis un fichier, id@version ou url |
| `tiktools plugin uninstall <id> [--purge]` | désinstaller un plugin **destructif** |
| `tiktools plugin search <query> [--category c] [--page N]` | rechercher des paquets dans le registre |
| `tiktools plugin inspect <spec>` | inspecter une spécification de paquet avant installation |
| `tiktools plugin outdated` | lister les plugins installés avec des mises à jour disponibles |
| `tiktools plugin update <id> [--version v] [--yes] [--no-wait]` | mettre à jour un plugin |
| `tiktools plugin update --all [--yes] [--no-wait]` | mettre à jour tous les plugins obsolètes |
| `tiktools plugin rollback <id> [version] [--no-wait]` | revenir à une version précédente **destructif** |
| `tiktools plugin versions <id>` | lister les versions disponibles d'un plugin |
| `tiktools plugin pin <id>` | épingler un plugin à sa version actuelle |
| `tiktools plugin unpin <id>` | retirer l'épinglage de version |
| `tiktools plugin policy <id> [--channel stable&#124;beta] [--auto off&#124;notify&#124;patch&#124;minor]` | définir le canal et la politique de mise à jour automatique |
| `tiktools plugin jobs` | lister les travaux de paquets |
| `tiktools plugin job <job-id>` | inspecter un travail de paquet |
| `tiktools plugin cancel <job-id>` | annuler un travail de paquet en cours **destructif** |
| `tiktools plugin enable <id>` | activer un plugin |
| `tiktools plugin disable <id>` | désactiver un plugin |
| `tiktools plugin start <id>` | démarrer un plugin |
| `tiktools plugin stop <id>` | arrêter un plugin **destructif** |
| `tiktools plugin settings get <id>` | lire la configuration d'un plugin |
| `tiktools plugin settings set <id> k=v...` | écrire la configuration d'un plugin |
| `tiktools plugin settings reset <id>` | réinitialiser la configuration d'un plugin **destructif** |
| `tiktools plugin health <id>` | vérification de santé d'un plugin |
| `tiktools plugin options <action-type/field>` | options d'un type d'action ou champ |
| `tiktools plugin action <type> [--config json] [--live]` | exécuter une action de plugin |
| `tiktools plugin commands` | lister les commandes contribuées par les plugins installés |
| `tiktools plugin help [id]` | décrire une commande contribuée |
| `tiktools plugin <id> <command> [--params json] [--live]` | invoquer une commande contribuée par un plugin |

### `tiktools cloud` — compte cloud et sauvegarde/restauration portable (remplacer avec TIKTOOLS_CLOUD_ORIGIN)

| Commande | Résumé |
| --- | --- |
| `tiktools cloud status` | état du compte cloud |
| `tiktools cloud login` | se connecter via le navigateur système |
| `tiktools cloud logout` | se déconnecter **destructif** |
| `tiktools cloud restore` | restaurer la session cloud |
| `tiktools cloud backup create [--out FILE]` | écrire une archive de sauvegarde portable |
| `tiktools cloud backup preview <file>` | inspecter une archive de sauvegarde |
| `tiktools cloud backup restore <file> [--yes]` | restaurer une archive de sauvegarde **destructif** |

### `tiktools publisher` — flux d'éditeur cloud (connexion requise)

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

### `tiktools registry` — registres de paquets et le catalogue cloud

| Commande | Résumé |
| --- | --- |
| `tiktools registry list` | lister les registres configurés |
| `tiktools registry refresh` | actualiser les métadonnées du registre |
| `tiktools registry package <id>` | afficher un paquet du registre |
| `tiktools registry cloud-catalog [--query q] [--after cursor] [--generation value] [--target name]` | rechercher dans le catalogue cloud |
| `tiktools registry cloud-summaries [--query q] [--cursor value] [--generation value] [--target name]` | page de résumés du catalogue cloud |
| `tiktools registry cloud-versions <id> [--target name] [--generation value] [--cursor value]` | versions d'un paquet du catalogue cloud |
| `tiktools registry cloud-pagination` | capacités de pagination du catalogue cloud |
| `tiktools registry defaults` | configuration par défaut du registre |

### `tiktools processor` — processeurs d'événements

| Commande | Résumé |
| --- | --- |
| `tiktools processor list` | lister les processeurs |
| `tiktools processor status` | état du pipeline de processeurs |
| `tiktools processor test <plugin-id> <processor-id> [event-json]` | exécuter un processeur contre un événement d'exemple |

### `tiktools live` — connexion à TikTok LIVE

| Commande | Résumé |
| --- | --- |
| `tiktools live connect --unique-id <id> --session-cookie <cookie> [--room-id <id>]` | se connecter à une salle en direct |
| `tiktools live pick --session-cookie <cookie>` | choisir une salle en direct interactivement |
| `tiktools live channels [--keyword text] [--offset N] [--limit N] [--session-cookie <cookie>]` | découvrir les chaînes en direct ; fonctionne sans session |
| `tiktools live disconnect` | se déconnecter de la salle actuelle **destructif** |
| `tiktools live status` | état actuel de la connexion |

### `tiktools points` — points de fidélité des spectateurs

| Commande | Résumé |
| --- | --- |
| `tiktools points config get` | lire la configuration des points |
| `tiktools points config set k=v...` | mettre à jour la configuration des points |
| `tiktools points viewer get <id>` | lire le solde d'un spectateur |
| `tiktools points adjust <id> <delta>` | ajouter ou retirer des points à un spectateur |
| `tiktools points leaderboard [--limit N]` | spectateurs avec le plus grand solde |
| `tiktools points reset [id]` | réinitialiser le solde d'un spectateur ou de tous **destructif** |

### `tiktools ranking` — définitions et état des classements de spectateurs

| Commande | Résumé |
| --- | --- |
| `tiktools ranking list` | lister les définitions de classement |
| `tiktools ranking metrics` | métriques disponibles pour les définitions de classement |
| `tiktools ranking create --record <json&#124;@file>` | créer une définition de classement |
| `tiktools ranking update --record <json&#124;@file>` | mettre à jour une définition de classement |
| `tiktools ranking get <id>` | lire un classement |
| `tiktools ranking snapshot <id>` | classement actuel d'un ranking |
| `tiktools ranking delete <id>` | supprimer un classement **destructif** |
| `tiktools ranking reset <id>` | réinitialiser l'état accumulé **destructif** |
| `tiktools ranking pause <id>` | suspendre l'accumulation |
| `tiktools ranking resume <id>` | reprendre l'accumulation |
| `tiktools ranking contribute --record <json&#124;@file>` | ajouter une contribution externe |

### `tiktools competition` — compétitions chronométrées de spectateurs

| Commande | Résumé |
| --- | --- |
| `tiktools competition list` | lister les compétitions |
| `tiktools competition get <id>` | lire une compétition |
| `tiktools competition create --record <json&#124;@file>` | créer une compétition |
| `tiktools competition update --record <json&#124;@file>` | mettre à jour une compétition |
| `tiktools competition delete <id>` | supprimer une compétition **destructif** |
| `tiktools competition state <id>` | état actuel de la compétition |
| `tiktools competition results <id>` | résultats finaux ou intermédiaires |
| `tiktools competition action --record <json&#124;@file>` | enregistrer une action de compétition |
| `tiktools competition mark-action-uncertain --record <json&#124;@file>` | marquer une action comme incertaine |
| `tiktools competition resolve-action --record <json&#124;@file>` | résoudre une action incertaine |
| `tiktools competition start <id>` | démarrer une compétition |
| `tiktools competition finish <id>` | terminer une compétition **destructif** |
| `tiktools competition candidates <id>` | candidats au titre de gagnant |
| `tiktools competition select-winner --record <json&#124;@file>` | sélectionner le gagnant |
| `tiktools competition pause <id>` | suspendre une compétition |
| `tiktools competition resume <id>` | reprendre une compétition |
| `tiktools competition cancel <id>` | annuler une compétition **destructif** |

### `tiktools automation` — règles d'automatisation d'événements et d'actions

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

### `tiktools globals` — globales clé/valeur partagées entre plugins

| Commande | Résumé |
| --- | --- |
| `tiktools globals list` | lister toutes les clés globales |
| `tiktools globals get <key>` | lire une globale |
| `tiktools globals set <key> <value>` | écrire une globale |
| `tiktools globals delete <key>` | supprimer une globale **destructif** |

### `tiktools workflow` — graphes de workflows visuels

| Commande | Résumé |
| --- | --- |
| `tiktools workflow list` | lister les graphes de workflow enregistrés |
| `tiktools workflow get <id>` | lire un graphe |
| `tiktools workflow save --record json` | créer ou remplacer un graphe |
| `tiktools workflow delete <id>` | supprimer un graphe **destructif** |
| `tiktools workflow enable <id>` | activer un graphe |
| `tiktools workflow disable <id>` | désactiver un graphe |

### `tiktools media` — validation et lecture de fichiers multimédias

| Commande | Résumé |
| --- | --- |
| `tiktools media validate <path> [--kind audio&#124;video&#124;image&#124;other]` | valider un fichier multimédia avant utilisation dans des actions |

### `tiktools system` — information et diagnostic du host

| Commande | Résumé |
| --- | --- |
| `tiktools system info` | version du host et information du runtime |
| `tiktools system health` | rapport de santé des composants |
| `tiktools system snapshot` | instantané complet de l'état pour le débogage |
| `tiktools system doctor` | diagnostiquer les problèmes courants de configuration |
| `tiktools system request-input-access` | demander l'accès aux entrées à l'utilisateur du bureau |
| `tiktools system shutdown` | arrêter le host en cours **destructif** |

### `tiktools template` — modèles d'automatisation et profils réutilisables

| Commande | Résumé |
| --- | --- |
| `tiktools template import <file> [--param k=v]... [--event-name n] [--action-name n] [--dry-run] [--no-save] [--disabled]` | importer un modèle comme règles d'automatisation |
| `tiktools template export --event <id> --out <file> [--title t] [--id template-id]` | exporter une règle d'événements comme modèle |
| `tiktools template list` | lister les modèles de la galerie |
| `tiktools template delete <template-id>` | supprimer un modèle **destructif** |
| `tiktools template profile-import <file> [--param k=v]... [--dry-run] [--no-save] [--disabled]` | importer un profil d'automatisation complet |
| `tiktools template profile-export --out <file> [--name n]` | exporter le profil d'automatisation |

### `tiktools api` — surface de contrôle générique orientée agents sur les méthodes du host

| Commande | Résumé |
| --- | --- |
| `tiktools api discover [--format json&#124;human]` | lister toutes les méthodes en direct du registre du host |
| `tiktools api schema <method> [--format json&#124;human]` | description, drapeaux et schémas d'une méthode |
| `tiktools api call <method> [params-json] [--timeout SEC] [--dry-run] [--confirm] [--redact&#124;--secrets-visible] [--explain] [--format json&#124;human] [--expected-revision N] [--command-id ID --host-instance-id ID] [--include-meta]` | valider et exécuter une méthode ; les destructives exigent --confirm |
| `tiktools api events [--topics t,...] [--timeout SEC] [--max-events N]` | diffuser les événements de domaine NDJSON |
| `tiktools api event get <name> [--type kind]... [--author text] [--limit N]` | rechercher dans le journal borné du feed |
| `tiktools api verify [--coverage&#124;--smoke] [--sandbox] [--timeout SEC]` | vérifier la surface de contrôle contre le host |

### `tiktools host` — runtimes de host sans interface et le passage direct de rpc

| Commande | Résumé |
| --- | --- |
| `tiktools host --stdio [--events]` | host sans interface sur stdio ; diffuse les événements avec --events |
| `tiktools host --ipc` | host sans interface en IPC local ; refuse quand un host est déjà en cours |
| `tiktools rpc <method> [params-json] &#124; rpc --stdio [--events]` | passage direct non typé vers toute méthode du host |
| `tiktools rpc --stdio [--events]` | boucle de requête/réponse NDJSON sur stdio |

### `tiktools web` — adaptateur de navigateur local (requiert la feature local-web)

| Commande | Résumé |
| --- | --- |
| `tiktools web --assets <directory> [--port N] [--join-origin URL] [--multi-session]` | servir l'adaptateur de navigateur local ; host --ipc doit être en cours |
| `tiktools web --print-code [--port N]` | imprimer le code d'appariement actif sans redémarrer |
| `tiktools web --reset-session [--port N]` | révoquer tous les navigateurs appariés et imprimer un nouveau code **destructif** |

### Conventions

- **résolution du host**: les commandes s'exécutent contre le host en cours via IPC local ; sans host actif elles échouent avec host_unavailable au lieu de démarrer un second runtime. Utilisez `tiktools --json system info` comme sonde légère : sortie 0 signifie qu'un host a répondu, sortie 3 que le host IPC est indisponible. --standalone exécute la commande contre un runtime isolé en processus.
- **sortie**: stdout porte le résultat ; les diagnostics vont sur stderr. --json imprime l'objet de résultat brut ; sans lui, les listes impriment une ligne par élément. Les valeurs json peuvent utiliser @path pour lire depuis un fichier. L'aide humaine suit la locale d'affichage : TIKTOOLS_LANG remplace la chaîne LC_ALL/LC_MESSAGES/LANG, et la sortie machine reste indépendante de la locale.
- **codes de sortie**: 0 ok, 1 erreur d'opération, 2 erreur d'usage, 3 échec de transport.
- **secrets**: live connect/pick lisent le cookie de session depuis --session-cookie ou la variable d'environnement TIKTOOLS_SESSION_COOKIE (le drapeau l'emporte) ; la valeur n'apparaît jamais dans les diagnostics. api call masque les secrets de sa sortie sauf si --secrets-visible est passé, et refuse les méthodes destructives sans --confirm.
