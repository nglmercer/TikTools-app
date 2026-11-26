import { defineTour } from '../model/define-tour.ts';

export default defineTour({
  id: 'automationFilters',
  version: 1,
  route: '/automations',
  scope: 'automations',
  tags: ['events'],
  icon: 'filter',
  shape: 'custom',
  layout: [
    { id: 'eventsTab', target: 'automation-tab-events', policy: 'cancel', prepareActions: ['automation.selectEvents'], prepare: ['[data-guide="automation-editor-close"]'] },
    { id: 'operators', target: 'event-editor-conditions', policy: 'skip', prepareActions: ['automation.openEvent'], prepare: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]', '[data-guide="automation-events"] tbody tr'] },
    { id: 'quantifiers', target: 'event-editor-run', policy: 'skip', prepareActions: ['automation.openEvent'], prepare: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]', '[data-guide="automation-events"] tbody tr'] },
    { id: 'verify', target: 'automation-runs', policy: 'skip', prepare: ['[data-guide="automation-editor-close"]'] },
  ],
  steps: {
    eventsTab: {
      es: ['Qué son los filtros', 'Las condiciones deciden si un evento se ejecuta o se omite: son la diferencia entre una automatización útil y ruido constante.'],
      en: ['What filters are', 'Conditions decide whether an event runs or is skipped: they are the difference between a useful automation and constant noise.'],
      pt: ['O que são filtros', 'Condições decidem se um evento executa ou é omitido: são a diferença entre automação útil e ruído constante.'],
      fr: ['Que sont les filtres', 'Les conditions décident si un événement s’exécute ou est ignoré : la différence entre automatisation utile et bruit.'],
      ja: ['フィルタとは', '条件はイベントを実行するかスキップするかを決めます: 有用な自動化とノイズの違いです。'],
      zh: ['什么是过滤器', '条件决定事件执行还是跳过: 是有用自动化与噪音的区别。'],
    },
    operators: {
      es: ['Operadores de comparación', 'Cada condición compara un dato del evento con un valor: igual, contiene, mayor que o expresión regular según el tipo de dato.'],
      en: ['Comparison operators', 'Each condition compares event data against a value: equals, contains, greater than or regex depending on the data type.'],
      pt: ['Operadores de comparação', 'Cada condição compara um dado do evento com um valor: igual, contém, maior que ou regex.'],
      fr: ['Opérateurs de comparaison', 'Chaque condition compare une donnée à une valeur : égal, contient, supérieur ou regex selon le type.'],
      ja: ['比較演算子', '各条件はイベントデータを値と比較します: 等しい・含む・より大きい・正規表現。'],
      zh: ['比较运算符', '每个条件将事件数据与值比较: 等于、包含、大于或正则。'],
    },
    quantifiers: {
      es: ['Cuantificadores any y all', 'Con all todas las condiciones deben cumplirse; con any basta una. Combínalos con el modo and/or del evento para reglas precisas.'],
      en: ['any and all quantifiers', 'With all, every condition must pass; with any, one is enough. Combine them with the event and/or mode for precise rules.'],
      pt: ['Quantificadores any e all', 'Com all todas as condições devem passar; com any basta uma. Combine com o modo and/or do evento.'],
      fr: ['Quantificateurs any et all', 'Avec all toutes les conditions doivent passer ; avec any une suffit. Combinez avec le mode and/or.'],
      ja: ['any と all 量指定子', 'all では全条件が必要、any では1つで十分。イベントの and/or モードと組み合わせます。'],
      zh: ['any 和 all 量词', 'all 要求所有条件通过,any 只需一个,配合事件的 and/or 模式。'],
    },
    verify: {
      es: ['Verifica en el historial', 'Las ejecuciones omitidas muestran qué condición falló. Revisa aquí antes de aflojar un filtro que parece no disparar nunca.'],
      en: ['Verify in the history', 'Skipped runs show which condition failed. Check here before loosening a filter that seems to never fire.'],
      pt: ['Verifique no histórico', 'Execuções omitidas mostram qual condição falhou. Revise aqui antes de afrouxar um filtro.'],
      fr: ['Vérifiez l’historique', 'Les exécutions ignorées montrent quelle condition a échoué. Vérifiez ici avant d’assouplir un filtre.'],
      ja: ['履歴で確認', 'スキップされた実行は失敗した条件を示します。フィルタを緩める前にここを確認してください。'],
      zh: ['在历史中验证', '跳过的执行显示哪个条件失败,放宽过滤器前先查看这里。'],
    },
  },
  card: {
      es: ['Filtros y condiciones', 'Operadores, cuantificadores y verificación de omisiones.'],
      en: ['Filters and conditions', 'Operators, quantifiers and skip verification.'],
      pt: ['Filtros e condições', 'Operadores, quantificadores e verificação de omissões.'],
      fr: ['Filtres et conditions', 'Opérateurs, quantificateurs et vérification des omissions.'],
      ja: ['フィルタと条件', '演算子、量指定子、スキップの検証。'],
      zh: ['过滤器和条件', '运算符、量词和跳过验证。'],
    },
});
