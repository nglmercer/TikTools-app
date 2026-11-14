import { expect, test } from 'bun:test';
import { validateCliBuildInfo } from './cli-release.ts';

test('release CLI must match source identity and serve native peers', () => {
  const build = { version: '4.2.2', sourceRevision: 'a'.repeat(40), buildMode: 'release', features: { cloudAuth: true, remoteClient: true, remoteControl: true } };
  expect(() => validateCliBuildInfo(build, build.version, build.sourceRevision)).not.toThrow();
  expect(() => validateCliBuildInfo({ ...build, sourceRevision: 'b'.repeat(40) }, build.version, build.sourceRevision)).toThrow('source revision');
  expect(() => validateCliBuildInfo({ ...build, features: { ...build.features, remoteControl: false } }, build.version, build.sourceRevision)).toThrow('headless');
  expect(() => validateCliBuildInfo({ ...build, buildMode: 'development' }, build.version, build.sourceRevision)).toThrow('build mode');
});
