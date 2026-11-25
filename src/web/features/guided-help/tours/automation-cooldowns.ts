import { defineTour } from '../model/define-tour.ts';

export default defineTour({
  id: 'automationCooldowns',
  version: 1,
  route: '/automations',
  scope: 'automations',
  tags: ['events'],
  icon: 'clock',
  shape: 'custom',
  layout: [
    { id: 'eventsTab', target: 'automation-tab-events', policy: 'cancel', prepare: ['[data-guide="automation-editor-close"]'] },
    { id: 'scope', target: 'event-editor-cooldown', policy: 'skip', prepare: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]', '[data-guide="automation-new-record"]'] },
    { id: 'runMode', target: 'event-editor-run', policy: 'skip', prepare: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]', '[data-guide="automation-new-record"]'] },
    { id: 'verify', target: 'automation-runs', policy: 'skip', prepare: ['[data-guide="automation-editor-close"]'] },
  ],
  steps: {
    eventsTab: {
      es: ['Por qué importan los cooldowns', 'Sin cooldown, un evento popular puede repetir el mismo efecto decenas de veces por minuto y arruinar el directo.'],
      en: ['Why cooldowns matter', 'Without a cooldown, a popular event can repeat the same effect dozens of times per minute and ruin the stream.'],
      pt: ['Por que cooldowns importam', 'Sem cooldown, um evento popular pode repetir o mesmo efeito dezenas de vezes por minuto.'],
      fr: ['Pourquoi les cooldowns', 'Sans cooldown, un événement populaire peut répéter le même effet des dizaines de fois par minute.'],
      ja: ['クールダウンの重要性', 'クールダウンなしでは、人気イベントが同じエフェクトを毎分何十回も繰り返し配信を損ないます。'],
      zh: ['冷却为何重要', '没有冷却,热门事件可能每分钟重复同一效果几十次,毁掉直播。'],
    },
    scope: {
      es: ['Alcance global o por usuario', 'El cooldown global limita el evento entero; el cooldown por usuario deja que cada espectador lo dispare una vez en su ventana.'],
      en: ['Global or per-user scope', 'Global cooldown limits the whole event; per-user cooldown lets each viewer fire it once within their window.'],
      pt: ['Escopo global ou por usuário', 'Cooldown global limita o evento inteiro; por usuário permite que cada espectador dispare uma vez por janela.'],
      fr: ['Portée globale ou par utilisateur', 'Le cooldown global limite tout l’événement ; par utilisateur chacun le déclenche une fois par fenêtre.'],
      ja: ['グローバル/ユーザー範囲', 'グローバルはイベント全体を制限、ユーザーごとは各視聴者がウィンドウ内で1回発動できます。'],
      zh: ['全局或按用户范围', '全局限制整个事件;按用户允许每位观众在窗口内触发一次。'],
    },
    runMode: {
      es: ['Modo de ejecución all o random', 'Con all se ejecutan todas las acciones vinculadas en orden; con random se elige una al azar, ideal para respuestas variadas.'],
      en: ['all or random run mode', 'With all, every linked action runs in order; with random, one is picked at random — ideal for varied responses.'],
      pt: ['Modo all ou random', 'Com all todas as ações vinculadas executam em ordem; com random uma é escolhida ao acaso.'],
      fr: ['Mode all ou random', 'Avec all toutes les actions liées s’exécutent en ordre ; avec random une est choisie au hasard.'],
      ja: ['all と random モード', 'all は全アクションを順番に実行、random はランダムに1つ選択。多彩な応答に最適です。'],
      zh: ['all 或 random 模式', 'all 按顺序执行全部关联操作,random 随机选一个,适合多样回复。'],
    },
    verify: {
      es: ['Verifica omisiones', 'El historial marca las ejecuciones omitidas por cooldown. Si todo se omite, la ventana probablemente es demasiado larga.'],
      en: ['Verify skips', 'The history marks runs skipped by cooldown. If everything is skipped, the window is probably too long.'],
      pt: ['Verifique omissões', 'O histórico marca execuções omitidas por cooldown. Se tudo é omitido, a janela provavelmente é longa demais.'],
      fr: ['Vérifiez les omissions', 'L’historique marque les exécutions ignorées par cooldown. Si tout est ignoré, la fenêtre est trop longue.'],
      ja: ['スキップを確認', '履歴はクールダウンによるスキップを示します。全部スキップならウィンドウが長すぎます。'],
      zh: ['验证跳过', '历史标记因冷却跳过的执行。若全部跳过,窗口可能太长。'],
    },
  },
  card: {
      es: ['Cooldowns y modos', 'Alcance global/por usuario y ejecución all o random.'],
      en: ['Cooldowns and run modes', 'Global/per-user scope and all or random execution.'],
      pt: ['Cooldowns e modos', 'Escopo global/por usuário e execução all ou random.'],
      fr: ['Cooldowns et modes', 'Portée globale/par utilisateur et exécution all ou random.'],
      ja: ['クールダウンとモード', 'グローバル/ユーザー範囲と all/random 実行。'],
      zh: ['冷却和运行模式', '全局/按用户范围与 all/random 执行。'],
    },
});
