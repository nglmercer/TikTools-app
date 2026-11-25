import { defineTour } from '../model/define-tour.ts';

export default defineTour({
  id: 'connectionRecovery',
  version: 1,
  route: '/connections',
  scope: 'connections',
  tags: ['connection'],
  icon: 'refresh',
  shape: 'custom',
  layout: [
    { id: 'form', target: 'connection-form', policy: 'cancel' },
    { id: 'explorer', target: 'connection-explorer', policy: 'skip' },
    { id: 'plugins', target: 'connection-plugins', policy: 'skip' },
  ],
  steps: {
    form: {
      es: ['Revisa creador y estado', 'Empieza con el handle exacto y el estado actual. Offline, autenticación y sala no disponible requieren recuperaciones distintas.'],
      en: ['Recheck the creator and state', 'Start with the exact creator handle and the current connection state. Offline, authentication and unavailable-room states need different recovery.'],
      pt: ['Verifique novamente criador e estado', 'Comece com o handle exato do criador e o estado atual da conexão. Offline, autenticação e sala indisponível exigem recuperações diferentes.'],
      fr: ['Revérifiez créateur et état', 'Commencez par le handle exact du créateur et l’état actuel de connexion. Offline, authentification et salle indisponible demandent des récupérations différentes.'],
      ja: ['クリエイターと状態を再確認', '正確なクリエイターハンドルと現在の接続状態から始めます。オフライン、認証、ルーム不可では復旧方法が異なります。'],
      zh: ['重新检查创作者和状态', '从准确的创作者句柄和当前连接状态开始。离线、认证和房间不可用需要不同的恢复方式。'],
    },
    explorer: {
      es: ['Verifica la sala LIVE', 'Actualiza la lista y confirma que la sala sea accesible. Puede estar offline, restringida o no disponible para invitados.'],
      en: ['Verify the LIVE room', 'Refresh the room list and confirm the room is accessible. A missing room can be offline, restricted or simply not available to guests.'],
      pt: ['Verifique a sala LIVE', 'Atualize a lista de salas e confirme que a sala está acessível. Uma sala ausente pode estar offline, restrita ou simplesmente indisponível para convidados.'],
      fr: ['Vérifiez la salle LIVE', 'Actualisez la liste des salles et confirmez que la salle est accessible. Une salle absente peut être hors ligne, restreinte ou simplement indisponible aux invités.'],
      ja: ['LIVEルームを確認', 'ルーム一覧を更新し、アクセス可能か確認します。見つからないルームはオフライン、制限付き、またはゲストに非公開の可能性があります。'],
      zh: ['验证 LIVE 房间', '刷新房间列表并确认房间可访问。找不到的房间可能离线、受限，或只是不向访客开放。'],
    },
    plugins: {
      es: ['Revisa integraciones tras reconectar', 'Las integraciones pueden ir detrás de una reconexión. Revisa su salud antes de asumir que fallaron automatizaciones u overlays.'],
      en: ['Check integrations after reconnecting', 'Connection integrations can lag behind a reconnect. Review their health before assuming automations or overlays failed.'],
      pt: ['Verifique integrações após reconectar', 'As integrações de conexão podem demorar após uma reconexão. Revise a saúde delas antes de assumir que automações ou overlays falharam.'],
      fr: ['Vérifiez les intégrations après reconnexion', 'Les intégrations de connexion peuvent avoir du retard après une reconnexion. Vérifiez leur santé avant de supposer que les automatisations ou overlays ont échoué.'],
      ja: ['再接続後に連携を確認', '接続連携は再接続に遅れることがあります。自動化やオーバーレイの失敗と判断する前に状態を確認します。'],
      zh: ['重连后检查集成', '连接集成可能滞后于重连。在判断自动化或悬浮层失败前，先检查其健康状态。'],
    },
  },
  card: {
      es: ['Recuperación de conexión', 'Diagnostica salas offline, autenticación e integraciones.'],
      en: ['Connection recovery', 'Diagnose offline rooms, authentication and integration recovery.'],
      pt: ['Recuperação de conexão', 'Diagnostique salas offline, autenticação e recuperação de integrações.'],
      fr: ['Récupération de connexion', 'Diagnostiquez les salles hors ligne, l’authentification et la reprise des intégrations.'],
      ja: ['接続の復旧', 'オフラインの部屋、認証、統合の復旧を診断します。'],
      zh: ['连接恢复', '诊断离线房间、身份验证和集成恢复。'],
    },
});
