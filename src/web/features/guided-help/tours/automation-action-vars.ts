import { defineTour } from '../model/define-tour.ts';

export default defineTour({
  id: 'automationActionVars',
  version: 1,
  route: '/automations',
  scope: 'automations',
  tags: ['actions', 'variables'],
  icon: 'code',
  shape: 'action',
  steps: {
    actionsTab: {
      es: ['Variables dentro de acciones', 'Las acciones leen variables compartidas y capturas del evento: nombres, conteos y valores que cambian en cada disparo.'],
      en: ['Variables inside actions', 'Actions read shared variables and event captures: names, counts and values that change on every run.'],
      pt: ['Variáveis dentro de ações', 'Ações leem variáveis compartilhadas e capturas do evento: nomes, contagens e valores que mudam a cada execução.'],
      fr: ['Variables dans les actions', 'Les actions lisent variables partagées et captures d’événement : noms, comptes et valeurs qui changent à chaque exécution.'],
      ja: ['アクション内の変数', 'アクションは共有変数とイベントのキャプチャを読みます: 実行ごとに変わる名前・カウント・値。'],
      zh: ['操作中的变量', '操作读取共享变量和事件捕获: 每次执行变化的名字、计数和值。'],
    },
    actions: {
      es: ['Encuentra acciones con variables', 'Las acciones que interpolan variables dependen de la pestaña Variables: un valor renombrado o borrado las afecta a todas.'],
      en: ['Find actions using variables', 'Actions interpolating variables depend on the Variables tab: a renamed or deleted value affects all of them.'],
      pt: ['Encontre ações com variáveis', 'Ações que interpolam variáveis dependem da aba Variáveis: um valor renomeado afeta todas.'],
      fr: ['Trouvez les actions à variables', 'Les actions interpolant des variables dépendent de l’onglet Variables : une valeur renommée les affecte toutes.'],
      ja: ['変数を使うアクションを確認', '変数を補間するアクションは Variables タブに依存します: 名前変更や削除は全アクションに影響します。'],
      zh: ['找到用变量的操作', '插值变量的操作依赖 Variables 标签页: 重命名或删除会影响全部。'],
    },
    settings: {
      es: ['Interpola valores en el texto', 'Usa las llaves de variable en los campos de texto: el nombre del espectador, el valor del regalo o un contador global se insertan al ejecutar.'],
      en: ['Interpolate values in text', 'Use variable braces in text fields: viewer name, gift value or a global counter get inserted at runtime.'],
      pt: ['Interpole valores no texto', 'Use chaves de variável nos campos: nome do espectador, valor do presente ou contador global se inserem ao executar.'],
      fr: ['Interpoler dans le texte', 'Utilisez les accolades dans les champs : nom du spectateur, valeur du cadeau ou compteur global insérés à l’exécution.'],
      ja: ['テキストに値を補間', 'テキストフィールドで変数の中括弧を使います: 視聴者名・ギフト価値・グローバルカウンタが実行時に挿入されます。'],
      zh: ['在文本中插值', '在文本字段使用变量大括号: 观众名、礼物价值或全局计数器在运行时插入。'],
    },
    permissions: {
      es: ['Secretos y permisos', 'Las variables secretas solo se escriben, nunca se leen en pantalla. Las acciones las reciben resueltas en tiempo de ejecución.'],
      en: ['Secrets and permissions', 'Secret variables are write-only and never shown. Actions receive them resolved at execution time.'],
      pt: ['Segredos e permissões', 'Variáveis secretas são somente escrita e nunca exibidas. As ações as recebem resolvidas em execução.'],
      fr: ['Secrets et permissions', 'Les variables secrètes sont en écriture seule, jamais affichées. Les actions les reçoivent résolues à l’exécution.'],
      ja: ['秘密と権限', '秘密変数は書き込み専用で表示されません。アクションは実行時に解決済みの値を受け取ります。'],
      zh: ['秘密与权限', '秘密变量只写不显示,操作在执行时获得解析后的值。'],
    },
  },
  card: {
      es: ['Variables en acciones', 'Interpolación de capturas y valores compartidos.'],
      en: ['Variables in actions', 'Interpolating captures and shared values.'],
      pt: ['Variáveis em ações', 'Interpolação de capturas e valores compartilhados.'],
      fr: ['Variables dans les actions', 'Interpolation de captures et valeurs partagées.'],
      ja: ['アクション内の変数', 'キャプチャと共有値の補間。'],
      zh: ['操作中的变量', '捕获和共享值的插值。'],
    },
});
