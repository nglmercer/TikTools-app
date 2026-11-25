import { defineTour } from '../model/define-tour.ts';

export default defineTour({
  id: 'automationEventChat',
  version: 1,
  route: '/automations',
  scope: 'automations',
  tags: ['events'],
  icon: 'chat',
  shape: 'event',
  steps: {
    eventsTab: {
      es: ['Chat como disparador', 'Los eventos de chat reaccionan a lo que escriben los espectadores: mensajes normales, emotes o comandos con prefijo como !dado.'],
      en: ['Chat as a trigger', 'Chat events react to what viewers type: plain messages, emotes, or prefixed commands like !dice.'],
      pt: ['Chat como gatilho', 'Eventos de chat reagem ao que os espectadores escrevem: mensagens, emotes ou comandos com prefixo como !dado.'],
      fr: ['Le chat comme déclencheur', 'Les événements de chat réagissent aux messages des spectateurs : messages, emotes ou commandes préfixées comme !dé.'],
      ja: ['チャットをトリガーに', 'チャットイベントは視聴者の入力に反応します: 通常メッセージ、エモート、!dice のようなプレフィックス付きコマンド。'],
      zh: ['聊天作为触发器', '聊天事件响应观众的输入: 普通消息、表情或 !dice 这样的前缀命令。'],
    },
    events: {
      es: ['Encuentra tus reglas de chat', 'Cada fila con disparador de chat muestra sus condiciones y acciones. El texto exacto del comando se define dentro del editor.'],
      en: ['Find your chat rules', 'Each row with a chat trigger shows its conditions and actions. The exact command text is defined inside the editor.'],
      pt: ['Encontre suas regras de chat', 'Cada linha com gatilho de chat mostra condições e ações. O texto exato do comando se define no editor.'],
      fr: ['Trouvez vos règles de chat', 'Chaque ligne avec un déclencheur de chat montre ses conditions et actions. Le texte exact se définit dans l’éditeur.'],
      ja: ['チャットのルールを確認', 'チャットトリガーの行には条件とアクションが表示されます。コマンドの正確なテキストはエディタで定義します。'],
      zh: ['找到聊天规则', '聊天触发器的行显示其条件和操作。命令的确切文本在编辑器中定义。'],
    },
    trigger: {
      es: ['Elige el tipo de chat', 'tiktok.chat cubre mensajes escritos y tiktok.emote_chat reacciona a emotes; en otras plataformas usa live.chat. Un evento, un disparador.'],
      en: ['Pick the chat type', 'tiktok.chat covers typed messages and tiktok.emote_chat reacts to emotes; on other platforms use live.chat. One event, one trigger.'],
      pt: ['Escolha o tipo de chat', 'tiktok.chat cobre mensagens escritas e tiktok.emote_chat reage a emotes; em outras plataformas use live.chat.'],
      fr: ['Choisissez le type de chat', 'tiktok.chat couvre les messages et tiktok.emote_chat réagit aux emotes ; sur d’autres plateformes utilisez live.chat.'],
      ja: ['チャットの種類を選択', 'tiktok.chat は書き込みメッセージ、tiktok.emote_chat はエモートに反応します。他プラットフォームでは live.chat。'],
      zh: ['选择聊天类型', 'tiktok.chat 覆盖文字消息,tiktok.emote_chat 响应表情;其他平台用 live.chat。'],
    },
    conditions: {
      es: ['Filtra por contenido del mensaje', 'Aquí defines el comando: compara el mensaje con un texto exacto o su prefijo, y exige roles como moderador o suscriptor si el comando es poderoso.'],
      en: ['Filter by message content', 'Define the command here: match the message against exact text or a prefix, and require roles like moderator or subscriber for powerful commands.'],
      pt: ['Filtre pelo conteúdo', 'Defina o comando aqui: compare a mensagem com texto exato ou prefixo e exija papéis como moderador ou assinante.'],
      fr: ['Filtrez par contenu', 'Définissez la commande ici : comparez le message à un texte exact ou un préfixe, exigez des rôles si nécessaire.'],
      ja: ['メッセージ内容で絞り込み', 'ここでコマンドを定義します: 完全一致やプレフィックスで比較し、強力なコマンドにはロールを要求できます。'],
      zh: ['按消息内容过滤', '在此定义命令: 用精确文本或前缀匹配消息,强大的命令可要求 moderator 等角色。'],
    },
  },
  card: {
      es: ['Eventos de chat y comandos', 'Mensajes, emotes y comandos con prefijo como disparadores.'],
      en: ['Chat events and commands', 'Messages, emotes and prefixed commands as triggers.'],
      pt: ['Eventos de chat e comandos', 'Mensagens, emotes e comandos com prefixo como gatilhos.'],
      fr: ['Événements de chat et commandes', 'Messages, emotes et commandes préfixées comme déclencheurs.'],
      ja: ['チャットイベントとコマンド', 'メッセージ、エモート、プレフィックスコマンドをトリガーに。'],
      zh: ['聊天事件和命令', '消息、表情和前缀命令作为触发器。'],
    },
});
