import { describe, expect, test } from 'bun:test';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  allocateFreePort,
  devUrl,
  findDesktopOwners,
  ipcEndpoint,
  legacyWindowsPipe,
  parsePsOwners,
  parseTasklistOwners,
  parseWhoamiSid,
  waitForHttp,
  windowsUserSid,
} from './dev-launch.ts';

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

  test('parseWhoamiSid extracts the SID and rejects garbage', () => {
    expect(parseWhoamiSid('"desktop-7\\user","S-1-5-21-1-2-3-1001"\r\n')).toBe(
      'S-1-5-21-1-2-3-1001',
    );
    expect(parseWhoamiSid('')).toBeUndefined();
    expect(parseWhoamiSid('ERROR: something failed')).toBeUndefined();
    expect(parseWhoamiSid('"a","not-a-sid!"')).toBeUndefined();
  });

  test('production windows pipe carries the user SID', () => {
    const previousName = process.env.TIKTOOLS_IPC_NAME;
    const previousSid = process.env.TIKTOOLS_TEST_USER_SID;
    delete process.env.TIKTOOLS_IPC_NAME;
    process.env.TIKTOOLS_TEST_USER_SID = 'S-1-5-21-1-2-3-1001';
    try {
      expect(windowsUserSid()).toBe('S-1-5-21-1-2-3-1001');
      expect(ipcEndpoint('win32')).toBe('\\\\.\\pipe\\tiktools-control-S-1-5-21-1-2-3-1001');
      expect(legacyWindowsPipe()).toBe('\\\\.\\pipe\\tiktools-control');
    } finally {
      if (previousName === undefined) delete process.env.TIKTOOLS_IPC_NAME;
      else process.env.TIKTOOLS_IPC_NAME = previousName;
      if (previousSid === undefined) delete process.env.TIKTOOLS_TEST_USER_SID;
      else process.env.TIKTOOLS_TEST_USER_SID = previousSid;
    }
  });

  test('invalid injected SID fails loudly', () => {
    const previous = process.env.TIKTOOLS_TEST_USER_SID;
    process.env.TIKTOOLS_TEST_USER_SID = 'bogus';
    try {
      expect(() => windowsUserSid()).toThrow(/invalid TIKTOOLS_TEST_USER_SID/);
    } finally {
      if (previous === undefined) delete process.env.TIKTOOLS_TEST_USER_SID;
      else process.env.TIKTOOLS_TEST_USER_SID = previous;
    }
  });

  test('waitForHttp refuses to treat a dead owner as ready', async () => {
    await expect(waitForHttp('http://127.0.0.1:9', () => false, 2, 1)).rejects.toThrow(
      /exited before becoming ready/,
    );
  });

  test('parseTasklistOwners finds desktop rows and ignores INFO noise', () => {
    const output = [
      '"tiktools-desktop.exe","1234","Console","1","48,000 K"',
      '"code.exe","5678","Console","1","120,000 K"',
    ].join('\r\n');
    expect(parseTasklistOwners(output)).toEqual([1234]);
    expect(parseTasklistOwners('INFO: No tasks are running which match the specified criteria.')).toEqual(
      [],
    );
    expect(parseTasklistOwners('')).toEqual([]);
  });

  test('parsePsOwners finds the binary and cargo-run hosts only', () => {
    const output = [
      '  101 /home/u/TikTools/target/debug/tiktools-desktop',
      '  102 cargo run -p tiktools-desktop',
      '  103 cargo build -p tiktools-desktop',
      '  104 /usr/bin/code /home/u/TikTools/target/debug/tiktools-desktop',
      '  105 /home/u/TikTools/target/debug/tiktools-desktop-helper',
    ].join('\n');
    // The helper binary shares the prefix but is a different argv[0] base.
    expect(parsePsOwners(output)).toEqual([101, 102]);
  });

  test('findDesktopOwners never throws and fails open', () => {
    expect(() => findDesktopOwners('linux')).not.toThrow();
    expect(Array.isArray(findDesktopOwners('linux'))).toBe(true);
  });
});
