import { describe, expect, test } from 'bun:test';
import { selectUpdateChannels } from './update-channel.ts';

describe('update feed promotion', () => {
  test('stable releases advance both channels only when newer', () => {
    expect(selectUpdateChannels('4.1.0', '4.0.0', '4.1.0-beta.1')).toEqual(['stable', 'beta']);
    expect(selectUpdateChannels('4.1.0', '4.2.0', '4.2.0-beta.1')).toEqual([]);
    expect(selectUpdateChannels('4.1.0', '4.0.0', '4.2.0-beta.1')).toEqual(['stable']);
  });
  test('prereleases only advance beta and equal precedence does not replace a feed', () => {
    expect(selectUpdateChannels('4.1.0-beta.2', '4.0.0', '4.1.0-beta.1')).toEqual(['beta']);
    expect(selectUpdateChannels('4.1.0-beta.1', null, '4.1.0')).toEqual([]);
    expect(selectUpdateChannels('4.1.0+build-2', '4.1.0+build-1', null)).toEqual(['beta']);
    expect(() => selectUpdateChannels('v4.1.0', null, null)).toThrow();
    expect(() => selectUpdateChannels('4.1.0', 'bad', null)).toThrow();
  });
});
