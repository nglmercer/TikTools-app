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
