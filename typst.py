#!/usr/bin/env python3
"""Compile or watch standalone Typst HTML and shared-resource sites."""
import argparse
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import hashlib
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import threading
import time
from urllib.parse import quote, urlsplit

from tools.html_equations import optimize_file
from tools.html_line_anchors import check_document_settings, inject, rewrite_main_images

ASSET_NAMES = (
    "theme.css", "footnotes.css", "headings.css", "mermaid.css",
    "links.js", "footnotes.js", "headings.js", "line_navigation.js",
    "pele.global.js", "mermaid.js", "LICENSE-Pele.txt",
)


def diagnostic_defaults(arguments):
    if any(arg == "--diagnostic-format" or arg.startswith("--diagnostic-format=") for arg in arguments):
        return []
    return ["--diagnostic-format", "short"]


PREVIEW_IMAGE_HELPER = '''#let __ypst_preview_image(url, ..args) = {
  let picture = image(..args)
  show image: it => {
    let attrs = (src: url,)
    if it.alt != none { attrs.insert("alt", it.alt) }
    if it.width != auto { attrs.insert("style", "width: " + repr(it.width)) }
    if it.height != auto { attrs.insert("style", "height: " + repr(it.height)) }
    html.elem("img", attrs: attrs)
  }
  picture
}
'''


def write_compiler_source(path, source, with_anchors, site=False, preview=None):
    check_document_settings(source)
    try:
        body = inject(source) if with_anchors else source
        if preview:
            body = rewrite_main_images(body, preview[0], preview[1], preview[2])
            body = PREVIEW_IMAGE_HELPER + body
    except ValueError as error:
        if not preview:
            raise
        # Keep watch alive while an editor temporarily contains invalid syntax.
        print(f"typst.py: preview preprocessing skipped: {error}", file=sys.stderr, flush=True)
        body = source
    with tempfile.NamedTemporaryFile(
        mode="w", encoding="utf-8", newline="\n", suffix=".tmp",
        prefix=".typst-source-", dir=path.parent, delete=False,
    ) as file:
        temporary = Path(file.name)
        file.write('#document("index.html")[\n' + body + "\n]\n" if site else body)
    try:
        temporary.replace(path)
    finally:
        temporary.unlink(missing_ok=True)


def make_compiler_source(source, with_anchors, site=False, preview=None):
    """Create a temporary entry beside the source to preserve relative paths."""
    original = source.read_text(encoding="utf-8")
    entry = tempfile.NamedTemporaryFile(
        mode="w", encoding="utf-8", newline="\n", suffix=".typ",
        prefix=".typst-bundle-", dir=source.parent, delete=False,
    )
    entry_path = Path(entry.name)
    entry.close()
    try:
        write_compiler_source(entry_path, original, with_anchors, site, preview)
    except BaseException:
        entry_path.unlink(missing_ok=True)
        raise
    return entry_path, original


def shared_assets(site_root):
    package_html = Path(__file__).resolve().parent / "html"
    sources = {name: (package_html / name).read_bytes() for name in ASSET_NAMES}
    resources = {
        "theme.css": b"\n".join(sources[name] for name in
                                  ("theme.css", "footnotes.css", "headings.css", "mermaid.css")),
        "theme.js": b"(() => {\n" + b"\n".join(sources[name] for name in
                    ("links.js", "footnotes.js", "headings.js", "line_navigation.js")) + b"\n})();\n",
        "pele.js": b"/*\n" + sources["LICENSE-Pele.txt"].replace(b"*/", b"* /") + b"\n*/\n"
                   + sources["pele.global.js"] + b"\n;\n" + sources["mermaid.js"],
        "LICENSE-Pele.txt": sources["LICENSE-Pele.txt"],
    }
    fingerprint = hashlib.sha256()
    for name, data in resources.items():
        fingerprint.update(name.encode("utf-8"))
        fingerprint.update(data)
    directory = site_root / "_shared" / "typst" / fingerprint.hexdigest()[:16]
    directory.mkdir(parents=True, exist_ok=True)
    for name, data in resources.items():
        destination = directory / name
        if destination.exists() and destination.read_bytes() == data:
            continue
        with tempfile.NamedTemporaryFile(dir=directory, prefix=".asset-", delete=False) as file:
            temporary = Path(file.name)
            file.write(data)
        try:
            temporary.replace(destination)
        finally:
            temporary.unlink(missing_ok=True)
    return directory


def assets_prefix(directory, page_dir, port=None, root=None):
    if port is not None:
        return "http://127.0.0.1:" + str(port) + "/" + quote(directory.relative_to(root).as_posix(), safe="/") + "/"
    relative = Path(os.path.relpath(directory, page_dir)).as_posix()
    return quote(relative.rstrip("/") + "/", safe="/.")


