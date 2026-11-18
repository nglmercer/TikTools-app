/**
 * Strings produced OUTSIDE the app's i18n catalogs: native host errors,
 * plugin-supplied descriptors and fixture messages.
 *
 * These are deliberately NOT resolved with `testMessage`: that helper is for
 * translated UI copy owned by this repository. Keeping host/plugin contracts
 * in one named place makes tests readable without pretending they are i18n.
 */
export const HOST_MESSAGES = {
  tokenNotFound: 'Token entry was not found.',
  apiTokenMissing: 'API token missing.',
  fileTooLarge: 'That file is larger than 1 MB.',
  apiReachableGameplayUnavailable: 'API reachable; gameplay unavailable.',
  noConfiguration: 'No configuration found. Select the game folder.',
  panelMoved: 'This panel moved to the plugin view.',
  turnOnCloudSync: 'Turn on Cloud sync in Settings, Account & Cloud.',
  sonicboomDescription: 'SonicBoom renders chat to speech on this machine.',
  definitionSaved: 'The definition was saved.',
} as const;
