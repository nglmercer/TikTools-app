import { defineTour } from '../model/define-tour.ts';
import { guideScope } from '../model/guide-vocabulary.ts';

export default defineTour({
  id: 'cloudOverlays',
  version: 1,
  route: '/overlays',
  scope: guideScope.overlays,
  demo: false,
  availability: { account: 'signed-in' },
  tags: ['overlays', 'account'],
  icon: 'cloud',
  shape: 'custom',
  layout: [
    { id: 'open', target: 'overlay-cloud-open', policy: 'cancel' },
    { id: 'widgets', target: 'overlay-widgets', policy: 'cancel' },
    { id: 'preview', target: 'overlay-preview', policy: 'skip' },
    { id: 'url', target: 'overlay-url', policy: 'skip' },
  ],
  steps: {
    open: {
      es: ['Abre la biblioteca Cloud', 'Cuando los overlays Cloud estén disponibles, explora documentos compartidos desde el botón de biblioteca Cloud.'],
      en: ['Open the Cloud library', 'When Cloud overlays are available, browse shared overlay documents from the Cloud library button.'],
      pt: ['Abra a biblioteca Cloud', 'Quando os overlays Cloud estiverem disponíveis, explore documentos compartilhados pelo botão da biblioteca Cloud.'],
      fr: ['Ouvrez la bibliothèque Cloud', 'Quand les overlays Cloud sont disponibles, parcourez les documents partagés depuis le bouton de bibliothèque Cloud.'],
      ja: ['Cloudライブラリを開く', 'Cloudオーバーレイが利用できる場合、Cloudライブラリボタンから共有オーバーレイドキュメントを参照します。'],
      zh: ['打开 Cloud 库', 'Cloud 悬浮层可用时，通过 Cloud 库按钮浏览共享的悬浮层文档。'],
    },
    widgets: {
      es: ['Selecciona el destino local', 'Elige qué widget local debe recibir el diseño Cloud seleccionado antes de aplicar nada.'],
      en: ['Select a local target', 'Choose which local widget should receive the selected Cloud design before applying anything.'],
      pt: ['Selecione um destino local', 'Escolha qual widget local deve receber o design Cloud selecionado antes de aplicar qualquer coisa.'],
      fr: ['Sélectionnez une cible locale', 'Choisissez quel widget local doit recevoir le design Cloud sélectionné avant d’appliquer quoi que ce soit.'],
      ja: ['ローカルの適用先を選択', '何も適用する前に、選択したCloudデザインを受け取るローカルウィジェットを選びます。'],
      zh: ['选择本地目标', '应用任何内容前，先选择哪个本地小部件应接收所选 Cloud 设计。'],
    },
    preview: {
      es: ['Previsualiza antes de publicar', 'Usa la vista previa local para comparar el diseño. Publicar o sobrescribir una copia Cloud sigue siendo una acción explícita.'],
      en: ['Preview before publishing', 'Use the local preview to compare the selected design. Publishing or overwriting a Cloud copy remains an explicit action.'],
      pt: ['Pré-visualize antes de publicar', 'Use a pré-visualização local para comparar o design selecionado. Publicar ou substituir uma cópia Cloud continua sendo uma ação explícita.'],
      fr: ['Prévisualisez avant de publier', 'Utilisez l’aperçu local pour comparer le design sélectionné. Publier ou écraser une copie Cloud reste une action explicite.'],
      ja: ['公開前にプレビュー', 'ローカルプレビューで選択したデザインを比較します。Cloudコピーの公開や上書きは明示的な操作のままです。'],
      zh: ['发布前预览', '使用本地预览比较所选设计。发布或覆盖 Cloud 副本仍然是明确操作。'],
    },
    url: {
      es: ['Copia la fuente correcta', 'Las URLs del gateway local y las URLs Cloud resuelven casos distintos. Verifica cuál debe usar tu escena de OBS.'],
      en: ['Copy the correct source', 'Local gateway URLs and Cloud overlay URLs solve different recovery cases. Verify which one your OBS scene should use.'],
      pt: ['Copie a fonte correta', 'URLs do gateway local e URLs de overlays Cloud resolvem casos de recuperação diferentes. Verifique qual delas sua cena do OBS deve usar.'],
      fr: ['Copiez la bonne source', 'Les URL de passerelle locale et les URL d’overlay Cloud répondent à des cas de récupération différents. Vérifiez laquelle votre scène OBS doit utiliser.'],
      ja: ['正しいソースをコピー', 'ローカルゲートウェイURLとCloudオーバーレイURLは異なる復旧用途向けです。OBSシーンで使うべき方を確認します。'],
      zh: ['复制正确的源', '本地网关 URL 和 Cloud 悬浮层 URL 用于不同的恢复场景。确认你的 OBS 场景应使用哪一种。'],
    },
  },
  card: {
      es: ['Overlays Cloud', 'Explora, compara y publica diseños de overlay compartidos.'],
      en: ['Cloud overlays', 'Browse, compare and publish shared overlay designs.'],
      pt: ['Overlays Cloud', 'Explore, compare e publique designs de overlay compartilhados.'],
      fr: ['Overlays Cloud', 'Parcourez, comparez et publiez des designs d’overlay partagés.'],
      ja: ['Cloudオーバーレイ', '共有オーバーレイデザインを参照、比較、公開します。'],
      zh: ['Cloud 悬浮层', '浏览、比较并发布共享悬浮层设计。'],
    },
});
