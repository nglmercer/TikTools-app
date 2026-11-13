---
title: "Sauvegarder et restaurer le travail local"
docId: "user.preferences.backups"
translationKey: "user.preferences.backups"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "7e6d88e43b32698c36afd6385bee8c44cc1223d669dd4c95414db604c17f254f"
translationRevision: "7e6d88e43b32698c36afd6385bee8c44cc1223d669dd4c95414db604c17f254f"
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
