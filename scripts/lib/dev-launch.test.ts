import { describe, expect, test } from 'bun:test';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { allocateFreePort, devUrl, ipcEndpoint, waitForHttp } from './dev-launch.ts';

describe('dev-launch', () => {
  test('devUrl pins loopback host', () => {
    expect(devUrl(5123)).toBe('http://127.0.0.1:5123');
  });

  test('allocateFreePort returns a bindable port', async () => {
    const port = await allocateFreePort();
    expect(port).toBeGreaterThan(0);
    expect(port).toBeLessThanOrEqual(65535);
    // The allocated port must be free for Vite to bind with strictPort.
    await new Promise<void>((resolve, reject) => {
      const server = createServer();
      server.on('error', reject);
      server.listen(port, '127.0.0.1', () => {
        server.close((error) => (error ? reject(error) : resolve()));
      });
    });
  });

  test('ipcEndpoint honors TIKTOOLS_HOME on unix', () => {
    const previous = process.env.TIKTOOLS_HOME;
    process.env.TIKTOOLS_HOME = join(tmpdir(), 'tiktools-dev-launch-test');
    try {
      expect(ipcEndpoint('linux')).toBe(
        join(tmpdir(), 'tiktools-dev-launch-test', 'tiktools-control.sock'),
      );
    } finally {
      if (previous === undefined) delete process.env.TIKTOOLS_HOME;
      else process.env.TIKTOOLS_HOME = previous;
    }
  });

  test('ipcEndpoint honors TIKTOOLS_IPC_NAME on windows', () => {
    const previous = process.env.TIKTOOLS_IPC_NAME;
    process.env.TIKTOOLS_IPC_NAME = 'tiktools-control-test-1';
    try {
      expect(ipcEndpoint('win32')).toBe('\\\\.\\pipe\\tiktools-control-test-1');
    } finally {
      if (previous === undefined) delete process.env.TIKTOOLS_IPC_NAME;
      else process.env.TIKTOOLS_IPC_NAME = previous;
    }
  });

  test('waitForHttp refuses to treat a dead owner as ready', async () => {
    await expect(waitForHttp('http://127.0.0.1:9', () => false, 2, 1)).rejects.toThrow(
      /exited before becoming ready/,
    );
  });
});
