import type { TourCatalog } from './index.ts';
export default {
  controls: { next: 'Suivant', previous: 'Précédent', done: 'Terminer le guide', close: 'Fermer le guide', progress: '{{current}} sur {{total}}' },
  connection: {
    form: { title: 'Choisir un créateur', description: 'Saisissez son nom et vérifiez la connexion. Le guide ne connecte jamais le compte et ne lit aucun cookie de session.' },
    explorer: { title: 'Explorer les salles LIVE', description: 'Actualisez ou choisissez une salle vous-même. La connexion de démonstration est simulée ; le mode natif dépend du serveur local et de TikTok.' },
    plugins: { title: 'Vérifier les intégrations', description: 'Examinez chaque connexion installée avant les effets. Ouvrir le guide ne contacte aucun service de plugin.' },
  },
  automation: {
    profiles: { title: 'Choisir le profil de travail', description: 'Gardez un profil pour les règles à examiner. Changer ou importer un profil exige une action explicite.' },
    library: { title: 'Examiner événements et actions', description: 'Un événement relie déclencheur, filtres et délai aux actions réutilisables. Gardez les nouveaux effets désactivés avant leur examen.' },
    templates: { title: 'Partir d’un modèle', description: 'Ouvrez Rule templates pour prévisualiser une configuration. Importation et activation ont leur propre examen ; le guide ne les exécute pas.' },
  },
  plugin: {
    catalog: { title: 'Installés et Explore', description: 'Examinez les paquets ou parcourez les entrées compatibles. Les plugins natifs et processus conservent les permissions système de leur utilisateur.' },
    search: { title: 'Trouver un paquet', description: 'Recherchez et vérifiez éditeur, version et prérequis. Un catalogue vide diffère d’un service indisponible.' },
    install: { title: 'Examiner l’installation', description: 'L’installation exige un examen des signatures et permissions. Le guide n’installe rien et n’exécute aucune action de test.' },
  },
  overlay: {
    gateway: { title: 'Vérifier la passerelle', description: 'La sortie en direct exige event-gateway et un identifiant de widget. Un aperçu local ne prouve pas la connexion OBS.' },
    preview: { title: 'Prévisualiser localement', description: 'L’aperçu emploie le même rendu avec des événements exemples. Il ne nécessite aucun LIVE réel et n’envoie aucune alerte OBS.' },
    design: { title: 'Examiner la présentation', description: 'Modifiez et enregistrez volontairement, puis copiez une URL actualisée pour OBS. Changes des overlays reste un suivi non intégré.' },
  },
  points: {
    rules: { title: 'Examiner les règles de points', description: 'Vérifiez les valeurs avant modification. Certains contrôles sont verrouillés pendant une session LIVE connectée ou occupée.' },
    leaderboard: { title: 'Examiner les points confirmés', description: 'Recherchez ou triez les spectateurs. Actualiser l’affichage ne modifie pas les points et ne rejoue aucun événement LIVE.' },
  },
  changes: {
    library: { title: 'Choisir les enregistrements', description: 'Événements, actions et variables ont des examens distincts. Conservez les brouillons avant d’examiner un autre état.' },
    review: { title: 'Examiner Changes', description: 'Changes des automatisations reste local. Variables révisées utilise des contrôles natifs de version ; cet examen ne téléverse pas les valeurs brutes.' },
    profiles: { title: 'Récupérer explicitement', description: 'Résolvez une confirmation incertaine avec le journal d’origine. Fermer le guide ne termine aucune tâche et ne répète aucune mutation.' },
  },
  diagnostics: {
    log: { title: 'Examiner l’historique limité', description: 'Filtrez source, niveau ou texte et examinez les répétitions. Vérifiez les exports expurgés avant partage ; l’historique dure seulement la session.' },
    back: { title: 'Examiner les preuves et récupérer', description: 'Copiez ou exportez les preuves examinées, puis utilisez la récupération d’origine pour conflits, sessions expirées ou commandes incertaines. Effacer les journaux ne répare pas leur cause.' },
  },
} satisfies TourCatalog;
