import { expect, test } from 'bun:test';
import { runSiteTask, type SiteTaskRunner } from './site-tasks.ts';

function recorder() {
  const calls: string[][] = [];
  const runner: SiteTaskRunner = {
    bun: '/usr/bin/bun',
    async run(...args) { calls.push(['run', ...args]); },
    async script(file, ...args) { calls.push(['script', file, ...args]); },
    async tool(name, ...args) { calls.push(['tool', name, ...args]); },
  };
  return { calls, runner };
}

test('docs dev starts the demo server so embedded frames have a proxy target', async () => {
  const { calls, runner } = recorder();
  await runSiteTask('docs', ['dev'], runner);
  expect(calls).toEqual([
    ['run', '/usr/bin/bun', 'run', '--cwd', 'docs', 'dev'],
    ['tool', 'vite', '--config', 'vite.demo.config.ts'],
  ]);
});

test('site dev reuses docs dev without starting a second demo server', async () => {
  const { calls, runner } = recorder();
  await runSiteTask('site', ['dev'], runner);
  expect(calls.filter((call) => call[1] === 'vite' && call[2] === '--config')).toHaveLength(1);
  expect(calls).toEqual([
    ['run', '/usr/bin/bun', 'run', '--cwd', 'docs', 'dev'],
    ['tool', 'vite', '--config', 'vite.demo.config.ts'],
  ]);
});
