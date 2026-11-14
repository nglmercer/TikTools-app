import { isAbsolute, resolve } from 'node:path';

/** Parsing is pure: checking an artifact must never silently rebuild it. */
export function headlessTeamsOptions(args: string[], root: string) {
  const options = new Map<string, string>();
  for (let index = 0; index < args.length; index += 2) {
    const key = args[index], value = args[index + 1];
    if (!key || !['--binary', '--backend', '--report', '--cloud-port'].includes(key) || !value || value.startsWith('--') || options.has(key)) {
      throw new Error('usage: test headless-teams --binary ABSOLUTE_PATH [--backend PATH] [--report PATH] [--cloud-port PORT]');
    }
    options.set(key, value);
  }
  const binary = options.get('--binary');
  if (!binary || !isAbsolute(binary)) throw new Error('--binary must be an absolute path to the compiled CLI');
  const cloudPort = options.get('--cloud-port');
  if (cloudPort && (!/^\d+$/.test(cloudPort) || Number(cloudPort) < 1024 || Number(cloudPort) > 65535)) throw new Error('--cloud-port must be 1024..65535');
  return { binary, backend: resolve(options.get('--backend') ?? resolve(root, '../RemakeBackend')),
    report: resolve(options.get('--report') ?? resolve(root, 'test-results/headless-teams.json')), cloudPort };
}
