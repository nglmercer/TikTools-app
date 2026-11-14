#!/usr/bin/env python3
"""TLS termination for the isolated Cloud acceptance fixture, on loopback only."""
import argparse
import asyncio
import json
from pathlib import Path
import ssl
from urllib.parse import urlsplit


class ReadyProbe:
    """Read only the outer WebSocket ready role, never inner TLS/RPC contents."""
    def __init__(self):
        self.buffer = bytearray()
        self.upgraded = False
        self.done = False
        self.controller = False

    def feed(self, data):
        if self.done:
            return
        self.buffer.extend(data)
        if not self.upgraded:
            end = self.buffer.find(b"\r\n\r\n")
            if end < 0:
                if len(self.buffer) > 65536:
                    raise ValueError("oversized fixture response headers")
                return
            header = bytes(self.buffer[:end])
            del self.buffer[:end + 4]
            if not header.startswith(b"HTTP/1.1 101 "):
                self.done = True
                self.buffer.clear()
                return
            self.upgraded = True
        while len(self.buffer) >= 2:
            first, second = self.buffer[:2]
            length, offset = second & 127, 2
            if second & 128:
                raise ValueError("fixture server must not mask WebSocket frames")
            if length in (126, 127):
                width = 2 if length == 126 else 8
                if len(self.buffer) < 2 + width:
                    return
                length = int.from_bytes(self.buffer[2:2 + width], "big")
                offset += width
            if length > 131072:
                raise ValueError("oversized fixture ready frame")
            if len(self.buffer) < offset + length:
                return
            payload = bytes(self.buffer[offset:offset + length])
            del self.buffer[:offset + length]
            if first == 0x81:
                value = json.loads(payload)
                if value.get("type") == "ready":
                    self.controller = value.get("role") == "controller"
                    self.done = True
                    self.buffer.clear()
                    return


async def serve(args):
    upstream = urlsplit(args.upstream)
    if upstream.scheme != "http" or upstream.hostname != "127.0.0.1" or not upstream.port:
        raise ValueError("upstream must be the loopback fixture server")
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.minimum_version = ssl.TLSVersion.TLSv1_2
    context.load_cert_chain(args.cert, args.key)
    active = set()
    controller_gate = asyncio.Event()
    controller_gate.set()

    async def connection(reader, writer):
        remote = None
        active.add(writer)
        try:
            other, remote = await asyncio.open_connection(upstream.hostname, upstream.port)
            active.add(remote)
            role = ReadyProbe()

            async def copy(source, target):
                while data := await source.read(65536):
                    target.write(data)
                    await target.drain()

            async def downstream():
                while data := await other.read(65536):
                    role.feed(data)
                    if role.controller:
                        await controller_gate.wait()
                    writer.write(data)
                    await writer.drain()

            tasks = [asyncio.create_task(copy(reader, remote)),
                     asyncio.create_task(downstream())]
            try:
                await asyncio.wait(tasks, return_when=asyncio.FIRST_COMPLETED)
            finally:
                for task in tasks:
                    task.cancel()
                await asyncio.gather(*tasks, return_exceptions=True)
        except (OSError, ssl.SSLError, ConnectionError):
            pass
        finally:
            for stream in [remote, writer]:
                if stream is not None:
                    active.discard(stream)
                    stream.close()
                    try:
                        await stream.wait_closed()
                    except (OSError, ssl.SSLError, ConnectionError):
                        pass

    server = await asyncio.start_server(connection, "127.0.0.1", args.port, ssl=context)
    port = server.sockets[0].getsockname()[1]
    print(port, flush=True)
    if not args.control:
        async with server:
            await server.serve_forever()
        return
    control = Path(args.control)
    if not control.is_absolute():
        raise ValueError("control file must be an absolute private fixture path")
    # Local fixture IPC only: no HTTP endpoint can disable the proxy. Closing
    # both listener and existing streams makes Cloud APIs and WSS unavailable.
    previous = None
    while True:
        mode = control.read_text().strip()
        online = mode in ("online", "hold-controller")
        if mode == "hold-controller":
            controller_gate.clear()
        if not online and server is not None:
            server.close()
            for stream in tuple(active):
                stream.close()
            await server.wait_closed()
            server = None
        elif online and server is None:
            server = await asyncio.start_server(connection, "127.0.0.1", port, ssl=context)
        if mode != "hold-controller":
            controller_gate.set()
        if mode != previous:
            control.with_name(control.name + ".state").write_text(mode)
            previous = mode
        await asyncio.sleep(0.05)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=0)
    parser.add_argument("--upstream", required=True)
    parser.add_argument("--cert", required=True)
    parser.add_argument("--key", required=True)
    parser.add_argument("--control")
    asyncio.run(serve(parser.parse_args()))
