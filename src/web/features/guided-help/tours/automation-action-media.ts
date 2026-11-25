import { defineTour } from '../model/define-tour.ts';

export default defineTour({
  id: 'automationActionMedia',
  version: 1,
  route: '/automations',
  scope: 'automations',
  tags: ['actions'],
  icon: 'video',
  shape: 'action',
  steps: {
    actionsTab: {
      es: ['Acciones de overlay y sonido', 'Las acciones de media muestran alertas en el overlay o reproducen sonidos: son el efecto visible de tus eventos.'],
      en: ['Overlay and sound actions', 'Media actions show alerts on the overlay or play sounds: they are the visible effect of your events.'],
      pt: ['Ações de overlay e som', 'Ações de mídia mostram alertas no overlay ou tocam sons: são o efeito visível dos seus eventos.'],
      fr: ['Actions overlay et son', 'Les actions média affichent des alertes sur l’overlay ou jouent des sons : l’effet visible de vos événements.'],
      ja: ['オーバーレイと音声アクション', 'メディアアクションはオーバーレイに警告を表示したり音声を再生します: イベントの見える効果です。'],
      zh: ['overlay 和声音操作', '媒体操作在 overlay 显示提醒或播放声音: 事件的可见效果。'],
    },
    actions: {
      es: ['Encuentra tus acciones de media', 'Cada fila indica el tipo de efecto. Como se comparten entre eventos, un cambio aquí afecta a todo lo que las usa.'],
      en: ['Find your media actions', 'Each row shows the effect type. Because they are shared across events, one change here affects everything using them.'],
      pt: ['Encontre suas ações de mídia', 'Cada linha indica o tipo de efeito. Como são compartilhadas, uma mudança afeta tudo que as usa.'],
      fr: ['Trouvez vos actions média', 'Chaque ligne indique le type d’effet. Partagées entre événements, un changement ici affecte tout.'],
      ja: ['メディアアクションを確認', '各行はエフェクトの種類を示します。共有されるため、ここの変更は利用中の全イベントに影響します。'],
      zh: ['找到媒体操作', '每行显示效果类型。因被共享,此处的修改影响所有使用它们的事件。'],
    },
    settings: {
      es: ['Configura el efecto', 'Elige el archivo de sonido o el contenido del overlay y su duración. Los campos dependen del tipo de acción; prueba siempre antes de activar.'],
      en: ['Configure the effect', 'Pick the sound file or overlay content and its duration. Fields depend on the action type; always test before enabling.'],
      pt: ['Configure o efeito', 'Escolha o arquivo de som ou conteúdo do overlay e sua duração. Teste sempre antes de ativar.'],
      fr: ['Configurez l’effet', 'Choisissez le fichier son ou le contenu overlay et sa durée. Testez toujours avant d’activer.'],
      ja: ['エフェクトを設定', '音声ファイルやオーバーレイ内容と長さを選びます。有効化前に必ずテストしてください。'],
      zh: ['配置效果', '选择声音文件或 overlay 内容及时长。启用前务必测试。'],
    },
    permissions: {
      es: ['Revisa los permisos', 'Las acciones de media pueden necesitar acceso a archivos o al overlay. Los permisos se listan aquí y nunca se conceden desde la guía.'],
      en: ['Review permissions', 'Media actions may need file or overlay access. Permissions are listed here and are never granted from the guide.'],
      pt: ['Revise as permissões', 'Ações de mídia podem precisar de acesso a arquivos ou overlay. As permissões se listam aqui e nunca se concedem desde a guia.'],
      fr: ['Vérifiez les permissions', 'Les actions média peuvent exiger l’accès aux fichiers ou à l’overlay. Elles sont listées ici, jamais accordées par le guide.'],
      ja: ['権限を確認', 'メディアアクションはファイルやオーバーレイへのアクセスが必要な場合があります。ガイドから付与はされません。'],
      zh: ['检查权限', '媒体操作可能需要文件或 overlay 访问权限。权限在此列出,引导不会授予。'],
    },
  },
  card: {
      es: ['Acciones de overlay y sonido', 'Alertas visuales y efectos de audio reutilizables.'],
      en: ['Overlay and sound actions', 'Reusable visual alerts and audio effects.'],
      pt: ['Ações de overlay e som', 'Alertas visuais e efeitos de áudio reutilizáveis.'],
      fr: ['Actions overlay et son', 'Alertes visuelles et effets audio réutilisables.'],
      ja: ['オーバーレイと音声アクション', '再利用可能な視覚警告と音声エフェクト。'],
      zh: ['overlay 和声音操作', '可复用的视觉提醒和音效。'],
    },
});
