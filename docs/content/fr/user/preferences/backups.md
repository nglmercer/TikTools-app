---
title: "Sauvegarder et restaurer le travail local"
docId: "user.preferences.backups"
translationKey: "user.preferences.backups"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "b2ae789f9c15df21b415b5bc006404dffd60430f193fdaba8cd94931e750eedc"
translationRevision: "8d4c1c22be78540b85ff938603d818d98cb9dbe33bbaa868edb9327de83971a4"
order: 20
---

# Sauvegarder et restaurer le travail local {#back-up-and-restore-local-work}

## Objectif et prérequis {#goal-and-prerequisites}

Préserver une configuration fonctionnelle avant les mises à jour, réinitialisations ou changements d’ordinateur. Il faut un stockage que vous contrôlez et le temps de fermer TikTools proprement. L’interface actuelle n’a pas de bouton universel **Back up everything** ; utilisez les exportations spécifiques et une copie des données avec l’application fermée.

## Exporter les définitions réutilisables {#export-reusable-definitions}

1. Dans **Automations**, sélectionnez le profil voulu et utilisez son action d’exportation pour enregistrer un fichier `.tikprofile.json`. Nommez-le avec la version de l’application et la date.
2. Pour un seul modèle réutilisable, utilisez la commande de téléchargement/exportation de la boîte de dialogue des modèles pour enregistrer `.tiktemplate.json` lorsqu’elle est proposée.
3. Examinez le fichier exporté avant de le partager. Les modèles peuvent contenir des destinations et valeurs de configuration. L’exportation d’un profil ne copie ni les soldes, ni les statistiques, variables d’exécution, fichiers multimédias ou identifiants de compte.

## Copier les données locales {#copy-the-local-data}

1. Arrêtez l’activité LIVE et tout service de collaboration, puis quittez TikTools normalement. Attendez que l’hôte ait cessé d’écrire.
2. Repérez la racine de données active. Les valeurs par défaut sont `%LOCALAPPDATA%\TikTools` sous Windows et `~/.local/share/TikTools` sous Linux, ou `TikTools` sous `XDG_DATA_HOME` si configuré. Un `TIKTOOLS_HOME` personnalisé ou une surcharge par répertoire modifie cet emplacement.
3. Copiez la racine et les répertoires de données, plugins, plugin-data et plugin-store redéfinis séparément dans un dossier de sauvegarde daté. Copiez tout le répertoire de base de données fermée, y compris les fichiers annexes SQLite présents, plutôt que de sélectionner un seul fichier `.db`.
4. Conservez séparément les fichiers audio référencés : les actions audio gardent des chemins plutôt que d’importer les octets des sons. Notez la version de l’application et les emplacements personnalisés avec la sauvegarde.
5. Protégez la copie car la configuration locale peut contenir des valeurs privées. L’authentification Cloud réside dans le coffre d’identifiants du système d’exploitation et n’est pas transportée par cette copie de dossier.

## Restaurer et vérifier {#restore-and-verify}

Avec TikTools fermé, conservez le dossier actuel comme copie de récupération distincte. Restaurez les fichiers enregistrés aux emplacements voulus et démarrez une version compatible. Examinez la sélection du profil, les soldes, variables et chemins multimédias avant de vous reconnecter. Pour les définitions portables, importez le profil/modèle exporté via Automations ; les définitions importées restent désactivées jusqu’à leur activation explicite.

## Résultat attendu, état et récupération {#expected-result-status-and-recovery}

Les copies réalisées avec l’application fermée et les outils d’importation/exportation nommés sont des méthodes de récupération disponibles, sans garantie universelle de restauration entre versions. Cloud Changes n’est pas une sauvegarde complète. Si une version rejette les données restaurées, arrêtez-vous et revenez à la copie préservée ; réinstaller un ancien binaire n’annule pas une migration de base de données. Consultez les [mises à jour](../updates.md) et le [dépannage](troubleshooting.md).

## Détails de fonctionnement {#operational-details}

Les chemins individuels priment sur `TIKTOOLS_HOME` : `TIKTOOLS_DATA_DIR` contient les bases, `TIKTOOLS_PLUGINS_DIR` les paquets, `TIKTOOLS_PLUGIN_DATA_DIR` leur état et `TIKTOOLS_PLUGIN_STORE_DIR` le magasin de paquets. `TIKTOOLS_LOG_DIR` et `TIKTOOLS_TEMP_DIR` choisissent journaux et fichiers temporaires. Les chemins relatifs dépendent du dossier de lancement. Avant sauvegarde, consultez `tiktools api call system.info --json` et notez les variables héritées ; copiez données/paquets/état avec l’hôte arrêté, y compris les WAL/SHM existants. Les journaux ne remplacent pas les bases.

`system.info` indique home/dataDir effectifs ; résolvez les autres dossiers depuis l’environnement réel de lancement de l’hôte, pas un autre terminal. Utilisez `system.health` pour ready/attentes analytiques et compteurs LIVE.
