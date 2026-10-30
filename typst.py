#!/usr/bin/env python3
"""Compile standalone Typst HTML and shared-resource sites."""
import argparse
import hashlib
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from urllib.parse import quote, urlsplit

from tools.html_equations import optimize_file
from tools.html_line_anchors import check_document_settings, inject

ASSET_NAMES = (
    "theme.css", "footnotes.css", "headings.css", "mermaid.css",
    "links.js", "footnotes.js", "headings.js", "line_navigation.js",
    "pele.global.js", "mermaid.js", "LICENSE-Pele.txt",
)


def diagnostic_defaults(arguments):
    if any(arg == "--diagnostic-format" or arg.startswith("--diagnostic-format=") for arg in arguments):
        return []
    return ["--diagnostic-format", "short"]


def make_compiler_source(source, site=False):
    """Keep relative imports intact and let the OS remove the temporary entry."""
    original = source.read_text(encoding="utf-8")
    check_document_settings(original)
    # Temporarily disabled; retain the injection implementation for future use.
    # body = inject(original)
    body = original
    entry = tempfile.NamedTemporaryFile(
        mode="w+", encoding="utf-8", newline="\n", suffix=".typ",
        prefix=".typst-bundle-", dir=source.parent,
    )
    try:
        entry.write('#document("index.html")[\n' + body + "\n]\n" if site else body)
        entry.flush()
    except BaseException:
        entry.close()
        raise
    return entry


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


def assets_prefix(directory, page_dir):
    relative = Path(os.path.relpath(directory, page_dir)).as_posix()
    return quote(relative.rstrip("/") + "/", safe="/.")


def main():
    parser = argparse.ArgumentParser(description=__doc__, epilog="Pass Typst options after --.")
    parser.add_argument("command", choices=("compile",))
    parser.add_argument("source", type=Path)
    parser.add_argument("output", nargs="?", type=Path)
    parser.add_argument("--site-root", type=Path, help="enable site mode and write shared assets under this directory")
    parser.add_argument("--assets-url", help="public URL prefix for shared assets (e.g. /static/typst/)")
    arguments = sys.argv[1:]
    separator = arguments.index("--") if "--" in arguments else len(arguments)
    options, extra = parser.parse_known_args(arguments[:separator])
    extra.extend(arguments[separator + 1:])
    if options.assets_url is not None and options.site_root is None:
        parser.error("--assets-url requires --site-root")
    if options.assets_url is not None:
        parsed_url = urlsplit(options.assets_url)
        if not options.assets_url or parsed_url.query or parsed_url.fragment or (parsed_url.scheme and parsed_url.scheme not in ("http", "https")) or (parsed_url.scheme and not parsed_url.netloc):
            parser.error("--assets-url must be a URL path or HTTP(S) URL without a query or fragment")
        options.assets_url = options.assets_url.rstrip("/") + "/"
    if str(options.source) == "-" or str(options.output) == "-":
        parser.error("stdin and stdout are not supported")
    source = options.source.resolve()
    root = Path.cwd().resolve()
    site_root = options.site_root.resolve() if options.site_root is not None else None
    if site_root is not None and options.output is None:
        try:
            relative_source = source.relative_to(root)
        except ValueError:
            parser.error("source must be inside the current working directory when output is omitted")
        requested = site_root / relative_source.with_suffix("")
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
    entry = None
    try:
        entry = make_compiler_source(source, site_root is not None)
        compiler_options = ["--format", "bundle" if site_root else "html", "--features", "bundle,html" if site_root else "html"]
        compiler_options.extend(diagnostic_defaults(extra))
        if site_root is not None:
            prefix = options.assets_url or assets_prefix(assets_dir, bundle_dir)
            compiler_options.extend(["--input", "ypst-template-bundle=true", "--input", f"ypst-template-assets-prefix={prefix}"])
        if not any(x == "--root" or x.startswith("--root=") for x in extra):
            compiler_options.extend(["--root", "."])
        compiler_options.extend(extra)
        result = subprocess.call(["typst", "compile", *compiler_options, entry.name, str(requested)])
        if result == 0:
            optimize_file(bundle_html)
        return result
    except (OSError, ValueError) as error:
        print(f"typst.py: {error}", file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        return 130
    finally:
        if entry is not None:
            entry.close()


if __name__ == "__main__":
    raise SystemExit(main())
