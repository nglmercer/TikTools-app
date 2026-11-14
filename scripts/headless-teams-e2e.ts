/** Real compiled CLI → IPC owner → native Cloud peer, against an isolated backend fixture. */
import { spawn, type ChildProcess } from 'node:child_process';
import { createReadStream } from 'node:fs';
import { createHash, randomUUID } from 'node:crypto';
import { mkdtemp, mkdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { isAbsolute, join, resolve } from 'node:path';
import { createInterface } from 'node:readline';

type Json = Record<string, unknown>;
type Fixture = { origin: string; browserOrigin: string; ownerCookie: string; moderatorCookie: string; outsiderCookie: string; moderatorNickname: string; moderatorId: string; runId: string };
function options(args: string[]): Map<string, string> {
  const parsed = new Map<string, string>();
  for (let i = 0; i < args.length; i += 2) {
    const key = args[i], value = args[i + 1];
    if (!key || !['--binary', '--fixture', '--report'].includes(key) || !value || parsed.has(key)) throw new Error('expected --binary PATH --fixture PATH --report PATH');
    parsed.set(key, resolve(value));
  }
  if (parsed.size !== 3) throw new Error('binary, private fixture and report are required');
  return parsed;
}
function check(value: unknown, message: string): asserts value { if (!value) throw new Error(message); }
const opts = options(process.argv.slice(2));
const binary = opts.get('--binary')!;
const fixturePath = opts.get('--fixture')!;
const reportPath = opts.get('--report')!;
check(isAbsolute(binary) && (await stat(binary)).isFile(), 'compiled CLI is missing');
const fixture = JSON.parse(await readFile(fixturePath, 'utf8')) as Fixture;
const origin = new URL(fixture.origin);
check(origin.protocol === 'https:' && origin.hostname === '127.0.0.1' && origin.port && origin.pathname === '/' && !origin.username && !origin.password && !origin.search && !origin.hash, 'fixture must use isolated loopback HTTPS');
check(typeof fixture.ownerCookie === 'string' && typeof fixture.moderatorCookie === 'string' && typeof fixture.outsiderCookie === 'string', 'fixture accounts are missing');
const root = await mkdtemp(join(tmpdir(), 'tiktools-headless-teams-'));
const homes = ['owner', 'agent', 'outsider'].map(name => join(root, name));
const [owner, agent, outsider] = homes as [string, string, string];
for (const home of homes) await mkdir(home, { mode: 0o700 });
const owned = new Set<ChildProcess>();
let cancelled = false;
let cleaningUp = false;
function cancel(): void {
  cancelled = true;
  for (const child of owned) {
    if (child.pid) { try { process.kill(-child.pid, 'SIGTERM'); } catch { /* exited */ } }
  }
}
process.on('SIGINT', cancel); process.on('SIGTERM', cancel);
async function binaryDigest(): Promise<string> {
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(binary)) hash.update(chunk as Buffer);
  return hash.digest('hex');
}
const report: { runId: string; binarySha256: string; tests: { name: string; passed: boolean; durationMs: number }[]; ok: boolean; error?: string; diagnostics?: unknown } = {
  runId: fixture.runId, binarySha256: await binaryDigest(), tests: [], ok: false,
};
function environment(home: string): NodeJS.ProcessEnv {
  const env = Object.fromEntries(Object.entries(process.env).filter(([name]) => !name.startsWith('TIKTOOLS_')));
  return { ...env, TIKTOOLS_HOME: home, TIKTOOLS_CLOUD_ORIGIN: fixture.origin, TIKTOOLS_REMOTE_PROFILES_DIR: join(home, 'profiles'),
    TIKTOOLS_DATA_DIR: join(home, 'data'), TIKTOOLS_PLUGINS_DIR: join(home, 'plugins'), TIKTOOLS_PLUGIN_DATA_DIR: join(home, 'plugin-data'),
    TIKTOOLS_BUILTIN_PLUGINS_DIR: join(home, 'builtins'), TIKTOOLS_DEV_PLUGINS_DIR: join(home, 'dev-plugins'), TIKTOOLS_LOG_DIR: join(home, 'logs'), TIKTOOLS_TEMP_DIR: join(home, 'temp') };
}
function start(home: string, args: string[]): ChildProcess {
  if (cancelled && !cleaningUp) throw new Error('acceptance cancelled');
  const child = spawn(binary, args, { cwd: home, env: environment(home), detached: true, stdio: ['pipe', 'pipe', 'pipe'] });
  owned.add(child);
  child.once('error', () => {});
  child.stdin!.on('error', () => {}); // Closed remote sessions surface through EOF/results, never an unhandled EPIPE.
  return child;
}
async function stop(child: ChildProcess): Promise<void> {
  if (child.exitCode === null && child.pid) {
    try { process.kill(-child.pid, 'SIGTERM'); } catch { /* already exited */ }
    await Promise.race([new Promise(resolve => child.once('close', resolve)), Bun.sleep(1500)]);
    try { process.kill(-child.pid, 'SIGKILL'); } catch { /* already exited */ }
  }
  owned.delete(child);
}
async function command(home: string, args: string[], input?: string, expectedFailure = false): Promise<Json> {
  const label = args[0] === '--remote-profile' ? `remote.${args[4] ?? args[2]}` : args[0] === 'api' ? `api.${args[2]}` : `${args[0]}.${args[1] ?? ''}`;
  const child = start(home, ['--json', ...args]);
  let output = ''; let errors = '';
  child.stdout!.on('data', value => { output += String(value); if (output.length > 4 * 1024 * 1024) child.kill(); });
  child.stderr!.on('data', value => { errors = (errors + String(value)).slice(-32768); });
  child.stdin!.end(input);
  const timer = setTimeout(() => { void stop(child); }, cleaningUp ? 5000 : 35000);
  try {
    const code = await new Promise<number | null>((resolve, reject) => { child.once('close', resolve); child.once('error', reject); });
    let value: Json;
    try { value = JSON.parse(output) as Json; } catch { throw new Error(`CLI ${label} returned invalid JSON (exit ${code}); stderr bytes=${errors.length}; cryptoProviderPanic=${errors.includes('CryptoProvider')}`); }
    if (expectedFailure) check(code !== 0, `CLI ${label} unexpectedly succeeded`);
    else check(code === 0, `CLI ${label} failed: ${String((value['error'] as Json | undefined)?.['code'] ?? value['code'] ?? code)} (${String((value['error'] as Json | undefined)?.['message'] ?? '')})`);
    return value;
  } finally { clearTimeout(timer); await stop(child); }
}
const rpc = (home: string, method: string, params: Json = {}, more: string[] = [], failure = false) => command(home, ['api', 'call', method, JSON.stringify(params), '--confirm', '--timeout', '15', ...more], undefined, failure);
async function until<T>(read: () => Promise<T>, valid: (value: T) => boolean, message: string, ms = 15000): Promise<T> {
  const deadline = Date.now() + ms;
  do { const value = await read(); if (valid(value)) return value; await Bun.sleep(500); } while (Date.now() < deadline);
  throw new Error(message);
}
async function test(name: string, operation: () => Promise<void>): Promise<void> {
  const start = Date.now();
  try { await operation(); report.tests.push({ name, passed: true, durationMs: Date.now() - start }); console.log(`PASS ${name}`); }
  catch (failure) { report.tests.push({ name, passed: false, durationMs: Date.now() - start }); throw failure; }
}
async function browserApproval(browserUrl: string, cookie: string): Promise<void> {
  const browser = new URL(browserUrl);
  check(browser.origin === origin.origin, 'login URL escaped the fixture origin');
  const query = browser.searchParams;
  const response = await fetch(`${fixture.origin}/api/v1/desktop/handoff`, { method: 'POST', redirect: 'error',
    headers: { cookie, origin: fixture.browserOrigin, 'content-type': 'application/json' },
    body: JSON.stringify({ redirectUri: query.get('redirectUri'), codeChallenge: query.get('codeChallenge'), state: query.get('state') }) });
  check(response.ok, `fixture browser approval failed (${response.status})`);
  const result = await response.json() as { callbackUrl: string };
  const callback = new URL(result.callbackUrl);
  check(callback.protocol === 'http:' && callback.hostname === '127.0.0.1', 'callback escaped loopback');
  const delivered = await fetch(callback, { redirect: 'error' });
  check(delivered.ok, 'native PKCE callback failed');
}
async function loginController(home: string, cookie: string): Promise<void> {
  const child = start(home, ['--json', 'remote', 'cloud', 'login']);
  let output = ''; child.stdout!.on('data', chunk => { output += String(chunk); });
  const lines = createInterface({ input: child.stderr! });
  const iterator = lines[Symbol.asyncIterator]();
  const deadline = setTimeout(() => { void stop(child); }, 20000);
  try {
    let loginUrl: string | undefined;
    for await (const line of { [Symbol.asyncIterator]: () => iterator }) { if (line.startsWith(fixture.origin)) { loginUrl = line; break; } }
    check(loginUrl, 'controller did not announce a native login URL');
    const complete = new Promise<number | null>(resolve => child.once('close', resolve));
    await browserApproval(loginUrl, cookie);
    check(await complete === 0 && (JSON.parse(output) as Json)['state'] === 'signed-in', 'controller login did not complete');
  } finally { clearTimeout(deadline); lines.close(); await stop(child); }
}
async function agentSession(withEvents = false) {
  const child = start(agent, ['--json', 'remote', 'serve', '--profile', 'agent', '--stdio', '--timeout', '10', ...(withEvents ? ['--events'] : [])]);
  const completed = new Promise<number | null>(resolve => child.once('close', resolve));
  child.stderr!.resume();
  const lines = createInterface({ input: child.stdout! });
  const iterator = lines[Symbol.asyncIterator]();
  async function next(): Promise<Json> {
    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
      const line = await Promise.race([iterator.next(), new Promise<never>((_, reject) => { timer = setTimeout(() => reject(new Error('remote NDJSON response deadline exceeded')), 15000); })]);
      check(!line.done, 'remote NDJSON ended before its response');
      return JSON.parse(line.value) as Json;
    } finally { clearTimeout(timer); }
  }
  const ready = await next();
  check(ready['method'] === 'connection.ready', `remote NDJSON connection failed: ${String((ready['error'] as Json | undefined)?.['code'])}`);
  const hello = ready['params'] as Json;
  let sequence = 0;
  const events: Json[] = [];
  return {
    hello, events,
    async waitDisconnected(): Promise<void> {
      let timer: ReturnType<typeof setTimeout> | undefined;
      try {
        const code = await Promise.race([completed, new Promise<never>((_, reject) => { timer = setTimeout(() => reject(new Error('idle remote NDJSON remained open after revocation')), 15000); })]);
        check(code !== 0, 'revoked idle client exited successfully');
      } finally { clearTimeout(timer); }
    },
    async call(method: string, params: Json = {}, meta?: Json, confirm = false): Promise<Json> {
      const id = `headless-${++sequence}`;
      child.stdin!.write(JSON.stringify({ jsonrpc: '2.0', id, method, params, meta, confirm }) + '\n');
      let response = await next();
      while (response['method'] && !('id' in response)) {
        events.push(response); if (events.length > 32) events.shift();
        response = await next();
      }
      check(response['id'] === id || response['id'] === null && response['error'], 'persistent client changed a request ID');
      return response;
    },
    async close(allowDisconnected = false): Promise<void> {
      child.stdin!.end();
      let timer: ReturnType<typeof setTimeout> | undefined;
      try { const code = await Promise.race([completed, new Promise<never>((_, reject) => { timer = setTimeout(() => reject(new Error('remote NDJSON did not close on EOF')), 5000); })]); if (!allowDisconnected) check(code === 0, 'remote NDJSON exit failed'); }
      finally { clearTimeout(timer); lines.close(); await stop(child); }
    },
  };
}
async function hostReady(): Promise<ChildProcess> {
  const host = start(owner, ['host', '--ipc']);
  host.stdin!.end(); host.stdout!.resume(); host.stderr!.resume();
  await until(() => rpc(owner, 'system.ping').catch(() => null), value => value !== null, 'headless IPC host did not become ready');
  return host;
}
let host: ChildProcess | undefined;
let teamId = ''; let grantId = ''; let grantRevision = 0;
try {
  await test('artifact advertises headless features', async () => {
    const build = await command(owner, ['--build-info']);
    check(build['cloudOrigin'] === fixture.origin, 'artifact is pinned to another Cloud origin; use a development binary or a release built for the isolated fixture origin');
    const features = build['features'] as Json;
    check(features['remoteControl'] === true && features['cloudAuth'] === true && features['remoteClient'] === true, 'CLI artifact lacks required headless features');
    host = await hostReady();
    const methods = await command(owner, ['api', 'discover']);
    check(Array.isArray(methods['methods']) && (methods['methods'] as Json[]).some(v => v['name'] === 'teams.hosts.create'), 'Teams API missing from artifact');
  });
  await test('real native authentication for owner and controllers', async () => {
    const begin = await rpc(owner, 'cloud.login.begin');
    const complete = rpc(owner, 'cloud.login.complete');
    await browserApproval(String(begin['browserUrl']), fixture.ownerCookie);
    check((await complete)['state'] === 'signed-in', 'owner login failed');
    await loginController(agent, fixture.moderatorCookie);
    await loginController(outsider, fixture.outsiderCookie);
    await command(outsider, ['remote', 'cloud', 'enroll', '--label', 'Unapproved outsider']);
    check((await command(agent, ['remote', 'cloud', 'status']))['verified'] === true, 'saved controller session is not verifiable');
  });
  await test('create and serve a team without a window', async () => {
    const team = await command(owner, ['teams', 'create', '--name', 'Headless acceptance', '--confirm']); teamId = String(team['id']);
    await command(owner, ['teams', 'host', 'enroll', '--team', teamId, '--label', 'Isolated host', '--confirm']);
    check((await command(owner, ['teams', 'host', 'start', '--wait', '--timeout', '20', '--confirm']))['state'] === 'ready', 'host did not become ready');
    check((await command(owner, ['teams', 'doctor']))['ok'] === true, 'host diagnostics failed');
  });
  await test('nickname-bound private invitation and pending approval', async () => {
    const people = await command(owner, ['teams', 'people', 'search', fixture.moderatorNickname]);
    check((people['items'] as Json[]).some(v => v['id'] === fixture.moderatorId && !('email' in v)), 'nickname lookup failed or exposed emails');
    const path = join(root, 'invitation');
    const invitation = await command(owner, ['teams', 'invitations', 'create', '--team', teamId, '--role', 'observer', '--recipient', fixture.moderatorId, '--output-file', path, '--confirm']);
    check(!('token' in invitation), 'invitation token escaped private output');
    if (process.platform !== 'win32') check(((await stat(path)).mode & 0o077) === 0, 'private invitation permissions are too broad');
    const pending = await command(agent, ['teams', 'join', '--profile', 'agent', '--kind', 'agent', '--invite-stdin', '--non-interactive', '--timeout', '15'], await readFile(path, 'utf8'));
    check(pending['state'] === 'approval_pending', 'redeeming silently approved the agent');
    await rm(path);
    const denied = await command(agent, ['remote', 'cloud', 'connect', '--host', teamId, '--profile', 'agent', '--kind', 'agent', '--non-interactive', '--timeout', '3'], undefined, true);
    check(['approval_pending', 'unauthorized', 'timeout'].includes(String((denied['error'] as Json)?.['code'])), 'pending admission failed for an unexpected reason');
    await command(owner, ['teams', 'requests', 'approve', '--team', teamId, '--request', String(pending['requestId']), '--confirm']);
    const grants = await command(owner, ['teams', 'grants', 'list', '--team', teamId]);
    const grant = (grants['items'] as Json[])[0]!; grantId = String(grant['id']);
    const policy = { nativeClientKind: 'agent', nativeGrant: { capabilities: ['live.read', 'points.read', 'points.adjust', 'commands.read'], resources: { viewer: ['alice'] }, maxPointsDelta: 10 } };
    await command(owner, ['teams', 'grants', 'update', '--team', teamId, '--grant', grantId, '--expected-revision', String(grant['revision']), '--role', 'custom', '--policy', JSON.stringify(policy), '--confirm']);
    await until(() => command(owner, ['teams', 'grants', 'list', '--team', teamId]), value => {
      const grant = (value['items'] as Json[]).find(v => v['id'] === grantId); grantRevision = Number(grant?.['revision']); return grant?.['status'] === 'active';
    }, 'host did not confirm the reviewed agent policy', 30000);
    await command(agent, ['remote', 'cloud', 'connect', '--host', teamId, '--profile', 'agent', '--kind', 'agent', '--wait', '--non-interactive', '--timeout', '25']);
    const memberships = await command(agent, ['teams', 'list', '--joined']);
    check(Array.isArray(memberships) ? memberships.length === 1 : Array.isArray(memberships['items']) && memberships['items'].length === 1, 'controller memberships not discoverable');
  });
  await test('persistent stdio preserves request IDs and closes on EOF', async () => {
    const session = await agentSession();
    try { check('result' in await session.call('live.status'), 'persistent client did not perform the real read'); }
    finally { await session.close(); }
  });
  await test('real delegated reads, bounded writes, events and command deduplication', async () => {
    const session = await agentSession(true);
    try {
      check('result' in await session.call('live.status'), 'authorized read failed');
      const subscribed = (await session.call('collaboration.subscribe', { domains: ['points'], pointsLimit: 0 }, undefined, true))['result'] as Json;
      check(subscribed?.['subscriptionId'], 'points subscription was not created');
      const baseline = (await session.call('collaboration.snapshot', { subscriptionId: subscribed['subscriptionId'] }, undefined, true))['result'] as Json;
      check(baseline?.['throughSequence'], 'points baseline was not delivered');
      check('result' in await session.call('collaboration.resume', { subscriptionId: subscribed['subscriptionId'], hostInstanceId: baseline['hostInstanceId'], lastAppliedSequence: baseline['throughSequence'] }, undefined, true), 'points event stream did not resume');
      const params = { uniqueId: 'alice', delta: 5 };
      const meta = { commandId: randomUUID(), hostInstanceId: session.hello['hostInstanceId'] };
      const unconfirmed = await session.call('points.adjust', params, meta);
      check((unconfirmed['error'] as Json | undefined)?.['code'] === 'confirmation_required', 'persistent agent write did not require explicit confirmation');
      const first = await session.call('points.adjust', params, meta, true);
      check('result' in first, `authorized durable write failed: ${String((first['error'] as Json | undefined)?.['code'])}`);
      const replay = await session.call('points.adjust', params, meta, true);
      check(JSON.stringify(first['result']) === JSON.stringify(replay['result']), 'durable command replay returned a different result');
      const state = await rpc(owner, 'points.viewer.get', { uniqueId: 'alice' });
      check((state['viewer'] as Json)?.['points'] === 5, 'durable command was lost or applied twice in owner storage');
      const excessive = await session.call('points.adjust', { uniqueId: 'alice', delta: 11 }, { commandId: randomUUID(), hostInstanceId: session.hello['hostInstanceId'] }, true);
      check((excessive['error'] as Json | undefined)?.['code'] === 'forbidden', 'points delta limit was not enforced');
      await until(async () => { await session.call('live.status'); return session.events; }, events => events.some(event => String((event['params'] as Json | undefined)?.['topic']).startsWith('points.')), 'typed points events were not forwarded through stdio', 5000);
      const denied = await session.call('points.adjust', { uniqueId: 'bob', delta: 5 }, { commandId: randomUUID(), hostInstanceId: session.hello['hostInstanceId'] }, true);
      check((denied['error'] as Json | undefined)?.['code'] === 'forbidden', 'resource restriction was not enforced');
      check(((await rpc(owner, 'points.viewer.get', { uniqueId: 'alice' }))['viewer'] as Json)?.['points'] === 5, 'rejected adjustments changed owner data');
      const unrelated = await command(outsider, ['remote', 'cloud', 'connect', '--host', teamId, '--profile', 'outsider', '--kind', 'agent', '--non-interactive', '--timeout', '3'], undefined, true);
      check(['forbidden', 'unauthorized', 'approval_pending'].includes(String((unrelated['error'] as Json | undefined)?.['code'])), `another account rejection was not authorization: ${String((unrelated['error'] as Json | undefined)?.['code'])}`);
    } finally { await session.close(); }
  });
  await test('restart recovers persisted host and agent identities', async () => {
    await rpc(owner, 'system.shutdown'); if (host) await stop(host);
    host = await hostReady(); await command(owner, ['cloud', 'restore']);
    await command(owner, ['teams', 'host', 'start', '--wait', '--timeout', '20', '--confirm']);
    check((await command(agent, ['remote', 'check', 'agent']))['connected'] === true, 'agent profile failed after host restart');
  });
  await test('revocation removes authorization from existing and new sessions', async () => {
    const session = await agentSession();
    try {
      check('result' in await session.call('live.status'), 'revocation test requires an initially authorized live session');
      const grants = await command(owner, ['teams', 'grants', 'list', '--team', teamId]);
      grantRevision = Number((grants['items'] as Json[]).find(v => v['id'] === grantId)?.['revision']);
      await command(owner, ['teams', 'grants', 'revoke', '--team', teamId, '--grant', grantId, '--expected-revision', String(grantRevision), '--confirm']);
      // Keep stdin open and send no more requests. Revocation must wake an
      // idle client even when --events was not selected.
      await session.waitDisconnected();
      await command(agent, ['remote', 'check', 'agent'], undefined, true);
    } finally { await session.close(true); }
  });
  check(await binaryDigest() === report.binarySha256, 'CLI artifact changed during acceptance');
  report.ok = true;
} catch (failure) {
  report.error = failure instanceof Error ? failure.message : 'acceptance failed';
  report.diagnostics = await Promise.allSettled([rpc(owner, 'teams.host.status'), rpc(owner, 'collaboration.status')]).then(results => results.map(item => item.status === 'fulfilled' ? { host: item.value['state'], lastError: item.value['lastError'], gateway: (item.value['gateway'] as Json | undefined)?.['lastError'] } : { unavailable: true }));
} finally {
  cleaningUp = true;
  // Logout only the fixture identities created above, while their server is alive.
  const ownerLogout = command(owner, ['cloud', 'logout']).catch(async () => {
    if (!host || host.exitCode !== null || host.signalCode !== null) return command(owner, ['--standalone', 'cloud', 'logout']);
    throw new Error('fixture owner logout failed');
  });
  const cleanup = await Promise.allSettled([ownerLogout, command(agent, ['remote', 'cloud', 'logout']), command(outsider, ['remote', 'cloud', 'logout'])]);
  if (cleanup.some(item => item.status === 'rejected')) { report.ok = false; report.error ??= 'fixture credential cleanup failed'; }
  await Promise.all([...owned].map(stop));
  await mkdir(resolve(reportPath, '..'), { recursive: true });
  await writeFile(reportPath, JSON.stringify(report, null, 2));
  await rm(root, { recursive: true, force: true });
  process.off('SIGINT', cancel); process.off('SIGTERM', cancel);
}
if (!report.ok) { console.error(report.error); process.exitCode = 1; }
