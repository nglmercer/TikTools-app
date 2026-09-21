/**
 * Builds the SonicBoom backend and stages it at the manifest entry
 * (`plugins/sonicboom/backend/dist/sonicboom-backend`, extensionless by
 * repo convention — see the Windows entry-suffix note in
 * docs/PLUGIN_UI_ARCHITECTURE.md).
 */
import { copyFileSync, existsSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { $ } from 'bun';

const root = join(import.meta.dir, '..');
const dist = join(root, 'plugins', 'sonicboom', 'backend', 'dist');
const staged = join(dist, 'sonicboom-backend');

await $`node scripts/cargo-with-linker.mjs build -p sonicboom-backend --release --locked`.cwd(root);

const built = join(root, 'target', 'release', 'sonicboom-backend');
const builtExe = `${built}.exe`;
const source = existsSync(built) ? built : builtExe;
if (!existsSync(source)) {
  console.error(`expected backend binary at ${built} (or ${builtExe})`);
  process.exit(1);
}
mkdirSync(dist, { recursive: true });
copyFileSync(source, staged);
console.log(`staged ${staged}`);
