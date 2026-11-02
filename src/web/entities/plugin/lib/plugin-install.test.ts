import { describe, expect, it } from 'vitest';
import { pluginPickerOptions } from './plugin-install.ts';

describe('plugin install', () => {
  it('requests a native .plugin file', () => {
    expect(pluginPickerOptions('Pick')).toEqual({ mode: 'file', kind: 'other', title: 'Pick', extensions: ['plugin'] });
  });
});
