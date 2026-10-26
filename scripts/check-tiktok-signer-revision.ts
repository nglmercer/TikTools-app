import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const expectedRevision = '9b4f344d105c62e23bf3a883c7a35066e001e078';
const cargo = await readFile(resolve(import.meta.dir, '..', 'Cargo.toml'), 'utf8');
const requiredCrates = [
  'ttl-sign-core',
  'ttl-live-discovery',
  'ttl-live-events',
  'ttl-live-proto',
  'ttl-live-ws',
  'ttl-sign-headless',
  'ttl-sign-embedded',
];

const mismatches = requiredCrates.flatMap((crate) => {
  const line = cargo.match(new RegExp(`^${crate.replaceAll('-', '\\-')}\\s*=\\s*\\{[^\\n]+$`, 'm'))?.[0];
  const revision = line?.match(/\brev\s*=\s*"([0-9a-f]+)"/)?.[1];
  return revision === expectedRevision ? [] : [`${crate}: ${revision ?? 'missing'}`];
});

if (/^ttl-live-events\s*=\s*\{[^\n]*path\s*=/m.test(cargo)) {
  mismatches.push('ttl-live-events must use upstream, without a local patch');
}

if (mismatches.length) {
  console.error(`tiktok-signer revision check failed (expected ${expectedRevision}):`);
  for (const mismatch of mismatches) console.error(`  ${mismatch}`);
  process.exit(1);
}

console.log(`All ${requiredCrates.length} tiktok-signer crates use ${expectedRevision}.`);
