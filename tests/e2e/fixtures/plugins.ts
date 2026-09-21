/**
 * Deterministic plugin fixtures for E2E specs: a SonicBoom-style server
 * plugin with connection + TTS pages, plus list/form page variants.
 */

export const SONICBOOM_ID = 'sonicboom.server';
export const SPEAK_ACTION = 'sonicboom.server.speak';
export const OUTPUT_ACTION = 'sonicboom.server.set-output-device';
export const VOICES_SOURCE = 'plugin-action-options:sonicboom.server.speak:voice';
export const OUTPUTS_SOURCE = 'plugin-action-options:sonicboom.server.set-output-device:device';

const text = (def: string) => ({ default: def, i18key: '' });

export function sonicboomDescriptor(overrides: Record<string, unknown> = {}) {
  return {
    id: SONICBOOM_ID,
    source: 'user',
    name: text('SonicBoom Server'),
    version: '1.0.0',
    description: text('Local speech synthesis server.'),
    dependency: text('SonicBoom server binary.'),
    permissions: [],
    actionTypeIds: [SPEAK_ACTION, OUTPUT_ACTION],
    eventTypeIds: [],
    hasSettings: true,
    hasConnectionProbe: true,
    supportsTokenProvisioning: true,
    ...overrides,
  };
}

export function sonicboomStatus(overrides: Record<string, unknown> = {}) {
  return {
    descriptor: sonicboomDescriptor(),
    installed: true,
    enabled: true,
    running: true,
    available: true,
    ...overrides,
  };
}

export function sonicboomConnectionPage() {
  return {
    id: 'connection',
    pluginId: SONICBOOM_ID,
    title: text('Connection'),
    icon: 'radio',
    sections: [{ kind: 'connection' }],
    source: { kind: 'plugin', pluginId: SONICBOOM_ID },
  };
}

export function sonicboomTtsPage() {
  return {
    id: 'tts',
    pluginId: SONICBOOM_ID,
    title: text('Text to Speech'),
    icon: 'voice',
    sections: [
      {
        kind: 'tts',
        title: text('Text to Speech'),
        actionType: SPEAK_ACTION,
        voicesFrom: VOICES_SOURCE,
        outputsFrom: OUTPUTS_SOURCE,
      },
    ],
    source: { kind: 'plugin', pluginId: SONICBOOM_ID },
  };
}

export function listPage() {
  return {
    id: 'voices',
    pluginId: SONICBOOM_ID,
    title: text('Voices'),
    sections: [{ kind: 'list', title: text('Available voices'), optionsFrom: VOICES_SOURCE }],
    source: { kind: 'plugin', pluginId: SONICBOOM_ID },
  };
}

export function formPage() {
  return {
    id: 'settings',
    pluginId: SONICBOOM_ID,
    title: text('Server settings'),
    sections: [{ kind: 'form', title: text('Server settings') }],
    source: { kind: 'plugin', pluginId: SONICBOOM_ID },
  };
}

export function textPage() {
  return {
    id: 'about',
    pluginId: SONICBOOM_ID,
    title: text('About'),
    sections: [
      {
        kind: 'text',
        title: text('About this plugin'),
        text: text('SonicBoom renders chat to speech on this machine.'),
      },
    ],
    source: { kind: 'plugin', pluginId: SONICBOOM_ID },
  };
}

export function sonicboomSettings() {
  return {
    schema: {
      type: 'object',
      properties: {
        serverUrl: { type: 'string', format: 'uri', default: 'http://127.0.0.1:17842' },
        apiToken: { type: 'string', secret: true },
      },
    },
    uiHints: {},
    values: { serverUrl: 'http://127.0.0.1:17842', apiToken: '' },
  };
}

export function voiceOptions() {
  return {
    options: [
      { value: 'M1', label: 'M1' },
      { value: 'F2', label: 'F2' },
      { value: 'N3', label: 'N3' },
    ],
    selected: null,
  };
}

export function outputOptions() {
  return {
    options: [
      { value: 'Speakers', label: 'Speakers' },
      { value: 'Headphones', label: 'Headphones' },
    ],
    selected: 'Speakers',
  };
}
