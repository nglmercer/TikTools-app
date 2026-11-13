/** Published Markdown languages. Application catalogs have their own lifecycle. */
export const docLocales = ['en', 'es', 'pt', 'fr', 'zh', 'ja'] as const;
export type DocLocale = typeof docLocales[number];
export function documentLocale(lang: string): DocLocale {
  const base = lang.toLowerCase().split('-')[0];
  const locale = base === 'jp' ? 'ja' : base;
  return docLocales.find((candidate) => candidate === locale) ?? 'en';
}

export const docCopy = {
  en: {
    label: 'English', demo: 'Demo', lastUpdated: 'Last updated', darkTitle: 'Switch to dark theme', lightTitle: 'Switch to light theme', description: 'TikTools user guides, developer documentation and an interactive Vue demo.',
    audience: 'Audience', user: 'User', developer: 'Developer', releases: 'Releases', edit: 'Edit this page', previous: 'Previous', next: 'Next', outline: 'On this page', navigation: 'Navigation', top: 'Return to top', language: 'Change language', theme: 'Theme', skip: 'Skip to content',
    search: 'Search', searchLabel: 'Search documentation', details: 'Display details', reset: 'Clear search', closeSearch: 'Close search', noResults: 'No results', select: 'select', enter: 'enter', navigate: 'navigate', up: 'up arrow', down: 'down arrow', close: 'close', escape: 'escape',
    loadDemo: 'Load interactive demo', closeDemo: 'Close demo', fullDemo: 'Open full demo',
    groups: ['Getting started', 'LIVE', 'Points and statistics', 'Automations', 'Plugins and audio', 'OBS and overlays', 'Cloud and collaboration', 'Preferences and support', 'Development setup', 'Architecture', 'API and collaboration', 'Plugins', 'Overlays', 'Components', 'Quality and maintenance'],
    reference: { source: 'Source', property: 'Property', type: 'Type', presence: 'Presence', default: 'Default', required: 'Required', optional: 'Optional', events: 'Own events (native control attributes are forwarded according to the component)', noEvents: 'No own declared events.', models: 'Models', noModels: 'No own v-model.', method: 'Method', effects: 'Effects', destructive: 'Destructive' },
  },
  es: {
    label: 'Español', demo: 'Demo', lastUpdated: 'Última actualización', darkTitle: 'Cambiar al tema oscuro', lightTitle: 'Cambiar al tema claro', description: 'Guías de usuario, documentación técnica y demo interactiva de TikTools.',
    audience: 'Audiencia', user: 'Usuario', developer: 'Desarrollo', releases: 'Versiones', edit: 'Editar esta página', previous: 'Anterior', next: 'Siguiente', outline: 'En esta página', navigation: 'Navegación', top: 'Volver arriba', language: 'Cambiar idioma', theme: 'Tema', skip: 'Saltar al contenido',
    search: 'Buscar', searchLabel: 'Buscar documentación', details: 'Mostrar detalles', reset: 'Borrar búsqueda', closeSearch: 'Cerrar búsqueda', noResults: 'Sin resultados', select: 'seleccionar', enter: 'intro', navigate: 'navegar', up: 'flecha arriba', down: 'flecha abajo', close: 'cerrar', escape: 'escape',
    loadDemo: 'Cargar demo interactiva', closeDemo: 'Cerrar demo', fullDemo: 'Abrir demo completa',
    groups: ['Empezar', 'LIVE', 'Puntos y estadísticas', 'Automatizaciones', 'Plugins y audio', 'OBS y overlays', 'Cloud y colaboración', 'Preferencias y soporte', 'Preparación', 'Arquitectura', 'API y colaboración', 'Plugins', 'Overlays', 'Componentes', 'Calidad y mantenimiento'],
    reference: { source: 'Código fuente', property: 'Propiedad', type: 'Tipo', presence: 'Presencia', default: 'Predeterminado', required: 'Obligatoria', optional: 'Opcional', events: 'Eventos propios (los atributos de control nativo se reenvían según el componente)', noEvents: 'Sin eventos propios declarados.', models: 'Modelos', noModels: 'Sin v-model propio.', method: 'Método', effects: 'Efectos', destructive: 'Destructivo' },
  },
  pt: {
    label: 'Português', demo: 'Demo', lastUpdated: 'Última atualização', darkTitle: 'Mudar para o tema escuro', lightTitle: 'Mudar para o tema claro', description: 'Guias de uso, documentação para desenvolvedores e demo interativa do TikTools.',
    audience: 'Público', user: 'Usuário', developer: 'Desenvolvimento', releases: 'Versões', edit: 'Editar esta página', previous: 'Anterior', next: 'Próxima', outline: 'Nesta página', navigation: 'Navegação', top: 'Voltar ao topo', language: 'Alterar idioma', theme: 'Tema', skip: 'Ir para o conteúdo',
    search: 'Buscar', searchLabel: 'Buscar na documentação', details: 'Mostrar detalhes', reset: 'Limpar busca', closeSearch: 'Fechar busca', noResults: 'Nenhum resultado', select: 'selecionar', enter: 'enter', navigate: 'navegar', up: 'seta para cima', down: 'seta para baixo', close: 'fechar', escape: 'escape',
    loadDemo: 'Carregar demo interativa', closeDemo: 'Fechar demo', fullDemo: 'Abrir demo completa',
    groups: ['Começar', 'LIVE', 'Pontos e estatísticas', 'Automações', 'Plugins e áudio', 'OBS e overlays', 'Cloud e colaboração', 'Preferências e suporte', 'Ambiente de desenvolvimento', 'Arquitetura', 'API e colaboração', 'Plugins', 'Overlays', 'Componentes', 'Qualidade e manutenção'],
    reference: { source: 'Código-fonte', property: 'Propriedade', type: 'Tipo', presence: 'Presença', default: 'Padrão', required: 'Obrigatória', optional: 'Opcional', events: 'Eventos próprios (os atributos de controles nativos são repassados conforme o componente)', noEvents: 'Nenhum evento próprio declarado.', models: 'Modelos', noModels: 'Sem v-model próprio.', method: 'Método', effects: 'Efeitos', destructive: 'Destrutivo' },
  },
  fr: {
    label: 'Français', demo: 'Démo', lastUpdated: 'Dernière mise à jour', darkTitle: 'Activer le thème sombre', lightTitle: 'Activer le thème clair', description: 'Guides d’utilisation, documentation technique et démo interactive de TikTools.',
    audience: 'Public', user: 'Utilisateur', developer: 'Développement', releases: 'Versions', edit: 'Modifier cette page', previous: 'Précédent', next: 'Suivant', outline: 'Sur cette page', navigation: 'Navigation', top: 'Retour en haut', language: 'Changer de langue', theme: 'Thème', skip: 'Aller au contenu',
    search: 'Rechercher', searchLabel: 'Rechercher dans la documentation', details: 'Afficher les détails', reset: 'Effacer la recherche', closeSearch: 'Fermer la recherche', noResults: 'Aucun résultat', select: 'sélectionner', enter: 'entrée', navigate: 'naviguer', up: 'flèche haut', down: 'flèche bas', close: 'fermer', escape: 'échap',
    loadDemo: 'Charger la démo interactive', closeDemo: 'Fermer la démo', fullDemo: 'Ouvrir la démo complète',
    groups: ['Premiers pas', 'LIVE', 'Points et statistiques', 'Automatisations', 'Plugins et audio', 'OBS et overlays', 'Cloud et collaboration', 'Préférences et assistance', 'Environnement de développement', 'Architecture', 'API et collaboration', 'Plugins', 'Overlays', 'Composants', 'Qualité et maintenance'],
    reference: { source: 'Code source', property: 'Propriété', type: 'Type', presence: 'Présence', default: 'Valeur par défaut', required: 'Obligatoire', optional: 'Facultative', events: 'Événements propres (les attributs des contrôles natifs sont transmis selon le composant)', noEvents: 'Aucun événement propre déclaré.', models: 'Modèles', noModels: 'Aucun v-model propre.', method: 'Méthode', effects: 'Effets', destructive: 'Destructif' },
  },
  zh: {
    label: '简体中文', demo: '演示', lastUpdated: '最后更新', darkTitle: '切换到深色主题', lightTitle: '切换到浅色主题', description: 'TikTools 用户指南、开发文档和交互式演示。',
    audience: '阅读对象', user: '用户', developer: '开发者', releases: '版本说明', edit: '编辑本页', previous: '上一页', next: '下一页', outline: '本页目录', navigation: '导航', top: '返回顶部', language: '切换语言', theme: '主题', skip: '跳转到内容',
    search: '搜索', searchLabel: '搜索文档', details: '显示详情', reset: '清空搜索', closeSearch: '关闭搜索', noResults: '没有找到结果', select: '选择', enter: '回车', navigate: '切换', up: '向上键', down: '向下键', close: '关闭', escape: 'Esc 键',
    loadDemo: '加载交互式演示', closeDemo: '关闭演示', fullDemo: '打开完整演示',
    groups: ['快速入门', 'LIVE', '积分与统计', '自动化', '插件与音频', 'OBS 与叠加层', 'Cloud 与协作', '偏好设置与支持', '开发环境', '架构', 'API 与协作', '插件', '叠加层', '组件', '质量与维护'],
    reference: { source: '源代码', property: '属性', type: '类型', presence: '是否必需', default: '默认值', required: '必需', optional: '可选', events: '组件自身的事件（原生控件属性按组件约定透传）', noEvents: '未声明组件自身的事件。', models: '模型', noModels: '没有组件自身的 v-model。', method: '方法', effects: '副作用', destructive: '破坏性操作' },
  },
  ja: {
    label: '日本語', demo: 'デモ', lastUpdated: '最終更新', darkTitle: 'ダークテーマに切り替え', lightTitle: 'ライトテーマに切り替え', description: 'TikTools のユーザーガイド、開発者向けドキュメント、操作できるデモ。',
    audience: '対象読者', user: 'ユーザー', developer: '開発者', releases: 'リリース', edit: 'このページを編集', previous: '前のページ', next: '次のページ', outline: 'このページの目次', navigation: 'ナビゲーション', top: 'ページ上部に戻る', language: '言語を切り替え', theme: 'テーマ', skip: '本文へ移動',
    search: '検索', searchLabel: 'ドキュメントを検索', details: '詳細を表示', reset: '検索をクリア', closeSearch: '検索を閉じる', noResults: '結果が見つかりません', select: '選択', enter: 'Enter キー', navigate: '移動', up: '上矢印キー', down: '下矢印キー', close: '閉じる', escape: 'Esc キー',
    loadDemo: '操作できるデモを読み込む', closeDemo: 'デモを閉じる', fullDemo: '完全版のデモを開く',
    groups: ['はじめに', 'LIVE', 'ポイントと統計', '自動化', 'プラグインと音声', 'OBS とオーバーレイ', 'Cloud と共同作業', '環境設定とサポート', '開発環境', 'アーキテクチャ', 'API と共同作業', 'プラグイン', 'オーバーレイ', 'コンポーネント', '品質と保守'],
    reference: { source: 'ソースコード', property: 'プロパティ', type: '型', presence: '必須・任意', default: '既定値', required: '必須', optional: '任意', events: '独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）', noEvents: '独自のイベントは宣言されていません。', models: 'モデル', noModels: '独自の v-model はありません。', method: 'メソッド', effects: '副作用', destructive: '破壊的操作' },
  },
} as const;


/** Locale paths are canonical IDs, independent of translated titles and anchors. */
export function localizedDocumentPath(path: string): { locale: DocLocale; tail: string } | undefined {
  const slash = path.indexOf('/');
  const prefix = path.slice(0, slash);
  const locale = docLocales.find((locale) => locale === prefix);
  return locale ? { locale, tail: path.slice(slash + 1) } : undefined;
}
