import type { DemoLocale } from '../scenarios.ts';
import type { ShowcaseStepId } from './script.ts';

/** Tooltip text: one short line per step, so it never competes with the UI it points at. */
export const showcaseCopy: Record<DemoLocale, Record<ShowcaseStepId, string>> = {
  en: { feed: 'Events arrive in real time', inspector: 'Open any viewer’s profile', automation: 'Turn events into actions', overlays: 'Preview it and add it to OBS', points: 'Points update with every event' },
  es: { feed: 'Los eventos llegan en tiempo real', inspector: 'Abre el perfil de cualquier espectador', automation: 'Convierte eventos en acciones', overlays: 'Previsualízalo y añádelo a OBS', points: 'Los puntos cambian con cada evento' },
  pt: { feed: 'Os eventos chegam em tempo real', inspector: 'Abra o perfil de qualquer espectador', automation: 'Transforme eventos em ações', overlays: 'Visualize e adicione ao OBS', points: 'Os pontos mudam a cada evento' },
  fr: { feed: 'Les événements arrivent en direct', inspector: 'Ouvrez le profil d’un spectateur', automation: 'Transformez les événements en actions', overlays: 'Prévisualisez et ajoutez à OBS', points: 'Les points évoluent à chaque événement' },
  zh: { feed: '事件实时到达', inspector: '打开任意观众的资料', automation: '把事件变成动作', overlays: '预览并添加到 OBS', points: '积分随每个事件更新' },
  ja: { feed: 'イベントがリアルタイムで届く', inspector: '視聴者のプロフィールを開く', automation: 'イベントをアクションに変える', overlays: 'プレビューして OBS に追加', points: 'イベントごとにポイントが更新される' },
};
