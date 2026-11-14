#!/usr/bin/env python3
"""Real native SDK + sibling RemakeBackend acceptance in an isolated database.

No user account, existing database or application data is used.
CLI mode stores only synthetic fixture identities in the OS keychain and logs
them out before the fixture server stops. SDK mode uses memory-only credentials.
The browser PKCE approval is exercised with temporary fixture accounts.
"""
import argparse
import json
import os
from pathlib import Path
import secrets
import shutil
import signal
import socket
import subprocess
import tempfile


def run_logged(command, cwd, log, env=None):
    with log.open("w") as output:
        child = subprocess.Popen(command, cwd=cwd, env=env, start_new_session=True,
                                 stdout=output, stderr=subprocess.STDOUT)
        try:
            code = child.wait()
        except BaseException:
            try:
                os.killpg(child.pid, signal.SIGTERM)
                child.wait(timeout=5)
            except (ProcessLookupError, subprocess.TimeoutExpired):
                try:
                    os.killpg(child.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                child.wait()
            raise
    if code:
        raise RuntimeError(f"step failed (exit {code}); inspect {log}")


def cargo_executable(log, target):
    for line in log.read_text().splitlines():
        try:
            artifact = json.loads(line)
        except ValueError:
            continue
        if artifact.get("target", {}).get("name") == target and artifact.get("executable"):
            return Path(artifact["executable"])
    raise RuntimeError(f"compiled executable {target} not found; inspect {log}")


def interrupted(_signal, _frame):
    raise KeyboardInterrupt


def main():
    signal.signal(signal.SIGTERM, interrupted)
    app = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--backend", type=Path, default=app.parent / "RemakeBackend")
    parser.add_argument("--backend-suite", action="store_true", help="also run the complete isolated v3 backend suite")
    parser.add_argument("--backend-filter", help="run one named backend fixture while diagnosing acceptance")
    parser.add_argument("--cli", type=Path, help="test an existing headless CLI instead of rebuilding the SDK test")
    parser.add_argument("--report", type=Path, help="JSON report for CLI acceptance")
    parser.add_argument("--cloud-port", type=int, default=0, help="fixed private TLS port for a release built with this Cloud origin")
    args = parser.parse_args()
    if args.cloud_port and not 1024 <= args.cloud_port <= 65535:
        parser.error("--cloud-port must be 1024..65535")
    if args.cli and (not args.cli.is_absolute() or not args.cli.is_file()):
        parser.error("--cli must be an absolute path to an existing compiled CLI")
    backend = args.backend.resolve()
    if not (backend / "crates/cloud-api/Cargo.toml").is_file():
        parser.error("--backend must identify the RemakeBackend checkout")
    lab = Path(tempfile.mkdtemp(prefix="tiktools-cloud-native-"))
    lab.chmod(0o700)
    print(f"Private acceptance logs: {lab}", flush=True)
    executable = args.cli
    if not executable:
        print("Building native acceptance executable", flush=True)
        artifacts = lab / "native-build.jsonl"
        run_logged(["cargo", "test", "--locked", "-p", "tiktools-client",
                    "--features", "cloud-peer,tiktools-core/native-tiktok,tiktools-remote/cloud-peer",
                    "--test", "peer_cloud", "--no-run", "--message-format=json"], app, artifacts)
        executable = cargo_executable(artifacts, "peer_cloud")
    print("Preparing isolated database tools", flush=True)
    tools = lab / "database-build.jsonl"
    run_logged(["cargo", "build", "--locked", "-p", "devtools", "--bin", "remake-dev", "--message-format=json"], backend, tools)
    binary = cargo_executable(tools, "remake-dev")
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
    env["TIKTOOLS_PEER_ACCEPTANCE_PROXY_PORT"] = str(args.cloud_port)
    if args.cli:
        env.update(TIKTOOLS_CLI_ACCEPTANCE_RUN_ID=secrets.token_hex(16),TIKTOOLS_CLI_ACCEPTANCE_BINARY=str(args.cli.resolve()),
                   TIKTOOLS_CLI_ACCEPTANCE_BUN=shutil.which("bun") or "bun",
                   TIKTOOLS_CLI_ACCEPTANCE_SCRIPT=str(app / "scripts/headless-teams-e2e.ts"),
                   TIKTOOLS_CLI_ACCEPTANCE_REPORT=str((args.report or lab / "headless-teams.json").resolve()))
    feature = "native-cli-acceptance" if args.cli else "native-peer-acceptance"
    selected_filter = args.backend_filter or ("native_cli_teams_end_to_end" if args.cli else "v3_tests" if args.backend_suite else "native_app_cloud_")
    selected = lab / "fixture-selection.log"
    run_logged(["cargo", "test", "--locked", "-p", "cloud-api", "--features", feature,
                selected_filter, "--", "--ignored", "--list"], backend, selected, env)
    names = [line for line in selected.read_text().splitlines() if line.endswith(": test")]
    if not names or args.cli and not any("native_cli_teams_end_to_end: test" in line for line in names):
        raise RuntimeError("backend does not contain the selected acceptance fixture; zero matching tests is not a pass")
    try:
        print("Starting private PostgreSQL cluster", flush=True)
        run_logged([str(binary), "db", "up"], backend, lab / "database-start.log", env)
        print("Running real account, invitation, approval, P2P and revocation flow", flush=True)
        run_logged([
            "cargo", "test", "--locked", "-p", "cloud-api",
            "--features", feature,
            selected_filter, "--",
            "--ignored", "--test-threads=1", "--nocapture",
        ], backend, lab / "acceptance.log", env)
        if args.cli:
            report = json.loads(Path(env["TIKTOOLS_CLI_ACCEPTANCE_REPORT"]).read_text())
            if not report.get("ok") or report.get("runId") != env["TIKTOOLS_CLI_ACCEPTANCE_RUN_ID"]:
                raise RuntimeError("CLI acceptance did not produce a passing report for this run")
        print("Native/Cloud acceptance passed", flush=True)
    finally:
        run_logged([str(binary), "db", "down"], backend, lab / "database-stop.log", env)
        if args.cli:
            shutil.rmtree(lab / "database")


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, OSError, KeyboardInterrupt) as failure:
        raise SystemExit(str(failure)) from None
