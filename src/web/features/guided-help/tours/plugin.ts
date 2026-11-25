import { defineTour } from '../model/define-tour.ts';

export default defineTour({
  id: 'plugin',
  version: 1,
  route: '/plugins',
  scope: 'plugins',
  task: 'plugin.installed',
  tags: ['plugins'],
  icon: 'puzzle',
  shape: 'custom',
  layout: [
    { id: 'catalog', target: 'plugin-catalog', policy: 'cancel' },
    { id: 'search', target: 'plugins-search', policy: 'skip' },
    { id: 'install', target: 'plugins-install', policy: 'skip', capabilities: ['plugins.packages.install'] },
  ],
  steps: {
    catalog: {
      es: ['Instalados y Explore', 'Revisa paquetes instalados o explora entradas compatibles. Los plugins nativos y de proceso mantienen los permisos del usuario en el sistema.'],
      en: ['Installed and Explore', 'Review installed packages or browse compatible catalog entries. Native and process plugins retain the operating-system permissions of their user.'],
      pt: ['Instalados e Explore', 'Revise pacotes instalados ou explore itens compatíveis. Plugins nativos e de processo mantêm as permissões do usuário no sistema.'],
      fr: ['Installés et Explore', 'Examinez les paquets ou parcourez les entrées compatibles. Les plugins natifs et processus conservent les permissions système de leur utilisateur.'],
      ja: ['インストール済みと「探す」', 'インストール済みのパッケージを確認するか、互換性のあるカタログ項目を探してください。ネイティブプラグインとプロセス型プラグインは、実行ユーザーのOS権限を保持します。'],
      zh: ['已安装与探索', '审查已安装的包，或浏览兼容的目录条目。原生插件和进程插件保留其用户的操作系统权限。'],
    },
    search: {
      es: ['Busca un paquete', 'Busca e inspecciona editor, versión y requisitos. Un catálogo vacío y un backend inaccesible son estados diferentes.'],
      en: ['Find a package', 'Search and inspect publisher, version and requirements. An empty catalog and an unavailable backend are different states.'],
      pt: ['Encontre um pacote', 'Pesquise e confira editor, versão e requisitos. Catálogo vazio e backend indisponível são estados diferentes.'],
      fr: ['Trouver un paquet', 'Recherchez et vérifiez éditeur, version et prérequis. Un catalogue vide diffère d’un service indisponible.'],
      ja: ['パッケージを探す', '検索し、発行者、バージョン、必要条件を確認してください。カタログが空の状態と、バックエンドを利用できない状態は異なります。'],
      zh: ['查找包', '搜索并检查发布者、版本和要求。目录为空和后端不可用是两种不同状态。'],
    },
    install: {
      es: ['Revisa la instalación', 'Instalar exige revisar firmas y permisos. La guía no instala, aprueba permisos nuevos ni ejecuta acciones de prueba.'],
      en: ['Review installation', 'Installation has its own signature and permission review. The guide does not install, approve new permissions or run a test action.'],
      pt: ['Revise a instalação', 'Instalar exige revisão de assinaturas e permissões. O guia não instala, aprova permissões nem executa ações de teste.'],
      fr: ['Examiner l’installation', 'L’installation exige un examen des signatures et permissions. Le guide n’installe rien et n’exécute aucune action de test.'],
      ja: ['インストールを確認', 'インストールには、署名と権限の個別確認があります。このガイドでは、インストール、新しい権限の承認、アクションのテスト実行は行いません。'],
      zh: ['审查安装', '安装有独立的签名和权限审查流程。指南不会安装、批准新权限或运行测试操作。'],
    },
  },
  card: {
      es: ['Plugins', 'Explora, inspecciona e instala plugins de forma segura.'],
      en: ['Plugins', 'Explore, inspect and install plugins safely.'],
      pt: ['Plugins', 'Explore, inspecione e instale plugins com segurança.'],
      fr: ['Plugins', 'Explorez, inspectez et installez des plugins en toute sécurité.'],
      ja: ['プラグイン', 'プラグインを安全に探索、検査、インストールします。'],
      zh: ['插件', '安全地探索、检查和安装插件。'],
    },
});
