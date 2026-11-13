#!/usr/bin/env python3
"""Prepare an isolated host for manual two-PC HTTPS/WSS acceptance."""
import argparse
import hashlib
import ipaddress
import json
import os
from pathlib import Path
import re
import ssl
import subprocess
import sys
import time
import urllib.request


def save(path, value, compact=False):
    with path.open('x') as stream:
        json.dump(value, stream, indent=None if compact else 2)
        stream.write('\n')


def environment(lab):
    env = os.environ.copy()
    env['TIKTOOLS_HOME'] = str(lab)
    env['TIKTOOLS_IPC_NAME'] = 'two-pc-' + hashlib.sha256(str(lab).encode()).hexdigest()[:20]
    for key, directory in {
        'DATA': 'data', 'PLUGINS': 'plugins', 'PLUGIN_DATA': 'plugin-data',
        'BUILTIN_PLUGINS': 'builtins', 'DEV_PLUGINS': 'dev-plugins',
        'LOG': 'logs', 'TEMP': 'temp', 'PLUGIN_STORE': 'plugin-store',
    }.items():
        path = lab / directory
        path.mkdir(exist_ok=True)
        env[f'TIKTOOLS_{key}_DIR'] = str(path)
    env['TIKTOOLS_CLOUD_ORIGIN'] = ''
    env.pop('TIKTOOLS_SESSION_COOKIE', None)
    return env


def rpc(binary, env, method, params=None):
    result = subprocess.run(
        [str(binary), '--json', 'rpc', method, json.dumps(params or {})],
        env=env, capture_output=True, text=True, timeout=20,
    )
    if result.returncode:
        raise RuntimeError(f'{method} failed: {result.stderr.strip()}')
    return json.loads(result.stdout)


def prepare(args, lab):
    if any(lab.iterdir()):
        raise ValueError('prepare requires an empty, dedicated lab directory')
    address = args.address
    try:
        parsed = ipaddress.ip_address(address)
        san = f'IP:{parsed}'
    except ValueError:
        if not re.fullmatch(r'[A-Za-z0-9](?:[A-Za-z0-9.-]*[A-Za-z0-9])?', address):
            raise ValueError('address must be a hostname or an IP, without a scheme or port')
        san = f'DNS:{address}'
    bind = ipaddress.ip_address(args.bind or address)
    if not 1 <= args.port <= 65535:
        raise ValueError('port must be between 1 and 65535')
    cert, key = lab / 'certificate.pem', lab / 'private-key.pem'
    subprocess.run([
        'openssl', 'req', '-x509', '-newkey', 'ec', '-pkeyopt',
        'ec_paramgen_curve:P-256', '-noenc', '-days', '7',
        '-subj', f'/CN={address}', '-addext', f'subjectAltName={san}',
        '-keyout', str(key), '-out', str(cert),
    ], check=True, capture_output=True)
    key.chmod(0o600)
    host = f'[{address}]' if ':' in address else address
    bind_host = f'[{bind}]' if bind.version == 6 else str(bind)
    save(lab / 'remote-config.json', {
        'enabled': True, 'bindAddress': f'{bind_host}:{args.port}',
        'advertisedOrigin': f'https://{host}:{args.port}', 'allowedOrigins': [],
        'tlsCertificate': str(cert), 'tlsPrivateKey': str(key), 'tlsProxy': False,
    })
    save(lab / 'invitation-grant.json', {'grant': {
        'capabilities': ['live.read', 'workflows.read', 'workflows.toggle',
                         'points.read', 'points.adjust', 'commands.read'],
        'resources': {'workflow': ['two-pc-lab'], 'viewer': ['two-pc-lab']},
        'maxPointsDelta': 10,
    }})
    print(f'Prepared {lab}; certificate expires in 7 days.')
    print(f'Origin: https://{host}:{args.port}')
    subprocess.run(['openssl', 'x509', '-in', str(cert), '-noout', '-fingerprint', '-sha256'], check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--directory', type=Path, required=True,
                        help='dedicated lab directory, separate from your normal TikTools home')
    parser.add_argument('--binary', type=Path,
                        default=Path(__file__).resolve().parents[1] / 'target/debug/tiktools')
    sub = parser.add_subparsers(dest='command', required=True)
    prep = sub.add_parser('prepare')
    prep.add_argument('--address', default='127.0.0.1')
    prep.add_argument('--bind', help='interface IP; defaults to address (required for DNS names)')
    prep.add_argument('--port', default=8788, type=int)
    for command in ['host', 'desktop', 'configure', 'status', 'check', 'invite', 'requests', 'stop']:
        sub.add_parser(command)
    approve = sub.add_parser('approve')
    approve.add_argument('request_id')
    args = parser.parse_args()
    os.umask(0o077)
    lab = args.directory.expanduser().resolve()
    lab.mkdir(parents=True, exist_ok=True)
    if args.command == 'prepare':
        prepare(args, lab)
        return
    config = json.loads((lab / 'remote-config.json').read_text())
    binary = args.binary.expanduser().resolve()
    env = environment(lab)
    if args.command == 'host':
        raise SystemExit(subprocess.call([str(binary), 'host', '--ipc'], env=env, cwd=lab))
    if args.command == 'desktop':
        desktop = binary.with_name('tiktools-desktop' + binary.suffix)
        raise SystemExit(subprocess.call([str(desktop)], env=env, cwd=lab))
    if args.command == 'configure':
        rpc(binary, env, 'collaboration.configure', config)
        for _ in range(30):
            status = rpc(binary, env, 'collaboration.status')
            if status['gateway']['running']:
                print(json.dumps(status, indent=2))
                return
            time.sleep(0.2)
        raise RuntimeError('gateway did not start; inspect status')
    if args.command == 'check':
        context = ssl.create_default_context(cafile=str(lab / 'certificate.pem'))
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}),
                                            urllib.request.HTTPSHandler(context=context))
        with opener.open(config['advertisedOrigin'] + '/remote/v1/info', timeout=10) as response:
            print(json.dumps(json.load(response), indent=2))
        return
    if args.command == 'invite':
        target = lab / f'invitation-{time.time_ns()}.json'
        grant = json.loads((lab / 'invitation-grant.json').read_text())
        value = rpc(binary, env, 'collaboration.invites.create', grant)
        save(target, value, compact=True)
        print(f'Private one-use invitation saved to {target}; transfer privately to PC B.')
        return
    if args.command == 'approve':
        result = rpc(binary, env, 'collaboration.requests.approve', {'id': args.request_id})
    elif args.command == 'requests':
        result = rpc(binary, env, 'collaboration.requests.list')
    elif args.command == 'stop':
        result = rpc(binary, env, 'system.shutdown')
    else:
        result = rpc(binary, env, 'collaboration.status')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        print(f'collaboration lab: {error}', file=sys.stderr)
        sys.exit(1)
