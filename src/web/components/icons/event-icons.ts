import type { AutomationEventType } from '../../../automation/contracts/events.ts';
import { BUILTIN_EVENT_TYPES } from '../../../automation/contracts/events.ts';
import type { I18nText } from '../../../automation/behavior/types.ts';
import type { IconName } from './icons.tsx';

export type EventPresentation = {
  icon: IconName;
  label: I18nText;
};

/**
 * Single source of truth for workflow event presentation. The wizard, node
 * picker, templates, canvas, and event pickers all consume this instead of
 * maintaining their own icon/label mappings.
 */
export const EVENT_PRESENTATION: Record<AutomationEventType, EventPresentation> = {
  'tiktok.chat': { icon: 'chat', label: { default: 'Chat message', i18key: 'workflow.event.tiktok.chat' } },
  'tiktok.gift': { icon: 'gift', label: { default: 'Gift received', i18key: 'workflow.event.tiktok.gift' } },
  'tiktok.like': { icon: 'heart', label: { default: 'Likes', i18key: 'workflow.event.tiktok.like' } },
  'tiktok.follow': { icon: 'follow', label: { default: 'New follower', i18key: 'workflow.event.tiktok.follow' } },
  'tiktok.share': { icon: 'share', label: { default: 'Live shared', i18key: 'workflow.event.tiktok.share' } },
  'tiktok.join': { icon: 'join', label: { default: 'Viewer joined', i18key: 'workflow.event.tiktok.join' } },
  'tiktok.social': { icon: 'users', label: { default: 'Social action', i18key: 'workflow.event.tiktok.social' } },
  'tiktok.room_stats': { icon: 'stats', label: { default: 'Room statistics', i18key: 'workflow.event.tiktok.room_stats' } },
  'tiktok.connected': { icon: 'connected', label: { default: 'LIVE connected', i18key: 'workflow.event.tiktok.connected' } },
  'tiktok.disconnected': { icon: 'disconnected', label: { default: 'LIVE disconnected', i18key: 'workflow.event.tiktok.disconnected' } },
  'points.awarded': { icon: 'trophy', label: { default: 'Points awarded', i18key: 'workflow.event.points.awarded' } },
  'plugin.emit': { icon: 'plugin', label: { default: 'Plugin event', i18key: 'workflow.event.plugin.emit' } },
};

export type EventChoice = {
  value: AutomationEventType;
  label: I18nText;
  icon: IconName;
};

/** Every built-in event with its presentation, in catalog order. */
export function workflowEventChoices(): EventChoice[] {
  return BUILTIN_EVENT_TYPES.map((value) => ({
    value,
    ...EVENT_PRESENTATION[value],
  }));
}

/**
 * Presentation for any event type string, including future plugin-declared
 * types that have no built-in entry yet.
 */
export function presentationForEvent(eventType: string): EventPresentation {
  return (EVENT_PRESENTATION as Record<string, EventPresentation>)[eventType]
    ?? { icon: 'plugin', label: { default: eventType, i18key: '' } };
}