RELOAD_SCRIPT = """<script>(()=>{let stamp=null;async function poll(){try{const r=await fetch('/__typst_bundle_stamp',{cache:'no-store'});const n=await r.text();if(stamp!==null&&n!==stamp)location.reload();stamp=n}catch{}setTimeout(poll,800)}poll()})();</script>"""


def serve(root, bundle_html, port, allow_fallback=False, source_root=None):
    class Handler(SimpleHTTPRequestHandler):
        def do_GET(self):
            if self.path.split("?", 1)[0] == "/__typst_bundle_stamp":
                try:
                    stamp = str(bundle_html.stat().st_mtime_ns)
                except OSError:
                    stamp = "0"
                body = stamp.encode("ascii")
                self.send_response(200)
                self.send_header("Content-Type", "text/plain; charset=utf-8")
                self.send_header("Content-Length", str(len(body)))
                self.send_header("Cache-Control", "no-store")
                self.end_headers()
                self.wfile.write(body)
                return
            if source_root is not None and self.path.split("?", 1)[0] in ("/", "/index.html"):
                try:
                    html = bundle_html.read_text(encoding="utf-8")
                except OSError:
                    self.send_error(503, "Document is compiling")
                    return
                html = html.replace("</body>", RELOAD_SCRIPT + "</body>")
                body = html.encode("utf-8")
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.send_header("Content-Length", str(len(body)))
                self.send_header("Cache-Control", "no-store")
                self.end_headers()
                self.wfile.write(body)
                return
            super().do_GET()

        def translate_path(self, path):
            from urllib.parse import unquote
            path_only = urlsplit(path).path
            if source_root is not None and path_only.startswith("/__source/"):
                target = (source_root / unquote(path_only[len("/__source/"):])).resolve()
                try:
                    target.relative_to(source_root)
                except ValueError:
                    return str(source_root / ".ypst-invalid-path")
                return str(target)
            return super().translate_path(path)

        def log_message(self, fmt, *args):
            pass

    class LocalHTTPServer(ThreadingHTTPServer):
        allow_reuse_address = False

        def server_bind(self):
            exclusive = getattr(socket, "SO_EXCLUSIVEADDRUSE", None)
            if exclusive is not None:
                self.socket.setsockopt(socket.SOL_SOCKET, exclusive, 1)
            super().server_bind()

    try:
        server = LocalHTTPServer(("127.0.0.1", port), partial(Handler, directory=str(root)))
    except OSError:
        if not allow_fallback:
            raise
        server = LocalHTTPServer(("127.0.0.1", 0), partial(Handler, directory=str(root)))
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    return server


