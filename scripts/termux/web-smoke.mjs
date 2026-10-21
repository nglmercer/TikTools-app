// Development-only integration probe; uses the web test toolchain's ws package.
import assert from 'node:assert/strict';
import { request as httpRequest } from 'node:http';
import { spawn } from 'node:child_process';
import { access, mkdtemp, mkdir, writeFile, rm, symlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { constants } from 'node:fs';
import { resolve, join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import WebSocket from 'ws';

const binary = resolve(process.env.TIKTOOLS_BIN ?? 'target/debug/tiktools');
await access(binary, constants.X_OK);
const root = await mkdtemp(join(tmpdir(), 'tt-web-'));
const children = [];
const environment = { ...process.env };
delete environment.TIKTOOLS_SESSION_COOKIE;
delete environment.LD_LIBRARY_PATH;
for (const key of ['TIKTOOLS_HOME', 'TIKTOOLS_DATA_DIR', 'TIKTOOLS_PLUGINS_DIR', 'TIKTOOLS_PLUGIN_DATA_DIR', 'TIKTOOLS_LOG_DIR', 'TIKTOOLS_TEMP_DIR', 'TIKTOOLS_BUILTIN_PLUGINS_DIR', 'TIKTOOLS_DEV_PLUGINS_DIR']) {
  environment[key] = join(root, key.toLowerCase());
}
const assets = join(root, 'assets');
await mkdir(assets);
await writeFile(join(assets, 'index.html'), '<!doctype html><html><head></head><body>isolated adapter smoke</body></html>');
const plugin = join(environment.TIKTOOLS_PLUGINS_DIR, 'web-smoke-plugin');
await mkdir(join(plugin, 'ui', 'dist'), { recursive: true });
await writeFile(join(plugin, 'plugin.json'), JSON.stringify({
  schemaVersion: 3, apiVersion: 1, id: 'web-smoke-plugin', name: 'Web smoke', version: '1.0.0',
  runtime: 'napi-vm', entry: 'index.js',
  ui: { apiVersion: 1, mode: 'webview', entry: 'ui/dist/index.html', pages: [{ id: 'test', title: { default: 'Test' } }] },
}));
await writeFile(join(plugin, 'index.js'), 'export default {call(){return {logs:[],intents:[],events:[]};}};');
await writeFile(join(plugin, 'ui', 'dist', 'index.html'), '<html><head></head><body>plugin fixture</body></html>');
await writeFile(join(plugin, 'ui', 'dist', 'module.js'), 'export const fixture = true;');
await writeFile(join(root, 'private.txt'), 'private test data');
await symlink(join(root, 'private.txt'), join(plugin, 'ui', 'dist', 'escape.txt'));
function start(args) {
  const child = spawn(binary, args, { cwd: root, env: environment, stdio: ['ignore', 'pipe', 'pipe'] });
  child.output = '';
  child.once('error', error => { child.failure = error; });
  child.stdout.on('data', data => { child.output = (child.output + data).slice(-16384); });
  child.stderr.on('data', data => { child.output = (child.output + data).slice(-16384); });
  children.push(child); return child;
}
async function waitFor(predicate, label) {
  const deadline = Date.now() + 15_000;
  while (Date.now() < deadline) {
    const failed = children.find(child => child.failure);
    if (failed) throw new Error(`could not launch test process: ${failed.failure.message}`);
    if (predicate()) return; await delay(50);
  }
  throw new Error(`deadline: ${label}`);
}
async function stop(child) {
  if (!child.pid || child.exitCode !== null || child.signalCode !== null) return;
  child.kill('SIGTERM');
  await waitFor(() => child.exitCode !== null || child.signalCode !== null, 'child shutdown');
}
async function open(url, headers) {
  const socket = new WebSocket(url, { headers });
  await new Promise((resolve, reject) => { socket.once('open', resolve); socket.once('error', reject); });
  return socket;
}
async function denied(url, headers) {
  await assert.rejects(open(url, headers));
}
function rpc(socket, method, params = {}) {
  return new Promise((resolve, reject) => {
    const id = crypto.randomUUID();
    const deadline = setTimeout(() => { socket.off('message', received); reject(new Error(`RPC timeout: ${method}`)); }, 10_000);
    function received(raw) {
      const message = JSON.parse(raw);
      if (message.response?.id !== id) return;
      clearTimeout(deadline); socket.off('message', received); resolve(message.response);
    }
    socket.on('message', received);
    socket.send(JSON.stringify({ jsonrpc: '2.0', id, method, params }));
  });
}
try {
  let host = start(['host', '--ipc']);
  // A probe exits only after the IPC connection and real system.info succeed.
  const probe = start(['--json', 'system', 'info']);
  await waitFor(() => probe.exitCode !== null, 'host IPC');
  assert.equal(probe.exitCode, 0, 'isolated host available');
  const adapter = start(['web', '--assets', assets, '--port', '0']);
  await waitFor(() => /http:\/\/127\.0\.0\.1:\d+/.test(adapter.output), 'adapter ready');
  const origin = adapter.output.match(/http:\/\/127\.0\.0\.1:\d+/)[0];
  const code = adapter.output.match(/minutes\): ([0-9]{6})$/m)[1];
  const wsUrl = origin.replace('http:', 'ws:') + '/__local/ws';
  const pairingPage = await (await fetch(origin)).text();
  assert.match(pairingPage, /Pairing code/);
  assert.match(pairingPage, /inputmode="numeric"/);
  assert.match(pairingPage, /autocomplete="one-time-code"/);
  const forgedStatus = await new Promise((resolve, reject) => {
    const request = httpRequest(origin, { headers: { Host: 'attacker.example' } }, response => { response.resume(); resolve(response.statusCode); });
    request.on('error', reject); request.end();
  });
  assert.equal(forgedStatus, 403);
  await denied(wsUrl, { Origin: origin });
  assert.equal((await fetch(origin + '/__local/pair', { method: 'POST', headers: { 'Content-Type': 'application/json', Origin: 'https://attacker.example' }, body: JSON.stringify({ code }) })).status, 403);
  const paired = await fetch(origin + '/__local/pair', { method: 'POST', headers: { 'Content-Type': 'application/json', Origin: origin }, body: JSON.stringify({ code }) });
  assert.equal(paired.status, 200);
  const cookie = paired.headers.get('set-cookie').split(';')[0];
  assert.match(cookie.split('=')[1], /^[a-f0-9]{64}$/);
  assert.equal((await fetch(origin + '/__local/pair', { method: 'POST', headers: { 'Content-Type': 'application/json', Origin: origin }, body: JSON.stringify({ code }) })).status, 401);
  const html = await (await fetch(origin, { headers: { Cookie: cookie } })).text();
  assert.match(html, /tiktools-transport/);
  const assetBase = html.match(/name="tiktools-plugin-assets" content="([^"]+)"/)[1];
  const pluginIndex = await fetch(origin + assetBase + 'web-smoke-plugin/index.html');
  assert.equal(pluginIndex.status, 200); assert.match(await pluginIndex.text(), /plugin fixture/);
  assert.match(pluginIndex.headers.get('content-security-policy'), /sandbox allow-scripts/);
  const module = await fetch(origin + assetBase + 'web-smoke-plugin/module.js', { headers: { Origin: 'null' } });
  assert.equal(module.status, 200); assert.equal(module.headers.get('access-control-allow-origin'), 'null');
  assert.equal((await fetch(origin + assetBase + 'web-smoke-plugin/escape.txt')).status, 404);
  assert.equal((await fetch(origin + '/__plugins/wrong/web-smoke-plugin/index.html')).status, 404);
  assert.equal((await fetch(origin + '/%2e%2e%2fprivate', { headers: { Cookie: cookie } })).status, 400);
  await denied(wsUrl, { Cookie: cookie, Origin: 'https://attacker.example' });
  let socket = await open(wsUrl, { Cookie: cookie, Origin: origin });
  const extraSockets = [];
  try {
    for (let n = 0; n < 7; n++) extraSockets.push(await open(wsUrl, { Cookie: cookie, Origin: origin }));
    await denied(wsUrl, { Cookie: cookie, Origin: origin });
  } finally { for (const extra of extraSockets) extra.close(); }
  const info = await rpc(socket, 'system.info'); assert.ok(info.result);
  const discovery = await rpc(socket, 'rpc.discover'); assert.ok(discovery.result);
  const backfill = await rpc(socket, 'ui.feed.backfill', { version: 1, after: 0 });
  assert.deepEqual(backfill.result.events, []);
  const invalidVersion = await rpc(socket, 'ui.feed.backfill', { version: 2, after: 0 });
  assert.equal(invalidVersion.error.code, 'invalid_params');
  const unknown = await rpc(socket, 'nonexistent.method'); assert.equal(unknown.error.code, 'method_not_found');
  const eventReceived = new Promise((resolve, reject) => {
    const deadline = setTimeout(() => reject(new Error('workflow event missing')), 5000);
    function receive(raw) { const message = JSON.parse(raw); if (message.method === 'event' && message.params?.topic === 'workflow.changed') { clearTimeout(deadline); socket.off('message', receive); resolve(message); } }
    socket.on('message', receive);
  });
  const workflow = await rpc(socket, 'workflows.save', { graph: { id: 'web-smoke', name: 'Web smoke 👋', enabled: false, nodes: [], edges: [] } });
  assert.ok(!workflow.error); await eventReceived;
  const set = await rpc(socket, 'globals.set', { key: 'web-smoke', value: 'hello 👋' }); assert.ok(!set.error);
  const cli = start(['--json', 'globals', 'get', 'web-smoke']);
  await waitFor(() => cli.exitCode !== null, 'CLI shared state');
  assert.equal(cli.exitCode, 0); assert.match(cli.output, /hello 👋/);
  let disconnected = false; socket.once('close', () => { disconnected = true; });
  await stop(host); await waitFor(() => disconnected, 'host disconnect propagated to WebSocket');
  host = start(['host', '--ipc']);
  const restarted = start(['--json', 'system', 'info']);
  await waitFor(() => restarted.exitCode !== null, 'restarted host IPC'); assert.equal(restarted.exitCode, 0);
  socket = await open(wsUrl, { Cookie: cookie, Origin: origin });
  const persisted = await rpc(socket, 'globals.get', { key: 'web-smoke' });
  assert.equal(persisted.result.value, 'hello 👋');
  disconnected = false; socket.once('close', () => { disconnected = true; });
  await stop(adapter); await waitFor(() => disconnected, 'adapter shutdown closes active WebSocket');
  assert.equal(host.exitCode, null, 'adapter must not stop host');
  const after = start(['--json', 'system', 'health']);
  await waitFor(() => after.exitCode !== null, 'host still available'); assert.equal(after.exitCode, 0);
  await stop(host);
  console.log('PASS: isolated local HTTP/WS pairing, origin/Host rejection, discovery, errors, shared CLI state, bounded clients, plugin assets/backfill, restart persistence and independent shutdown');
} finally {
  for (const child of children.reverse()) {
    try { await stop(child); } catch { child.kill('SIGKILL'); }
  }
  await rm(root, { recursive: true, force: true });
}
