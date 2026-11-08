import { describe, expect, it } from 'vitest';
import { viewerGrant, type EffectGrants } from '../api/types.ts';
import { parseGrant } from './grants.ts';

const effects = (): EffectGrants => ({
  http: [{ origin: 'https://hooks.example', methods: ['POST'], paths: ['/live'], queryFields: ['source'], headerNames: ['content-type'], renderFields: ['event.data.comment'], bodyFields: ['message'], allowTextBody: false, maxBodyBytes: 256, maxResponseBytes: 1024, allowPrivateNetwork: false }],
  audio: [{ fileRefs: ['/media/alert.wav'], output: 'host-default', maxVolume: 0.5, overlaps: ['drop'] }], emitTopics: ['moderator.alert'],
});
const parse = (value: unknown) => parseGrant({ ...viewerGrant(), effects: value });

describe('bounded external effect grants', () => {
  it('preserves old five-field grants and round-trips exact effect restrictions', () => {
    expect(Object.keys(parseGrant(viewerGrant()))).toHaveLength(5);
    expect(parse(effects()).effects).toEqual(effects());
    expect(parse({ http: [], audio: [], emitTopics: [] }).effects).toEqual({ http: [], audio: [], emitTopics: [] });
    expect(() => parse({ ...effects(), unknown: true })).toThrow('invalid_grant');
  });
  it('rejects unknown fields, unsafe origins, ambiguous paths and non-explicit HTTP access', () => {
    for (const change of [
      { unknown: true }, { origin: 'https://hooks.example/' }, { origin: 'https://HOOKS.example' }, { origin: 'https://user@hooks.example' },
      { origin: 'http://hooks.example', allowPrivateNetwork: true }, { origin: 'http://127.0.0.1:8080' },
      { paths: ['/%2f'] }, { paths: ['/a/../live'] }, { paths: ['//other.example/live'] }, { paths: ['/*'] },
      { paths: ['/live?x=1'] }, { methods: ['CONNECT'] }, { methods: ['POST', 'POST'] }, { methods: [] },
      { headerNames: ['Content-Type'] }, { headerNames: ['authorization\r\nx-other'] }, { renderFields: ['globals.token'] },
      { renderFields: ['event.user'] }, { queryFields: ['*'] }, { maxBodyBytes: 262145 }, { maxResponseBytes: 0 },
      { maxResponseBytes: 2097153 }, { allowTextBody: 'true' },
    ]) {
      const value = effects(); Object.assign(value.http[0]!, change);
      expect(() => parse(value), JSON.stringify(change)).toThrow('invalid_grant');
    }
    for (const name of ['host', 'content-length', 'transfer-encoding', 'connection', 'upgrade', 'proxy-authorization', 'proxy-connection', 'te', 'trailer']) {
      const value = effects(); value.http[0]!.headerNames = [name]; expect(() => parse(value)).toThrow('invalid_grant');
    }
    const value = effects(); Object.assign(value.http[0]!, { origin: 'http://127.0.0.1:8080', allowPrivateNetwork: true, headerNames: ['authorization', 'cookie'] });
    expect(parse(value).effects).toEqual(value);
  });
  it('keeps file references, output, volume, topics and total budget bounded', () => {
    for (const change of [{ fileRefs: ['relative.wav'] }, { fileRefs: ['/media/../alert.wav'] }, { fileRefs: ['/media/*.wav'] }, { output: 'remote' }, { maxVolume: 1.1 }, { maxVolume: NaN }, { overlaps: [] }, { overlaps: ['any'] }]) {
      const value = effects(); Object.assign(value.audio[0]!, change); expect(() => parse(value)).toThrow('invalid_grant');
    }
    for (const topic of ['*', 'moderator.*', 'MODERATOR.alert', 'moderator.alert ', 'moderator.\nalert']) {
      const value = effects(); value.emitTopics = [topic]; expect(() => parse(value)).toThrow('invalid_grant');
    }
    const value = effects(); value.http = Array.from({ length: 33 }, () => value.http[0]!); expect(() => parse(value)).toThrow('invalid_grant');
    const large = effects(); large.audio[0]!.fileRefs = Array.from({ length: 100 }, (_, index) => `/media/${'a'.repeat(3000)}-${index}.wav`);
    expect(() => parse(large)).toThrow('invalid_grant');
  });
});
