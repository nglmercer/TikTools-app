/**
 * Assembled tour modules: the single registration point. Adding a tour is one
 * file in this directory plus one import and one array entry here; every
 * downstream structure (registry, per-locale catalogs, catalog metadata) is
 * derived from the modules themselves.
 */
import analytics from './analytics.ts';
import automation_action_http from './automation-action-http.ts';
import automation_action_media from './automation-action-media.ts';
import automation_action_tts from './automation-action-tts.ts';
import automation_action_vars from './automation-action-vars.ts';
import automation_actions from './automation-actions.ts';
import automation_cooldowns from './automation-cooldowns.ts';
import automation_event_chat from './automation-event-chat.ts';
import automation_event_gift from './automation-event-gift.ts';
import automation_event_room from './automation-event-room.ts';
import automation_event_social from './automation-event-social.ts';
import automation_events from './automation-events.ts';
import automation_filters from './automation-filters.ts';
import automation_templates from './automation-templates.ts';
import automation_variables from './automation-variables.ts';
import backup_restore from './backup-restore.ts';
import backup from './backup.ts';
import changes from './changes.ts';
import cloud_overlays from './cloud-overlays.ts';
import connection_recovery from './connection-recovery.ts';
import connection from './connection.ts';
import diagnostics from './diagnostics.ts';
import feed_inspector from './feed-inspector.ts';
import feed from './feed.ts';
import overlay_obs from './overlay-obs.ts';
import overlay from './overlay.ts';
import plugin_setup from './plugin-setup.ts';
import plugin from './plugin.ts';
import points from './points.ts';
import rankings from './rankings.ts';
import settings from './settings.ts';


export const tourModules = [
  automation_events,
  automation_actions,
  automation_variables,
  changes,
  automation_event_chat,
  automation_event_gift,
  automation_event_social,
  automation_event_room,
  automation_action_media,
  automation_action_tts,
  automation_action_http,
  automation_action_vars,
  automation_filters,
  automation_cooldowns,
  automation_templates,
  connection,
  connection_recovery,
  plugin,
  plugin_setup,
  overlay,
  overlay_obs,
  cloud_overlays,
  points,
  rankings,
  analytics,
  diagnostics,
  feed,
  feed_inspector,
  settings,
  backup,
  backup_restore,
] as const;

export type TourModuleId<M> = M extends { id: infer I } ? I : never;
export type TourId = TourModuleId<(typeof tourModules)[number]>;
