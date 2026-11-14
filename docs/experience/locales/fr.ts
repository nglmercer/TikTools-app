import type { ExperienceCopy } from '../types.ts';

const copy: ExperienceCopy = {
  interactive: 'Exemple interactif', fictional: 'Données fictives', tryIt: 'Essayer les commandes', reset: 'Réinitialiser l’exemple', openFull: 'Ouvrir la démo complète', close: 'Fermer l’exemple',
  preview: 'Aperçu', loading: 'Chargement de l’interface réelle…', task: 'À vous de jouer', success: 'Résultat à vérifier',
  topicsTitle: 'Apprendre par la pratique', topicsDescription: 'Une interface réelle, une tâche précise et un résultat à vérifier. Choisissez votre prochain parcours.', readGuide: 'Lire le guide',
  topics: {
    connection: { title: 'Connecter votre LIVE', description: 'Choisissez un créateur et découvrez les événements de votre audience.', task: 'Connectez demo_creator, puis examinez un événement entrant.', result: 'La connexion devient active et des événements fictifs apparaissent dans le flux.' },
    automation: { title: 'Transformer les événements en actions', description: 'Créez une règle de chat et examinez son exécution.', task: 'Explorez l’action d’exemple, lancez un test et examinez son résultat.', result: 'L’historique des automatisations affiche une exécution terminée et son résultat rendu.' },
    overlays: { title: 'Préparer votre habillage de diffusion', description: 'Prévisualisez un widget et préparez sa source navigateur pour OBS.', task: 'Choisissez un widget, modifiez son apparence et rejouez l’aperçu.', result: 'L’aperçu reflète votre design. La diffusion réelle dans OBS nécessite une passerelle active.' },
    points: { title: 'Récompenser votre audience', description: 'Définissez les règles d’attribution et comprenez les soldes des spectateurs.', task: 'Examinez les règles et comparez le solde d’un spectateur après un événement.', result: 'Le solde d’exemple évolue. Les attributions de la démo sont fixes ; les taux réels se vérifient lors d’un LIVE.' },
  },
  otherTasks: {
    feed: { task: 'Faites défiler les événements d’exemple, sélectionnez une entrée du flux et ouvrez le profil du spectateur.', result: 'Les détails de l’événement fictif et l’activité enregistrée du spectateur apparaissent sans connexion à TikTok.' },
    analytics: { task: 'Consultez les totaux actuels, générez un événement d’exemple et comparez l’activité mise à jour.', result: 'Les totaux reflètent l’événement simulé. Ils ne mesurent pas une véritable diffusion.' },
    plugins: { task: 'Examinez le plugin d’exemple et ses autorisations, puis essayez ses commandes d’installation simulée.', result: 'L’état du plugin change dans la démo ; aucun paquet, service ni périphérique audio n’est contacté.' },
    cloud: { task: 'Comparez la variable greeting locale avec l’exemple Cloud et choisissez la modification examinée à appliquer.', result: 'La comparaison fictive est résolue localement. Aucun compte Cloud ni aucune donnée distante ne sont modifiés.' },
    collaboration: { task: 'Examinez la demande de l’appareil fictif et approuvez uniquement les accès limités que vous souhaitez accorder.', result: 'L’identité d’exemple apparaît avec les autorisations examinées. Aucun appareil externe ne rejoint cette démo.' },
    settings: { task: 'Modifiez une préférence locale, observez son effet, puis réinitialisez la démo.', result: 'La préférence s’applique uniquement à cette instance de démo ; la réinitialisation restaure son état initial.' },
    diagnostics: { task: 'Sélectionnez l’entrée en échec, examinez ses détails et réessayez une action simulée.', result: 'Vous pouvez suivre l’échec d’exemple et le résultat de la nouvelle tentative sans contacter de service externe.' },
  },
  previewLabels: { creator: 'Créateur', event: 'Événement de l’audience', action: 'Automatisation', output: 'Diffusion', chat: 'Message du chat', gift: 'Cadeau reçu', connected: 'Connecté', disconnected: 'Déconnecté', message: 'Bonjour TikTools !' },
  connectionStates: { label: 'Exemple de connexion', normal: 'Connexion réussie', offline: 'Créateur hors ligne', auth: 'Session requise', offlineHelp: 'Le créateur n’est pas en direct. Vérifiez que sa diffusion a commencé avant de réessayer.', authHelp: 'Cette salle exige une session TikTok autorisée. Dans l’application de bureau, saisissez un cookie de session récent dans le champ masqué. Ne collez jamais de véritables identifiants dans cette démo.' },
  releases: { stable: 'Stable', prerelease: 'Préversion', notes: 'Notes de version', source: 'Code source', downloads: 'Téléchargements', noDownloads: 'Aucun téléchargement pour ce système dans cet instantané.', checksums: 'Sommes de contrôle SHA-256', published: 'Versions publiées', snapshot: 'Instantané des versions vérifié', refresh: 'Voir les dernières versions sur GitHub', all: 'Tous', highlights: 'Points marquants', unavailable: 'Les données des versions sont indisponibles. Consultez les versions publiées sur GitHub.' },
};
export default copy;
