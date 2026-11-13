import { spawn } from 'node:child_process';
import { closeSync, openSync } from 'node:fs';
import { readFile } from 'node:fs/promises';

/** Own the wrapper AND WebKit descendants. Regular files never wait for pipe EOF. */
export function startAppImageProcess(image: string, args: string[], env: NodeJS.ProcessEnv, capture: string) {
  const stdout = `${capture}.stdout`, stderr = `${capture}.stderr`;
  const out = openSync(stdout, 'w'), err = openSync(stderr, 'w');
  const child = (() => {
    try { return spawn(image, args, { env, detached: true, stdio: ['ignore', out, err] }); }
    finally { closeSync(out); closeSync(err); }
  })();
  let ended = false;
  const exited = new Promise<number | null>((resolve, reject) => {
    child.once('exit', (code) => { ended = true; resolve(code); });
    child.once('error', (error) => { ended = true; reject(error); });
  });
  // A launch error can arrive while the caller is polling the frontend log.
  void exited.catch(() => {});
  function signalGroup(signal: NodeJS.Signals) {
    if (!child.pid) return;
    try { process.kill(-child.pid, signal); }
    catch (error) { if ((error as NodeJS.ErrnoException).code !== 'ESRCH') throw error; }
  }
  async function wait(ms: number): Promise<boolean> {
    let timer: ReturnType<typeof setTimeout> | undefined;
    try { return await Promise.race([exited.then(() => true), new Promise<false>((resolve) => { timer = setTimeout(() => resolve(false), ms); })]); }
    finally { clearTimeout(timer); }
  }
  return {
    get ended() { return ended; },
    exited, wait,
    output: async () => Promise.all([readFile(stdout, 'utf8'), readFile(stderr, 'utf8')]),
    async stop() {
      signalGroup('SIGINT');
      try { await wait(2000); }
      finally { signalGroup('SIGKILL'); } // Wrapper exit alone does not establish descendant exit.
      if (!await wait(1000)) throw new Error('AppImage process group did not exit after SIGKILL');
    },
  };
}
