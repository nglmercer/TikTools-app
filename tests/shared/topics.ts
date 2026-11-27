/** Event topics pushed by the host and delivered through `control.onTopic`. */
export const TOPIC = {
  liveUiEvent: 'live.ui-event',
  liveConnected: 'live.connected',
  liveEvent: 'live.event',
  pointsChanged: 'points.changed',
  pluginEvent: 'plugin.event',
  workflowChanged: 'workflow.changed',
  liveLikesRanking: 'live.likes-ranking',
  rankingUpdated: 'ranking.updated',
} as const;
export type Topic = (typeof TOPIC)[keyof typeof TOPIC];
