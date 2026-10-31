"""Validate requirement documents and open a local, read-only traceability viewer."""

import argparse
from functools import partial
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import webbrowser

from graph import build_graph

WEB = Path(__file__).parent / "web"


class ViewerHandler(BaseHTTPRequestHandler):
    def __init__(self, *args, graph, **kwargs):
        self.graph = graph
        super().__init__(*args, **kwargs)

    def do_GET(self):
        assets = {
            "/": ("index.html", "text/html; charset=utf-8"),
            "/app.js": ("app.js", "text/javascript; charset=utf-8"),
            "/style.css": ("style.css", "text/css; charset=utf-8"),
        }
        if self.path == "/data":
            body = json.dumps(self.graph).encode()
            content_type = "application/json; charset=utf-8"
        elif self.path in assets:
            filename, content_type = assets[self.path]
            body = (WEB / filename).read_bytes()
        else:
            self.send_error(404)
            return
        self.send_response(200)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Content-Type-Options", "nosniff")
        self.send_header(
            "Content-Security-Policy",
            "default-src 'self'; script-src 'self'; style-src 'self'; object-src 'none'; frame-ancestors 'none'",
        )
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_args):
        pass


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    selection = parser.add_mutually_exclusive_group()
    selection.add_argument("--schema", type=Path)
    selection.add_argument("--schema-version", help="Bundled version (default: v1)")
    parser.add_argument(
        "--validator", default="requirements-validate", help=argparse.SUPPRESS
    )
    parser.add_argument(
        "--port", type=int, default=0, help="Local port; default chooses a free port"
    )
    parser.add_argument(
        "--no-browser",
        action="store_true",
        help="Print the URL without opening a browser",
    )
    parser.add_argument("documents", type=Path, nargs="+")
    args = parser.parse_args()
    command = [args.validator]
    if args.schema:
        command += ["--schema", str(args.schema)]
    if args.schema_version:
        command += ["--schema-version", args.schema_version]
    # Validate the exact snapshots served, even if the originals change at startup.
    try:
        documents = [
            (str(path), json.loads(path.read_text())) for path in args.documents
        ]
        with tempfile.TemporaryDirectory(prefix="requirements-viewer-") as temporary:
            for index, (source, document) in enumerate(documents):
                snapshot = Path(temporary) / f"{index}.json"
                snapshot.write_text(json.dumps(document))
                result = subprocess.run(
                    command + [str(snapshot)], capture_output=True, text=True
                )
                if result.returncode:
                    print(
                        f"FAIL: {source}\n{result.stderr.replace(str(snapshot), source)}",
                        file=sys.stderr,
                    )
                    return 1
                print(f"PASS: {source}", flush=True)
        graph = build_graph(documents)
        handler = partial(ViewerHandler, graph=graph)
        with ThreadingHTTPServer(("127.0.0.1", args.port), handler) as server:
            url = f"http://127.0.0.1:{server.server_port}/"
            print(f"Requirements viewer: {url}\nPress Ctrl+C to stop.", flush=True)
            if not args.no_browser:
                webbrowser.open(url)
            server.serve_forever()
    except KeyboardInterrupt:
        return 0
    except (OSError, ValueError, KeyError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
