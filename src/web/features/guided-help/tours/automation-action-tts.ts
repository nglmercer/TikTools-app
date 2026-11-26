import { defineTour } from '../model/define-tour.ts';

export default defineTour({
  id: 'automationActionTts',
  version: 1,
  route: '/automations',
  scope: 'automations',
  tags: ['actions'],
  icon: 'volume',
  shape: 'action',
  requirements: { automationActionExample: { tag: 'tts' } },
  steps: {
    actionsTab: {
      es: ['Acciones de voz (TTS)', 'Las acciones TTS leen texto en voz alta: mensajes de chat, agradecimientos o anuncios con la voz que elijas.'],
      en: ['Voice actions (TTS)', 'TTS actions read text aloud: chat messages, thank-yous or announcements in the voice you choose.'],
      pt: ['Ações de voz (TTS)', 'Ações TTS leem texto em voz alta: mensagens de chat, agradecimentos ou anúncios.'],
      fr: ['Actions vocales (TTS)', 'Les actions TTS lisent du texte à voix haute : messages, remerciements ou annonces.'],
      ja: ['音声アクション (TTS)', 'TTSアクションはテキストを読み上げます: チャット、お礼、アナウンスを選んだ声で。'],
      zh: ['语音操作 (TTS)', 'TTS 操作朗读文本: 聊天消息、感谢或公告。'],
    },
    actions: {
      es: ['Encuentra tus acciones TTS', 'Una misma voz puede servir a muchos eventos. Editarla cambia cómo suenan todos a la vez.'],
      en: ['Find your TTS actions', 'One voice can serve many events. Editing it changes how all of them sound at once.'],
      pt: ['Encontre suas ações TTS', 'Uma mesma voz pode servir a muitos eventos. Editá-la muda como todos soam ao mesmo tempo.'],
      fr: ['Trouvez vos actions TTS', 'Une même voix peut servir de nombreux événements. La modifier change tous leurs sons à la fois.'],
      ja: ['TTSアクションを確認', '1つの声を多くのイベントで共有できます。編集すると全イベントの音声が一度に変わります。'],
      zh: ['找到 TTS 操作', '一个语音可服务多个事件,编辑后所有事件的声音同时改变。'],
    },
    settings: {
      es: ['Configura voz y texto', 'Define voz, velocidad y plantilla del texto. Las variables entre llaves se reemplazan con datos del evento, como el nombre del espectador.'],
      en: ['Configure voice and text', 'Set voice, speed and text template. Brace variables are replaced with event data, like the viewer name.'],
      pt: ['Configure voz e texto', 'Defina voz, velocidade e modelo de texto. Variáveis entre chaves se substituem por dados do evento.'],
      fr: ['Configurez voix et texte', 'Définissez voix, vitesse et modèle de texte. Les variables entre accolades sont remplacées par les données de l’événement.'],
      ja: ['声とテキストを設定', '声・速度・テキストのテンプレートを定義します。中括弧の変数はイベントデータに置き換わります。'],
      zh: ['配置声音和文本', '定义声音、速度和文本模板。大括号变量会被替换为事件数据。'],
    },
    permissions: {
      es: ['Revisa los permisos', 'TTS usa el motor de voz del sistema o del plugin. Revisa aquí qué acceso necesita antes de confiar en ella.'],
      en: ['Review permissions', 'TTS uses the system or plugin speech engine. Review what access it needs here before relying on it.'],
      pt: ['Revise as permissões', 'TTS usa o motor de voz do sistema ou plugin. Revise aqui o acesso necessário.'],
      fr: ['Vérifiez les permissions', 'TTS utilise le moteur vocal du système ou du plugin. Vérifiez ici l’accès requis.'],
      ja: ['権限を確認', 'TTSはシステムまたはプラグインの音声エンジンを使います。必要なアクセスをここで確認してください。'],
      zh: ['检查权限', 'TTS 使用系统或插件语音引擎,在此确认所需访问权限。'],
    },
  },
  card: {
      es: ['Acciones de voz (TTS)', 'Lectura en voz alta de mensajes y anuncios.'],
      en: ['Voice actions (TTS)', 'Reading messages and announcements aloud.'],
      pt: ['Ações de voz (TTS)', 'Leitura em voz alta de mensagens e anúncios.'],
      fr: ['Actions vocales (TTS)', 'Lecture à voix haute de messages et annonces.'],
      ja: ['音声アクション (TTS)', 'メッセージとアナウンスの読み上げ。'],
      zh: ['语音操作 (TTS)', '朗读消息和公告。'],
    },
});
