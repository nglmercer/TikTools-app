import type { LocationQuery, RouteLocationRaw } from 'vue-router';

/**
 * Settings sections that moved into the area they configure. Older links (`/settings?section=globals`)
 * keep working by landing on the new home instead of on a Settings section that no longer exists.
 */
const movedSections: Readonly<Record<string, RouteLocationRaw>> = {
  globals: { path: '/automations', query: { view: 'variables' } },
  plugins: { path: '/plugins', query: { view: 'preferences' } },
  advanced: { path: '/plugins', query: { view: 'preferences' } },
};

export function legacySettingsRedirect(query: LocationQuery): RouteLocationRaw | true {
  const section = typeof query['section'] === 'string' ? query['section'] : '';
  return Object.hasOwn(movedSections, section) ? (movedSections[section] as RouteLocationRaw) : true;
}
