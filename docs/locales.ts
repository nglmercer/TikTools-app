/** Published Markdown languages. Application catalogs have their own lifecycle. */
export const docLocales = ['en', 'es', 'pt', 'fr'] as const;
export type DocLocale = typeof docLocales[number];
export function documentLocale(lang: string): DocLocale {
  const locale = lang.split('-')[0];
  return docLocales.find((candidate) => candidate === locale) ?? 'en';
}

export const docCopy = {
  en: {
    label: 'English', description: 'TikTools user guides, developer documentation and an interactive Vue demo.',
    audience: 'Audience', user: 'User', developer: 'Developer', releases: 'Releases', edit: 'Edit this page', previous: 'Previous', next: 'Next', outline: 'On this page', navigation: 'Navigation', top: 'Return to top', language: 'Change language', theme: 'Theme', skip: 'Skip to content',
    search: 'Search', searchLabel: 'Search documentation', details: 'Display details', reset: 'Clear search', closeSearch: 'Close search', noResults: 'No results', select: 'select', enter: 'enter', navigate: 'navigate', up: 'up arrow', down: 'down arrow', close: 'close', escape: 'escape',
    loadDemo: 'Load interactive demo', closeDemo: 'Close demo', fullDemo: 'Open full demo',
    groups: ['Getting started', 'LIVE', 'Points and statistics', 'Automations', 'Plugins and audio', 'OBS and overlays', 'Cloud and collaboration', 'Preferences and support', 'Development setup', 'Architecture', 'API and collaboration', 'Plugins', 'Overlays', 'Components', 'Quality and maintenance'],
    reference: { source: 'Source', property: 'Property', type: 'Type', presence: 'Presence', default: 'Default', required: 'Required', optional: 'Optional', events: 'Own events (native control attributes are forwarded according to the component)', noEvents: 'No own declared events.', models: 'Models', noModels: 'No own v-model.', method: 'Method', effects: 'Effects', destructive: 'Destructive' },
  },
  es: {
    label: 'Español', description: 'Guías de usuario, documentación técnica y demo interactiva de TikTools.',
    audience: 'Audiencia', user: 'Usuario', developer: 'Desarrollo', releases: 'Versiones', edit: 'Editar esta página', previous: 'Anterior', next: 'Siguiente', outline: 'En esta página', navigation: 'Navegación', top: 'Volver arriba', language: 'Cambiar idioma', theme: 'Tema', skip: 'Saltar al contenido',
    search: 'Buscar', searchLabel: 'Buscar documentación', details: 'Mostrar detalles', reset: 'Borrar búsqueda', closeSearch: 'Cerrar búsqueda', noResults: 'Sin resultados', select: 'seleccionar', enter: 'intro', navigate: 'navegar', up: 'flecha arriba', down: 'flecha abajo', close: 'cerrar', escape: 'escape',
    loadDemo: 'Cargar demo interactiva', closeDemo: 'Cerrar demo', fullDemo: 'Abrir demo completa',
    groups: ['Empezar', 'LIVE', 'Puntos y estadísticas', 'Automatizaciones', 'Plugins y audio', 'OBS y overlays', 'Cloud y colaboración', 'Preferencias y soporte', 'Preparación', 'Arquitectura', 'API y colaboración', 'Plugins', 'Overlays', 'Componentes', 'Calidad y mantenimiento'],
    reference: { source: 'Código fuente', property: 'Propiedad', type: 'Tipo', presence: 'Presencia', default: 'Predeterminado', required: 'Obligatoria', optional: 'Opcional', events: 'Eventos propios (los atributos de control nativo se reenvían según el componente)', noEvents: 'Sin eventos propios declarados.', models: 'Modelos', noModels: 'Sin v-model propio.', method: 'Método', effects: 'Efectos', destructive: 'Destructivo' },
  },
  pt: {
    label: 'Português', description: 'Guias de uso, documentação para desenvolvedores e demo interativa do TikTools.',
    audience: 'Público', user: 'Usuário', developer: 'Desenvolvimento', releases: 'Versões', edit: 'Editar esta página', previous: 'Anterior', next: 'Próxima', outline: 'Nesta página', navigation: 'Navegação', top: 'Voltar ao topo', language: 'Alterar idioma', theme: 'Tema', skip: 'Ir para o conteúdo',
    search: 'Buscar', searchLabel: 'Buscar na documentação', details: 'Mostrar detalhes', reset: 'Limpar busca', closeSearch: 'Fechar busca', noResults: 'Nenhum resultado', select: 'selecionar', enter: 'enter', navigate: 'navegar', up: 'seta para cima', down: 'seta para baixo', close: 'fechar', escape: 'escape',
    loadDemo: 'Carregar demo interativa', closeDemo: 'Fechar demo', fullDemo: 'Abrir demo completa',
    groups: ['Começar', 'LIVE', 'Pontos e estatísticas', 'Automações', 'Plugins e áudio', 'OBS e overlays', 'Cloud e colaboração', 'Preferências e suporte', 'Ambiente de desenvolvimento', 'Arquitetura', 'API e colaboração', 'Plugins', 'Overlays', 'Componentes', 'Qualidade e manutenção'],
    reference: { source: 'Código-fonte', property: 'Propriedade', type: 'Tipo', presence: 'Presença', default: 'Padrão', required: 'Obrigatória', optional: 'Opcional', events: 'Eventos próprios (os atributos de controles nativos são repassados conforme o componente)', noEvents: 'Nenhum evento próprio declarado.', models: 'Modelos', noModels: 'Sem v-model próprio.', method: 'Método', effects: 'Efeitos', destructive: 'Destrutivo' },
  },
  fr: {
    label: 'Français', description: 'Guides d’utilisation, documentation technique et démo interactive de TikTools.',
    audience: 'Public', user: 'Utilisateur', developer: 'Développement', releases: 'Versions', edit: 'Modifier cette page', previous: 'Précédent', next: 'Suivant', outline: 'Sur cette page', navigation: 'Navigation', top: 'Retour en haut', language: 'Changer de langue', theme: 'Thème', skip: 'Aller au contenu',
    search: 'Rechercher', searchLabel: 'Rechercher dans la documentation', details: 'Afficher les détails', reset: 'Effacer la recherche', closeSearch: 'Fermer la recherche', noResults: 'Aucun résultat', select: 'sélectionner', enter: 'entrée', navigate: 'naviguer', up: 'flèche haut', down: 'flèche bas', close: 'fermer', escape: 'échap',
    loadDemo: 'Charger la démo interactive', closeDemo: 'Fermer la démo', fullDemo: 'Ouvrir la démo complète',
    groups: ['Premiers pas', 'LIVE', 'Points et statistiques', 'Automatisations', 'Plugins et audio', 'OBS et overlays', 'Cloud et collaboration', 'Préférences et assistance', 'Environnement de développement', 'Architecture', 'API et collaboration', 'Plugins', 'Overlays', 'Composants', 'Qualité et maintenance'],
    reference: { source: 'Code source', property: 'Propriété', type: 'Type', presence: 'Présence', default: 'Valeur par défaut', required: 'Obligatoire', optional: 'Facultative', events: 'Événements propres (les attributs des contrôles natifs sont transmis selon le composant)', noEvents: 'Aucun événement propre déclaré.', models: 'Modèles', noModels: 'Aucun v-model propre.', method: 'Méthode', effects: 'Effets', destructive: 'Destructif' },
  },
} as const;
