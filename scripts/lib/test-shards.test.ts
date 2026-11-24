import { expect, test } from 'bun:test';
import { partitionTestNames } from './test-shards.ts';

test('partitions names into balanced, lossless shards', () => {
  const names = ['a', 'b', 'c', 'd', 'e'];
  const shards = partitionTestNames(names, 2);
  expect(shards).toHaveLength(2);
  expect(shards.map((shard) => shard.length)).toEqual([3, 2]);
  expect(shards.flat().sort()).toEqual([...names].sort());
});

test('never creates empty shards and caps shards at the test count', () => {
  expect(partitionTestNames(['only'], 4)).toEqual([['only']]);
  expect(partitionTestNames([], 4)).toEqual([]);
});
