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
  feed: {
    view: { title: 'Choisir la vue du flux', description: 'Le flux en direct affiche chat, cadeaux et abonnements sous forme de cartes ; passez en Liste pour le journal compact.' },
    filters: { title: 'Filtrer l’activité', description: 'Filtrez par type ou recherchez un spectateur ou un message. Les filtres s’appliquent aussi aux cartes du flux.' },
    stage: { title: 'Lire le mur en direct', description: 'Les messages importants restent affichés. Les j’aime et les arrivées apparaissent brièvement comme réactions, puis disparaissent.' },
    controls: { title: 'Pause, effacement et inspection', description: 'Mettez le défilement en pause, effacez l’historique enregistré ou ouvrez l’inspecteur pour plus de détails.' },
  },
  backup: {
    plan: { title: 'Connaissez vos limites', description: 'Chaque offre a ses propres limites de stockage et de sauvegardes. Ces jauges montrent ce que vous utilisez.' },
    tools: { title: 'Cloud ou ce PC', description: 'Basculez entre l’enregistrement dans votre compte cloud et le travail avec des fichiers sur ce PC : exportez vos données ou importez un fichier existant.' },
    create: { title: 'Enregistrez une sauvegarde', description: 'Nommez une sauvegarde et enregistrez-la dans votre compte. Elle contient règles, profils, réglages et fichiers cloud, jamais de secrets ni de cookies.' },
    list: { title: 'Téléchargez, restaurez ou supprimez', description: 'Chaque sauvegarde peut être téléchargée, restaurée sur tout appareil connecté à ce compte ou supprimée. La restauration montre d\'abord ce qui changera.' },
  },
  analytics: {
    range: { title: "Choisissez la plage de dates", description: "Choisissez un préréglage ou des dates personnalisées. Le résumé se recharge pour le créateur actif ; changer la plage ne modifie jamais les événements enregistrés." },
    metrics: { title: "Choisissez la métrique", description: "Les cartes comparent chats, cadeaux, likes et arrivées. La métrique sélectionnée pilote le graphique d’activité sans modifier les données d’analyse." },
    activity: { title: "Lisez le graphique d’activité", description: "Utilisez le graphique pour repérer les jours et heures actifs. Les périodes vides peuvent signifier aucune session LIVE ou aucun événement correspondant." },
    facts: { title: "Vérifiez les faits clés", description: "Les totaux et le nombre de sessions donnent une vue compacte avant de comparer des spectateurs individuels." },
    contributors: { title: "Recherchez les contributeurs", description: "Triez ou recherchez dans le tableau pour trouver les spectateurs les plus actifs. Il s’agit d’un classement en lecture seule." },
  },
  variables: {
    library: { title: "Ouvrez Variables", description: "Les variables vivent avec les automatisations. Ouvrez l’onglet Variables pour examiner les valeurs partagées par les événements et les actions." },
    values: { title: "Vérifiez valeurs et secrets", description: "Les valeurs locales peuvent être marquées comme secrètes. L’éditeur ne révèle jamais un secret enregistré et son enregistrement reste toujours explicite." },
    review: { title: "Comparez avant de synchroniser", description: "Utilisez Changes pour comparer les révisions locales et Cloud. Examinez d’abord la différence ; le guide ne téléverse pas les valeurs secrètes brutes." },
  },
  settings: {
    sections: { title: "Choisissez une section de paramètres", description: "General contient les préférences de l’application. Account contient le profil Cloud, le plan et les outils de sauvegarde." },
    language: { title: "Changez la langue", description: "La langue s’applique immédiatement à l’interface. Elle ne change ni les données du LIVE ni le comportement des automatisations." },
    review: { title: "Choisissez le mode de révision", description: "Le mode de révision contrôle le niveau de confirmation demandé par les flux de synchronisation sensibles. Les modes plus sûrs exigent une révision plus explicite." },
    updater: { title: "Vérifiez les mises à jour manuellement", description: "Utilisez la carte de mise à jour pour voir le canal et rechercher des mises à jour. Si elles sont indisponibles, téléchargez la version par le chemin de récupération documenté." },
  },
  overlayObs: {
    gateway: { title: "Commencez par la passerelle", description: "Les widgets OBS reçoivent les événements via la passerelle locale. Confirmez qu’elle fonctionne avant de copier une URL de source." },
    widget: { title: "Choisissez le widget", description: "Choisissez la source follows, cadeaux, chat, likes ou ranking à placer dans OBS." },
    preview: { title: "Testez avec l’aperçu", description: "Rejouez des événements d’exemple et ajustez le design localement. L’aperçu ne prouve pas que OBS est connecté." },
    url: { title: "Copiez l’URL OBS", description: "Copiez l’URL de source seulement lorsque la passerelle est prête, puis collez-la dans une Browser Source OBS avec la taille voulue." },
    design: { title: "Réglez mouvement et apparence", description: "L’éditeur de design inclut maintenant style d’animation, durée, easing et stagger. Enregistrez avant de copier une URL actualisée." },
  },
  cloudOverlays: {
    open: { title: "Ouvrez la bibliothèque Cloud", description: "Quand les overlays Cloud sont disponibles, parcourez les documents partagés depuis le bouton de bibliothèque Cloud." },
    widgets: { title: "Sélectionnez une cible locale", description: "Choisissez quel widget local doit recevoir le design Cloud sélectionné avant d’appliquer quoi que ce soit." },
    preview: { title: "Prévisualisez avant de publier", description: "Utilisez l’aperçu local pour comparer le design sélectionné. Publier ou écraser une copie Cloud reste une action explicite." },
    url: { title: "Copiez la bonne source", description: "Les URL de passerelle locale et les URL d’overlay Cloud répondent à des cas de récupération différents. Vérifiez laquelle votre scène OBS doit utiliser." },
  },
  automationAdvanced: {
    library: { title: "Ouvrez un événement délibérément", description: "Le comportement avancé vit dans un événement. Sélectionnez ou créez un événement désactivé avant d’expérimenter." },
    editor: { title: "Vérifiez filtres, cooldown et ordre", description: "Dans l’éditeur, conditions, alternatives, portée du cooldown, mode aléatoire et délais décident exactement quand les actions s’exécutent." },
    templates: { title: "Utilisez les modèles comme point de départ", description: "Les modèles peuvent encoder des valeurs par défaut plus sûres. L’importation exige toujours une révision et n’active pas la règle à elle seule." },
    runs: { title: "Vérifiez avec tests et historique", description: "Utilisez les exécutions de test et le panneau d’historique pour confirmer timing, actions ignorées et comportement du cooldown avant d’activer l’événement." },
  },
  rankings: {
    tabs: { title: "Classements ou compétitions", description: "Les classements accumulent des scores. Les compétitions comparent des classements sélectionnés pendant un événement limité." },
    list: { title: "Choisissez ou créez un classement", description: "La liste montre métrique, mode et état. Créez un classement seulement après avoir décidé quelle action du spectateur doit marquer des points." },
    detail: { title: "Inspectez le tableau des scores", description: "La vue détaillée montre score, état de pause et intégrité. Pause, reprise et réinitialisation sont des opérations explicites." },
    competitions: { title: "Configurez les compétitions", description: "Les compétitions exigent des classements compatibles et des limites claires. Elles sont expérimentales ; vérifiez l’overlay avant de les utiliser à l’antenne." },
  },
  pluginSetup: {
    catalog: { title: "Ouvrez le plugin installé", description: "Commencez dans Installed, pas Explore, pour configurer un plugin déjà approuvé sur cette machine." },
    details: { title: "Vérifiez permissions et état", description: "L’en-tête des détails montre l’état d’activation, la version et les actions qui affectent le runtime. Vérifiez les permissions avant l’activation." },
    settings: { title: "Configurez les réglages", description: "L’onglet Settings contient les valeurs propres au plugin. Enregistrez d’abord lorsque le plugin le demande avant de tester." },
    connection: { title: "Connectez et testez", description: "L’onglet Connection gère les identifiants de services externes et propose une action de test lorsque le plugin en fournit une." },
  },
  connectionRecovery: {
    form: { title: "Revérifiez créateur et état", description: "Commencez par le handle exact du créateur et l’état actuel de connexion. Offline, authentification et salle indisponible demandent des récupérations différentes." },
    explorer: { title: "Vérifiez la salle LIVE", description: "Actualisez la liste des salles et confirmez que la salle est accessible. Une salle absente peut être hors ligne, restreinte ou simplement indisponible aux invités." },
    plugins: { title: "Vérifiez les intégrations après reconnexion", description: "Les intégrations de connexion peuvent avoir du retard après une reconnexion. Vérifiez leur santé avant de supposer que les automatisations ou overlays ont échoué." },
  },
  feedInspector: {
    controls: { title: "Ouvrez l’inspecteur", description: "Utilisez le contrôle d’inspecteur dans la barre du feed. Il ne met pas le LIVE en pause et ne change pas les événements enregistrés." },
    panel: { title: "Comprenez le panneau", description: "L’inspecteur contient des onglets spectateurs, détails d’événement et profil. Son contenu suit l’événement ou le spectateur sélectionné." },
    viewers: { title: "Comparez les spectateurs actifs", description: "Basculez entre meilleurs spectateurs et likes pour voir qui stimule l’activité de la session actuelle." },
    details: { title: "Inspectez un événement", description: "Les détails d’événement incluent comptes, points, heure et preuves du message, utiles pour diagnostiquer le comportement des automatisations." },
    profile: { title: "Ouvrez un profil de spectateur", description: "L’onglet profil résume le spectateur et ses messages récents sans exposer de données privées du compte." },
  },
  backupRestore: {
    tools: { title: "Choisissez la source de restauration", description: "Les sauvegardes Cloud et les fichiers locaux sont des chemins de récupération séparés. Choisissez la source avant de vérifier ce qui changera." },
    list: { title: "Restaurez délibérément", description: "Une ligne de sauvegarde peut télécharger, restaurer ou supprimer. Restaurer montre toujours une révision des données affectées avant application." },
    plan: { title: "Vérifiez le résultat", description: "Après restauration, vérifiez plan, profils, paramètres et automatisations clés. Une restauration réussie ne revalide pas tous les identifiants externes." },
  },
} satisfies TourCatalog;
