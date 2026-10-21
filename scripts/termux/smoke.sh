#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}
if [[ -z ${TIKTOOLS_BIN:-} ]]; then
  cargo build -p tiktools-cli --locked
  export TIKTOOLS_BIN="$PWD/target/debug/tiktools"
fi
python - <<'PY'
import json, os, pathlib, select, signal, subprocess, tempfile, time
cli = str(pathlib.Path(os.environ['TIKTOOLS_BIN']).resolve())
with tempfile.TemporaryDirectory(prefix='tt-', dir=os.environ.get('TMPDIR', os.environ['HOME'])) as scratch:
    root = pathlib.Path(scratch)
    env = os.environ.copy()
    env.pop('TIKTOOLS_SESSION_COOKIE', None)
    env['TIKTOOLS_HOME'] = str(root / 'home')
    for item in ['DATA', 'PLUGINS', 'PLUGIN_DATA', 'LOG', 'TEMP', 'BUILTIN_PLUGINS', 'DEV_PLUGINS']:
        env[f'TIKTOOLS_{item}_DIR'] = str(root / item)
    def run(*args):
        proc = subprocess.run([cli, '--json', *args], cwd=root, env=env,
                              capture_output=True, text=True, timeout=30)
        if proc.returncode:
            raise RuntimeError(f'{args}: {proc.returncode}: {proc.stderr}: {proc.stdout}')
        value = json.loads(proc.stdout)
        assert not (isinstance(value, dict) and value.get('jsonrpc') == '2.0' and 'error' in value), value
        return value
    host = None
    log = open(root / 'host.log', 'w+')
    def start():
        global host
        host = subprocess.Popen([cli, 'host', '--ipc'], cwd=root, env=env, stdout=log, stderr=log)
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            if host.poll() is not None:
                log.seek(0)
                raise RuntimeError('host exited: ' + log.read())
            try:
                info = run('system', 'info')
                assert info['pid'] == host.pid, 'connected to unexpected runtime'
                return
            except (RuntimeError, subprocess.TimeoutExpired):
                time.sleep(.1)
        raise RuntimeError('IPC startup deadline exceeded')
    try:
        run('--standalone', 'system', 'info')
        start()
        duplicate = subprocess.run([cli, 'host', '--ipc'], cwd=root, env=env,
            capture_output=True, text=True, timeout=10)
        assert duplicate.returncode != 0, 'duplicate host acquired ownership'
        for args in [('system','health'), ('system','doctor'), ('live','status'),
                     ('plugin','list'), ('processor','list'), ('processor','status'),
                     ('automation','list'), ('workflow','list'), ('api','discover')]:
            run(*args)
        assert run('system','info')['pid'] == host.pid
        socket_path = root / 'home' / 'tiktools-control.sock'
        assert socket_path.stat().st_mode & 0o777 == 0o600
        # Validate the public event inspector, with retries to avoid a
        # subscription/startup race. Only this isolated points store changes.
        watcher = subprocess.Popen([cli, '--json', 'api', 'events', '--topics',
            'points.*', '--max-events', '1', '--timeout', '8'], cwd=root, env=env,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        try:
            for _ in range(10):
                run('points', 'adjust', 'smoke', '1')
                if watcher.poll() is not None:
                    break
                time.sleep(.1)
            output, error = watcher.communicate(timeout=10)
            assert watcher.returncode == 0, error
            assert 'points.changed' in json.dumps(json.loads(output)), output
        finally:
            if watcher.poll() is None:
                watcher.terminate()
                watcher.wait(timeout=5)
        # All mutations below are confined to this throwaway runtime.
        run('globals', 'set', 'termuxSmoke', 'Unicode 😀')
        host.send_signal(signal.SIGTERM)
        assert host.wait(timeout=35) == 0
        assert not socket_path.exists(), 'SIGTERM left socket behind'
        portable = root / 'PLUGINS' / 'portable-smoke'
        portable.mkdir(parents=True, exist_ok=True)
        (portable / 'plugin.json').write_text(json.dumps({
            'schemaVersion':3, 'apiVersion':1, 'id':'portable-smoke', 'name':'PortableSmoke',
            'version':'1.0.0', 'runtime':'napi-vm', 'entry':'index.js'}))
        (portable / 'index.js').write_text('export default {call(request, context) { return {summary: "Android", logs: [], intents: [], events: []}; }};')
        start()
        assert run('globals','get','termuxSmoke')['value'] == 'Unicode 😀'
        run('plugin','enable','portable-smoke')
        run('plugin','health','portable-smoke')
        run('plugin','disable','portable-smoke')
        host.send_signal(signal.SIGINT)
        assert host.wait(timeout=35) == 0
        assert not socket_path.exists(), 'SIGINT left socket behind'
        start()
        if run('system','info')['platform'] in ['AndroidTermux', 'AndroidOther']:
            native = root / 'PLUGINS' / 'unvalidated-native'
            native.mkdir(parents=True, exist_ok=True)
            (native / 'plugin.json').write_text(json.dumps({
                'schemaVersion':3, 'id':'unvalidated-native', 'name':'Unvalidated Native',
                'version':'1.0.0', 'runtime':'native', 'entry':'desktop.so'}))
            (native / 'desktop.so').write_bytes(b'not a validated Android library')
        # SIGKILL leaves a stale socket; only the ownership-guarded host may recover it.
        host.kill()
        host.wait(timeout=5)
        start()
        assert run('globals','get','termuxSmoke')['value'] == 'Unicode 😀'
        if run('system','info')['platform'] in ['AndroidTermux', 'AndroidOther']:
            plugins = run('plugin','list')
            assert 'Android ABI validation' in json.dumps(plugins), plugins
            refused = subprocess.run([cli,'--json','plugin','start','unvalidated-native'],
                cwd=root, env=env, capture_output=True, text=True, timeout=10)
            assert refused.returncode == 1, refused.stderr
            assert json.loads(refused.stdout)['error']['code'] == 'capability_unavailable'
        run('api','call','system.shutdown','{}','--confirm')
        assert host.wait(timeout=35) == 0
        # Deterministic local event test: stdio processes requests in order and
        # drains their events before reading the next request, with no LIVE session.
        requests = '\n'.join(json.dumps({'jsonrpc':'2.0','id':i,'method':method,'params':params})
            for i, method, params in [(1,'points.adjust',{'uniqueId':'smoke','delta':1}),
                                      (2,'system.shutdown',{})]) + '\n'
        stdio = subprocess.run([cli,'host','--stdio','--events'], input=requests,
            cwd=root, env=env, capture_output=True, text=True, timeout=40)
        assert stdio.returncode == 0, stdio.stderr
        messages = [json.loads(line) for line in stdio.stdout.splitlines()]
        assert any('points.changed' in json.dumps(msg) for msg in messages), messages
        assert all('error' not in msg for msg in messages), messages
        # A live stdin pipe must not keep the Tokio runtime alive after signal cleanup.
        idle = subprocess.Popen([cli,'host','--stdio'], cwd=root, env=env,
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        try:
            idle.stdin.write(json.dumps({'jsonrpc':'2.0','id':1,'method':'system.info','params':{}}) + '\n')
            idle.stdin.flush()
            assert select.select([idle.stdout], [], [], 10)[0], 'stdio startup deadline'
            json.loads(idle.stdout.readline())
            time.sleep(.3)
            idle.send_signal(signal.SIGTERM)
            assert idle.wait(timeout=10) == 0, 'idle stdio signal shutdown failed'
        finally:
            if idle.poll() is None:
                idle.kill()
                idle.wait(timeout=5)
            idle.stdin.close()
            idle.stdout.close()
            idle.stderr.close()
        report = run('api','verify','--sandbox')
        assert report.get('ok') is True, report
        print('PASS: standalone, IPC, same owner, JSON discovery, SQLite restart, SIGINT/SIGTERM, stale socket, CLI/stdio events, portable VM lifecycle, unsupported native errors, sandbox')
        print('SKIPPED: authenticated LIVE, external/native plugins, frontend, desktop')
    finally:
        if host is not None and host.poll() is None:
            host.terminate()
            try:
                host.wait(timeout=35)
            except subprocess.TimeoutExpired:
                host.kill()
                host.wait(timeout=5)
        log.close()
PY
