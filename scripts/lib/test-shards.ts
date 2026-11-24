/** Partition test names into stable, balanced shards.
 *
 * Each shard runs in its own process, so a process-global test home is
 * isolated per shard. Round-robin keeps shards balanced even when one module
 * contributes many tests.
 */
export function partitionTestNames(names: readonly string[], shards: number): string[][] {
  const count = Math.max(1, Math.min(shards, names.length));
  const result: string[][] = Array.from({ length: count }, () => []);
  names.forEach((name, index) => { result[index % count]!.push(name); });
  return result.filter((shard) => shard.length > 0);
}
