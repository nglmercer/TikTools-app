import { defineTour } from '../model/define-tour.ts';

export default defineTour({
  id: 'overlay',
  version: 1,
  route: '/overlays',
  scope: 'overlays',
  task: 'overlay.previewed',
  tags: ['overlays'],
  icon: 'layers',
  shape: 'custom',
  layout: [
    { id: 'gateway', target: 'overlay-gateway', policy: 'cancel' },
    { id: 'preview', target: 'overlay-preview', policy: 'cancel' },
    { id: 'design', target: 'overlay-design', policy: 'skip' },
  ],
  steps: {
    gateway: {
      es: ['Comprueba el gateway', 'La salida en vivo necesita el plugin event-gateway y credencial de widget. Una vista previa local no demuestra conexión OBS.'],
      en: ['Check gateway status', 'Live output requires the event-gateway plugin and a widget credential. A local preview does not prove OBS connectivity.'],
      pt: ['Confira o gateway', 'A saída ao vivo requer event-gateway e credencial de widget. A prévia local não comprova conexão com OBS.'],
      fr: ['Vérifier la passerelle', 'La sortie en direct exige event-gateway et un identifiant de widget. Un aperçu local ne prouve pas la connexion OBS.'],
      ja: ['ゲートウェイの状態を確認', 'ライブ出力にはevent-gatewayプラグインとウィジェットの認証情報が必要です。ローカルのプレビューだけでは、OBSへの接続は確認できません。'],
      zh: ['检查网关状态', '实时输出需要 event-gateway 插件和组件凭据。本地预览不能证明已连接 OBS。'],
    },
    preview: {
      es: ['Previsualiza localmente', 'La vista utiliza el mismo renderizador con eventos de muestra. No necesita LIVE real ni envía alertas a OBS.'],
      en: ['Preview locally', 'The preview uses the same widget renderer with sample events. It needs no real LIVE session and sends no alerts to OBS.'],
      pt: ['Visualize localmente', 'A prévia usa o mesmo renderizador com eventos de exemplo. Não precisa de LIVE real nem envia alertas ao OBS.'],
      fr: ['Prévisualiser localement', 'L’aperçu emploie le même rendu avec des événements exemples. Il ne nécessite aucun LIVE réel et n’envoie aucune alerte OBS.'],
      ja: ['ローカルでプレビュー', 'プレビューは同じウィジェット描画機能とサンプルイベントを使用します。実際のLIVEセッションは不要で、OBSに通知を送りません。'],
      zh: ['本地预览', '预览使用相同的组件渲染器和示例事件，无需真实直播会话，也不会向 OBS 发送提醒。'],
    },
    design: {
      es: ['Revisa el diseño', 'Edita y guarda intencionalmente; después copia una URL actualizada para OBS. Changes de Overlays sigue siendo un seguimiento sin integrar.'],
      en: ['Review the design', 'Edit and save intentionally, then copy an updated source URL for OBS. Overlay Changes remains an unmerged follow-up.'],
      pt: ['Revise o design', 'Edite e salve intencionalmente; depois copie uma URL atualizada para OBS. Changes de Overlays continua como trabalho não integrado.'],
      fr: ['Examiner la présentation', 'Modifiez et enregistrez volontairement, puis copiez une URL actualisée pour OBS. Changes des overlays reste un suivi non intégré.'],
      ja: ['デザインを確認', '内容を確認して編集・保存し、OBS用の更新済みソースURLをコピーしてください。「オーバーレイの変更」は、まだマージされていない後続の対応です。'],
      zh: ['审查设计', '明确编辑并保存后，再复制更新的源 URL 供 OBS 使用。“叠加层更改”仍是尚未合并的后续工作。'],
    },
  },
  card: {
      es: ['Overlays', 'Previsualiza un overlay localmente antes de usarlo en directo.'],
      en: ['Overlays', 'Preview an overlay locally before using it on air.'],
      pt: ['Overlays', 'Pré-visualize um overlay localmente antes de usá-lo no ar.'],
      fr: ['Overlays', 'Prévisualisez un overlay localement avant de l’utiliser en direct.'],
      ja: ['オーバーレイ', '配信前にオーバーレイをローカルでプレビューします。'],
      zh: ['叠加层', '在直播前于本地预览叠加层。'],
    },
});
