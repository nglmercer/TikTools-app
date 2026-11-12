import type { ProfilePack } from './rule-profiles.ts';

export interface ExportedProfileDoc {
  doc: Record<string, unknown>;
  entries: number;
  skipped: number;
}

export function buildProfileDoc(
  pack: ProfilePack,
  events: Array<{ id: string; name: string; enabled: boolean; trigger: string; filters: unknown; cooldownMs: number; cooldownScope: string; runMode: string; actionIds: string[] }>,
  actions: Array<{ id: string; name: string; typeId: string; enabled: boolean; config: unknown }>,
): ExportedProfileDoc {
  const eventSet = new Set(pack.eventIds);
  const actionSet = new Set(pack.actionIds);
  const byId = new Map(actions.map((action) => [action.id, action]));
  const templates: unknown[] = [];
  let skipped = 0;
  for (const event of events) {
    if (!eventSet.has(event.id)) continue;
    const exported = event.actionIds
      .filter((id) => actionSet.has(id))
      .map((id) => byId.get(id))
      .filter((action) => action !== undefined)
      .map((action) => ({
        name: action.name,
        typeId: action.typeId,
        enabled: action.enabled,
        config: action.config ?? {},
      }));
    if (exported.length === 0) {
      skipped += 1;
      continue;
    }
    templates.push({
      template: {
        templateVersion: 1,
        id: `export-${event.id}`.slice(0, 64),
        title: event.name,
        description: '',
        icon: 'plugin',
        actions: exported,
        event: {
          name: event.name,
          enabled: event.enabled,
          trigger: event.trigger,
          filters: event.filters ?? [],
          cooldownMs: event.cooldownMs,
          cooldownScope: event.cooldownScope,
          runMode: event.runMode,
        },
      },
      params: {},
    });
  }
  return {
    doc: {
      profileVersion: 1,
      id: pack.id,
      name: pack.name === '' ? pack.id : pack.name,
      description: pack.description,
      params: {},
      templates,
    },
    entries: templates.length,
    skipped,
  };
}

export function downloadTextFile(filename: string, text: string): void {
  if (typeof document === 'undefined' || typeof URL === 'undefined') return;
  const url = URL.createObjectURL(new Blob([text], { type: 'application/json' }));
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
}
