/** Provision a disposable real backend and exercise an existing CLI artifact. */
import { stat } from 'node:fs/promises';
import { join } from 'node:path';
import { headlessTeamsOptions } from './lib/headless-teams.ts';
import { createProcessRunner, forwardTerminationSignals } from './lib/process-runner.ts';

const root = join(import.meta.dir, '..');
const options = headlessTeamsOptions(process.argv.slice(2), root);
if (!(await stat(options.binary).catch(() => null))?.isFile()) throw new Error('compiled CLI does not exist');
if (!(await stat(join(options.backend, 'crates/cloud-api/Cargo.toml')).catch(() => null))?.isFile()) throw new Error('--backend must identify a RemakeBackend checkout with the CLI acceptance fixture');
const runner = createProcessRunner({ cwd: root });
const dispose = forwardTerminationSignals(runner);
try {
  await runner.run(['python3', 'scripts/collaboration-cloud-lab.py', '--cli', options.binary,
    '--backend', options.backend, '--report', options.report, ...(options.cloudPort ? ['--cloud-port', options.cloudPort] : [])]);
} finally { dispose(); }