def main():
    parser = argparse.ArgumentParser(description=__doc__, epilog="Pass Typst options after --.")
    parser.add_argument("command", choices=("compile", "watch"))
    parser.add_argument("source", type=Path)
    parser.add_argument("output", nargs="?", type=Path)
    parser.add_argument("--no-line-anchors", action="store_true")
    parser.add_argument("--interval", type=float, default=0.5, help="source polling interval in seconds")
    parser.add_argument("--port", type=int, help="local preview server port in watch mode (default: 3000; 0 picks a free port)")
    parser.add_argument("--site-root", type=Path, help="enable site mode and write shared assets under this directory")
    parser.add_argument("--assets-url", help="public URL prefix for shared assets (e.g. /static/typst/)")
    parser.add_argument("--no-serve", action="store_true", help="disable the local HTTP preview server")
    arguments = sys.argv[1:]
    separator = arguments.index("--") if "--" in arguments else len(arguments)
    options, extra = parser.parse_known_args(arguments[:separator])
    extra.extend(arguments[separator + 1:])
    if options.interval <= 0:
        parser.error("--interval must be positive")
    if options.assets_url is not None and options.site_root is None:
        parser.error("--assets-url requires --site-root")
    if options.assets_url is not None:
        parsed_url = urlsplit(options.assets_url)
        if not options.assets_url or parsed_url.query or parsed_url.fragment or (parsed_url.scheme and parsed_url.scheme not in ("http", "https")) or (parsed_url.scheme and not parsed_url.netloc):
            parser.error("--assets-url must be a URL path or HTTP(S) URL without a query or fragment")
        options.assets_url = options.assets_url.rstrip("/") + "/"
    if options.port is not None and not 0 <= options.port < 65536:
        parser.error("--port must be between 0 and 65535")
    if str(options.source) == "-" or str(options.output) == "-":
        parser.error("stdin and stdout are not supported")
    if options.command == "watch" and (options.output is not None or options.site_root is not None or options.no_serve or options.assets_url is not None):
        parser.error("watch uses a temporary preview site; output, --site-root, --assets-url and --no-serve are compile-only")
    source = options.source.resolve()
    root = Path.cwd().resolve()
    cache = tempfile.TemporaryDirectory(prefix="ypst-preview-") if options.command == "watch" else None
    site_root = Path(cache.name) if cache else (options.site_root.resolve() if options.site_root is not None else None)
    if site_root is not None and options.output is None:
        try:
            relative_source = source.relative_to(root)
        except ValueError:
            parser.error("source must be inside the current working directory when output is omitted")
        requested = site_root / "page" if cache else site_root / relative_source.with_suffix("")
    elif options.output is None:
        requested = source.with_suffix(".html")
    else:
        requested = options.output.resolve()
    if site_root is not None and requested.suffix.lower() == ".html":
        requested = requested.with_suffix(".bundle")
    if source == requested or (requested.exists() and requested.is_file() and os.path.samefile(source, requested)):
        parser.error("output must not overwrite the source")
    if any(x in ("--format", "-f", "--deps", "--deps-format") or
           x.startswith(("--format=", "-f=", "--deps=", "--deps-format=")) for x in extra):
        parser.error("format and dependency output are managed by the wrapper")

    if options.command == "watch" and not options.no_serve and cache is None:
        try:
            requested.relative_to(root)
            if site_root is not None and options.assets_url is None:
                site_root.relative_to(root)
        except ValueError:
            parser.error("preview output and site root must be inside the current working directory")
    if site_root is not None:
        bundle_dir = requested
        if bundle_dir == site_root / "_shared" or bundle_dir in (site_root / "_shared").parents:
            parser.error("bundle output must not contain the shared assets directory")
        bundle_dir.mkdir(parents=True, exist_ok=True)
        bundle_html = bundle_dir / "index.html"
        try:
            assets_dir = shared_assets(site_root)
        except OSError as error:
            parser.error(f"could not prepare shared assets: {error}")
        if bundle_dir == assets_dir or bundle_dir in assets_dir.parents:
            parser.error("shared assets directory must be outside the bundle output directory")
    else:
        bundle_dir = None
        bundle_html = requested
        bundle_html.parent.mkdir(parents=True, exist_ok=True)
    deps_file = tempfile.NamedTemporaryFile(prefix=".typst-deps-", suffix=".json", dir=root, delete=False)
    deps_path = Path(deps_file.name)
    deps_file.close()
    server = None
    entry_path = None
    process = None
    try:
        port = None
        if options.command == "watch" and not options.no_serve:
            requested_port = 3000 if options.port is None else options.port
            server = serve(site_root, bundle_html, requested_port, allow_fallback=options.port is None, source_root=root)
            port = server.server_address[1]
            print(f"Preview: http://127.0.0.1:{port}/", flush=True)
        preview = (source.parent, root, f"http://127.0.0.1:{port}") if cache else None
        entry_path, last_text = make_compiler_source(source, not options.no_line_anchors, site_root is not None, preview)
        compiler_options = ["--format", "bundle" if site_root else "html", "--features", "bundle,html" if site_root else "html", "--deps", str(deps_path)]
        compiler_options.extend(diagnostic_defaults(extra))
        if site_root is not None:
            prefix = options.assets_url or assets_prefix(assets_dir, bundle_dir, port=port, root=site_root if port is not None else None)
            compiler_options.extend(["--input", "ypst-template-bundle=true", "--input", f"ypst-template-assets-prefix={prefix}"])
        if not any(x == "--root" or x.startswith("--root=") for x in extra):
            compiler_options.extend(["--root", "."])
        compiler_options.extend(extra)
        if options.command == "compile":
            result = subprocess.call(["typst", "compile", *compiler_options, str(entry_path), str(requested)])
            if result == 0:
                optimize_file(bundle_html)
            return result

        command = ["typst", "watch", *compiler_options, "--no-serve", str(entry_path), str(requested)]
        process = subprocess.Popen(command)
        last_compiled = None
        last_error = None
        while process.poll() is None:
            current = source.read_text(encoding="utf-8")
            if current != last_text:
                write_compiler_source(entry_path, current, not options.no_line_anchors, site_root is not None, preview)
                last_text = current
            try:
                compiled = bundle_html.stat().st_mtime_ns
                if compiled != last_compiled:
                    optimize_file(bundle_html)
                    last_compiled = bundle_html.stat().st_mtime_ns
                last_error = None
            except FileNotFoundError:
                pass
            except (OSError, ValueError) as error:
                message = str(error)
                if message != last_error:
                    print(f"typst.py: {message}", file=sys.stderr, flush=True)
                    last_error = message
            time.sleep(options.interval)
        return process.returncode
    except (OSError, ValueError) as error:
        print(f"typst.py: {error}", file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        return 130
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
            process.wait()
        if server is not None:
            server.shutdown()
            server.server_close()
        if entry_path is not None:
            entry_path.unlink(missing_ok=True)
        deps_path.unlink(missing_ok=True)
        if cache is not None:
            cache.cleanup()


if __name__ == "__main__":
    raise SystemExit(main())
