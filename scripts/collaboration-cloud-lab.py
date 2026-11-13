#!/usr/bin/env python3
"""Real native SDK + sibling RemakeBackend acceptance in an isolated database.

No user account, OS keychain, existing database or application data is used.
The browser PKCE approval is exercised with temporary fixture accounts.
"""
import argparse
import json
import os
from pathlib import Path
import secrets
import socket
import subprocess
import tempfile


def run_logged(command, cwd, log, env=None):
    with log.open("w") as output:
        result = subprocess.run(command, cwd=cwd, env=env,
                                stdout=output, stderr=subprocess.STDOUT)
    if result.returncode:
        raise RuntimeError(f"step failed (exit {result.returncode}); inspect {log}")


def main():
    app = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--backend", type=Path, default=app.parent / "RemakeBackend")
    parser.add_argument("--backend-suite", action="store_true", help="also run the complete isolated v3 backend suite")
    parser.add_argument("--backend-filter", help="run one named backend fixture while diagnosing acceptance")
    args = parser.parse_args()
    backend = args.backend.resolve()
    if not (backend / "crates/cloud-api/Cargo.toml").is_file():
        parser.error("--backend must identify the RemakeBackend checkout")
    lab = Path(tempfile.mkdtemp(prefix="tiktools-cloud-native-"))
    lab.chmod(0o700)
    print(f"Private acceptance logs: {lab}", flush=True)
    print("Building native acceptance executable", flush=True)
    artifacts = lab / "native-build.jsonl"
    run_logged([
        "cargo", "test", "--locked", "-p", "tiktools-client",
        "--features", "cloud-peer,tiktools-core/native-tiktok,tiktools-remote/cloud-peer",
        "--test", "peer_cloud", "--no-run", "--message-format=json",
    ], app, artifacts)
    executable = None
    for line in artifacts.read_text().splitlines():
        try:
            artifact = json.loads(line)
        except ValueError:
            continue
        if artifact.get("target", {}).get("name") == "peer_cloud" and artifact.get("executable"):
            executable = artifact["executable"]
    if not executable:
        raise RuntimeError(f"native test executable not found; inspect {artifacts}")
    print("Preparing isolated database tools", flush=True)
    run_logged(["cargo", "build", "--locked", "-p", "devtools", "--bin", "remake-dev"],
               backend, lab / "database-build.log")
    binary = backend / "target/debug/remake-dev"
    print("Creating a private TLS authority for this test process", flush=True)
    ca = lab / "ca.pem"
    root_key = lab / "ca-key.pem"
    key = lab / "server-key.pem"
    certificate = lab / "server.pem"
    csr = lab / "server.csr"
    extensions = lab / "server-extensions.cnf"
    extensions.write_text("subjectAltName=IP:127.0.0.1\nbasicConstraints=critical,CA:FALSE\n"
                          "keyUsage=critical,digitalSignature,keyEncipherment\n"
                          "extendedKeyUsage=serverAuth\n")
    run_logged(["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes",
                "-keyout", str(root_key), "-out", str(ca), "-days", "1",
                "-subj", "/CN=TikTools Isolated Acceptance CA",
                "-addext", "basicConstraints=critical,CA:TRUE",
                "-addext", "keyUsage=critical,keyCertSign,cRLSign"],
               app, lab / "tls-ca.log")
    run_logged(["openssl", "req", "-new", "-newkey", "rsa:2048", "-nodes",
                "-keyout", str(key), "-out", str(csr), "-subj", "/CN=127.0.0.1"],
               app, lab / "tls-request.log")
    run_logged(["openssl", "x509", "-req", "-in", str(csr), "-CA", str(ca),
                "-CAkey", str(root_key), "-CAcreateserial", "-out", str(certificate),
                "-days", "1", "-extfile", str(extensions)], app, lab / "tls-sign.log")
    root_key.chmod(0o600)
    key.chmod(0o600)
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]
    env = os.environ.copy()
    database = f"postgres://postgres:{secrets.token_hex(24)}@127.0.0.1:{port}/postgres"
    env.update(DATABASE_URL=database, CLOUD_TEST_DATABASE_URL=database,
               REMAKE_DEV_DATA_DIR=str(lab / "database"), CLOUD_TEST_FAST_HASH="1",
               TIKTOOLS_PEER_ACCEPTANCE_TEST_BINARY=executable,
               TIKTOOLS_PEER_ACCEPTANCE_TLS_PROXY=str(app / "scripts/collaboration-cloud-proxy.py"),
               TIKTOOLS_PEER_ACCEPTANCE_TLS_CERT=str(certificate),
               TIKTOOLS_PEER_ACCEPTANCE_TLS_KEY=str(key), SSL_CERT_FILE=str(ca))
    try:
        print("Starting private PostgreSQL cluster", flush=True)
        run_logged([str(binary), "db", "up"], backend, lab / "database-start.log", env)
        print("Running real account, invitation, approval, P2P and revocation flow", flush=True)
        run_logged([
            "cargo", "test", "--locked", "-p", "cloud-api",
            "--features", "native-peer-acceptance",
            args.backend_filter or ("v3_tests" if args.backend_suite else "native_app_cloud_"), "--",
            "--ignored", "--test-threads=1", "--nocapture",
        ], backend, lab / "acceptance.log", env)
        print("Native/Cloud acceptance passed", flush=True)
    finally:
        run_logged([str(binary), "db", "down"], backend, lab / "database-stop.log", env)


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, OSError) as failure:
        raise SystemExit(str(failure)) from None
