/** Hash routes the app serves (the same locations `tabLocation` builds). */
export const ROUTE = {
  root: '/',
  plugins: '/#/plugins',
  settings: '/#/settings',
  settingsPlugins: '/#/settings?section=plugins',
  settingsGlobals: '/#/settings?section=globals',
  automations: '/#/automations',
  automationVariables: '/#/automations?view=variables',
  overlays: '/#/overlays',
  connections: '/#/connections',
} as const;
