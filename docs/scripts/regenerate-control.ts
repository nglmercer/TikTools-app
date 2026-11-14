import { execFileSync } from 'node:child_process';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join, isAbsolute } from 'node:path';
import { contractSources, digest } from './control-contract.ts';
const root = resolve(import.meta.dirname, '../..');
const profile = await mkdtemp(join(tmpdir(), 'tiktools-docs-schema-'));
try {
  const args = process.argv.slice(2);
  if (args.length && (args.length !== 2 || args[0] !== '--binary' || !isAbsolute(args[1]!))) throw new Error('usage: regenerate-control.ts [--binary ABSOLUTE_PATH]');
  let executable = args[1];
  if (!executable) {
    const build = execFileSync('cargo', ['build', '--locked', '-p', 'tiktools-cli', '--bin', 'tiktools', '--message-format=json'], { cwd: root, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024, stdio: ['ignore', 'pipe', 'inherit'] });
    for (const line of build.split('\n')) {
      let artifact: { target?: { name?: string }; executable?: string };
      try { artifact = JSON.parse(line) as typeof artifact; } catch { continue; }
      if (artifact.target?.name === 'tiktools' && artifact.executable) executable = artifact.executable;
    }
    if (!executable) throw new Error('Cargo did not produce a CLI executable');
  }
  const env = Object.fromEntries(Object.entries(process.env).filter(([name]) => !name.startsWith('TIKTOOLS_')));
  const schema = execFileSync(executable, ['--standalone', '--json', 'api', 'discover'], { cwd: profile,
    env: { ...env, TIKTOOLS_HOME: join(profile, 'host'), TIKTOOLS_DATA_DIR: join(profile, 'data'), TIKTOOLS_BUILTIN_PLUGINS_DIR: join(profile, 'empty-plugins'), TIKTOOLS_DEV_PLUGINS_DIR: join(profile, 'dev-plugins'), TIKTOOLS_CLOUD_ORIGIN: '',
      XDG_DATA_HOME: join(profile, 'xdg-data'), XDG_CONFIG_HOME: join(profile, 'config'), APPDATA: join(profile, 'roaming'), LOCALAPPDATA: join(profile, 'local') }, maxBuffer: 16 * 1024 * 1024 });
  const value = JSON.parse(schema.toString()) as { methods?: unknown[] };
  if (!Array.isArray(value.methods) || !value.methods.length) throw new Error('Missing discovered RPC methods.');
  await writeFile(resolve(root, 'docs/contracts/control-api.json'), schema);
  await writeFile(resolve(root, 'docs/contracts/provenance.json'), JSON.stringify({ version: 1, sourceCommit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(), generator: 'tiktools --standalone --json api discover', schemaSha256: digest(schema), sources: await contractSources() }, null, 2) + '\n');
  execFileSync(process.execPath, ['docs/scripts/generate-references.ts'], { cwd: root, stdio: 'inherit' });
} finally { await rm(profile, { recursive: true, force: true }); }
