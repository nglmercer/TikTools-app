import { defineTour } from '../model/define-tour.ts';
import { guideScope } from '../model/guide-vocabulary.ts';

export default defineTour({
  id: 'overlayObs',
  version: 1,
  route: '/overlays',
  scope: guideScope.overlays,
  tags: ['overlays'],
  icon: 'video',
  shape: 'custom',
  layout: [
    { id: 'gateway', target: 'overlay-gateway', policy: 'cancel' },
    { id: 'widget', target: 'overlay-widgets', policy: 'skip' },
    { id: 'preview', target: 'overlay-preview', policy: 'skip' },
    { id: 'url', target: 'overlay-url', policy: 'cancel' },
    { id: 'design', target: 'overlay-design', policy: 'skip' },
  ],
  steps: {
    gateway: {
      es: ['Empieza por el gateway', 'Los widgets de OBS reciben eventos mediante el gateway local. Confirma que está activo antes de copiar cualquier URL.'],
      en: ['Start with the gateway', 'OBS widgets receive events through the local gateway. Confirm it is running before copying any source URL.'],
      pt: ['Comece pelo gateway', 'Os widgets do OBS recebem eventos pelo gateway local. Confirme que ele está ativo antes de copiar qualquer URL de fonte.'],
      fr: ['Commencez par la passerelle', 'Les widgets OBS reçoivent les événements via la passerelle locale. Confirmez qu’elle fonctionne avant de copier une URL de source.'],
      ja: ['ゲートウェイから開始', 'OBSウィジェットはローカルゲートウェイ経由でイベントを受け取ります。ソースURLをコピーする前に動作を確認します。'],
      zh: ['从网关开始', 'OBS 小部件通过本地网关接收事件。复制任何源 URL 前，先确认网关正在运行。'],
    },
    widget: {
      es: ['Elige el widget', 'Elige la fuente de follows, regalos, chat, likes o ranking que quieres colocar en OBS.'],
      en: ['Choose the widget', 'Pick the follow, gift, chat, likes or ranking source you want to place in OBS.'],
      pt: ['Escolha o widget', 'Escolha a fonte de follows, presentes, chat, likes ou ranking que você quer colocar no OBS.'],
      fr: ['Choisissez le widget', 'Choisissez la source follows, cadeaux, chat, likes ou ranking à placer dans OBS.'],
      ja: ['ウィジェットを選択', 'OBSに配置したいfollow、gift、chat、likes、rankingのソースを選びます。'],
      zh: ['选择小部件', '选择要放入 OBS 的 follow、gift、chat、likes 或 ranking 源。'],
    },
    preview: {
      es: ['Prueba con la vista previa', 'Reproduce eventos de ejemplo y ajusta el diseño localmente. La vista previa no demuestra que OBS esté conectado.'],
      en: ['Test with the preview', 'Replay sample events and adjust the design locally. The preview does not prove that OBS is connected.'],
      pt: ['Teste com a pré-visualização', 'Reproduza eventos de exemplo e ajuste o design localmente. A pré-visualização não prova que o OBS está conectado.'],
      fr: ['Testez avec l’aperçu', 'Rejouez des événements d’exemple et ajustez le design localement. L’aperçu ne prouve pas que OBS est connecté.'],
      ja: ['プレビューでテスト', 'サンプルイベントを再生し、デザインをローカルで調整します。プレビューだけではOBS接続は確認できません。'],
      zh: ['用预览测试', '重放示例事件并在本地调整设计。预览不能证明 OBS 已连接。'],
    },
    url: {
      es: ['Copia la URL para OBS', 'Copia la URL solo cuando el gateway esté listo y pégala en una Browser Source de OBS con el tamaño deseado.'],
      en: ['Copy the OBS URL', 'Copy the source URL only after the gateway is ready, then paste it into an OBS Browser Source with the intended size.'],
      pt: ['Copie a URL do OBS', 'Copie a URL da fonte somente depois que o gateway estiver pronto e cole-a em uma Browser Source do OBS com o tamanho desejado.'],
      fr: ['Copiez l’URL OBS', 'Copiez l’URL de source seulement lorsque la passerelle est prête, puis collez-la dans une Browser Source OBS avec la taille voulue.'],
      ja: ['OBS URLをコピー', 'ゲートウェイの準備ができてからソースURLをコピーし、目的のサイズでOBSのBrowser Sourceに貼り付けます。'],
      zh: ['复制 OBS URL', '只有网关就绪后才复制源 URL，然后按所需大小粘贴到 OBS Browser Source。'],
    },
    design: {
      es: ['Ajusta animación y apariencia', 'El editor incluye estilo de animación, duración, curva y retardo. Guarda antes de copiar una URL actualizada.'],
      en: ['Tune motion and appearance', 'The design editor now includes animation style, duration, easing and stagger. Save before copying a refreshed URL.'],
      pt: ['Ajuste movimento e aparência', 'O editor de design agora inclui estilo de animação, duração, easing e stagger. Salve antes de copiar uma URL atualizada.'],
      fr: ['Réglez mouvement et apparence', 'L’éditeur de design inclut maintenant style d’animation, durée, easing et stagger. Enregistrez avant de copier une URL actualisée.'],
      ja: ['動きと外観を調整', 'デザインエディターにはアニメーションスタイル、duration、easing、staggerがあります。更新したURLをコピーする前に保存します。'],
      zh: ['调整动画和外观', '设计编辑器现在包含动画样式、时长、缓动和交错。复制更新后的 URL 前先保存。'],
    },
  },
  card: {
      es: ['Overlay hacia OBS', 'Gateway, vista previa, URL de origen y flujo de Browser Source.'],
      en: ['Overlay to OBS', 'Gateway, preview, source URL and Browser Source flow.'],
      pt: ['Overlay para o OBS', 'Gateway, pré-visualização, URL da fonte e fluxo de Browser Source.'],
      fr: ['Overlay vers OBS', 'Passerelle, aperçu, URL source et flux Browser Source.'],
      ja: ['オーバーレイをOBSへ', 'ゲートウェイ、プレビュー、ソースURL、Browser Sourceの手順。'],
      zh: ['将悬浮层添加到 OBS', '网关、预览、源 URL 和 Browser Source 流程。'],
    },
});
