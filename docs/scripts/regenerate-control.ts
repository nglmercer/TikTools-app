import { execFileSync } from 'node:child_process';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { contractSources, digest } from './control-contract.ts';
const root = resolve(import.meta.dirname, '../..');
const profile = await mkdtemp(join(tmpdir(), 'tiktools-docs-schema-'));
try {
  execFileSync('cargo', ['build', '--locked', '-p', 'tiktools-cli', '--bin', 'tiktools'], { cwd: root, stdio: 'inherit' });
  const executable = resolve(root, 'target/debug', process.platform === 'win32' ? 'tiktools.exe' : 'tiktools');
  const schema = execFileSync(executable, ['--standalone', '--json', 'api', 'discover'], { cwd: root, env: { ...process.env, XDG_DATA_HOME: join(profile, 'data'), XDG_CONFIG_HOME: join(profile, 'config'), APPDATA: join(profile, 'roaming'), LOCALAPPDATA: join(profile, 'local') }, maxBuffer: 16 * 1024 * 1024 });
  const value = JSON.parse(schema.toString()) as { methods?: unknown[] };
  if (!Array.isArray(value.methods) || !value.methods.length) throw new Error('Missing discovered RPC methods.');
  await writeFile(resolve(root, 'docs/contracts/control-api.json'), schema);
  await writeFile(resolve(root, 'docs/contracts/provenance.json'), JSON.stringify({ version: 1, sourceCommit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(), generator: 'tiktools --standalone --json api discover', schemaSha256: digest(schema), sources: await contractSources() }, null, 2) + '\n');
  execFileSync(process.execPath, ['docs/scripts/generate-references.ts'], { cwd: root, stdio: 'inherit' });
} finally { await rm(profile, { recursive: true, force: true }); }
