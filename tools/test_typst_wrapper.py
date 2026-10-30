import importlib.util
import json
import os
from pathlib import Path
import subprocess
import socket
import sys
import tempfile
import threading
import time
import unittest
import urllib.request
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("typst_wrapper", ROOT / "typst.py")
wrapper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(wrapper)


class WrapperTests(unittest.TestCase):
    def test_site_preserves_link_destinations(self):
        with tempfile.TemporaryDirectory(dir=ROOT) as directory:
            root = Path(directory)
            source = root / "a.typ"
            source.write_text('#link("b.typ?x=1&y=2#part")[Next] #link("file.pdf")[File] '
                              '#link("https://example.com/a.typ")[External] '
                              '#link("#part")[Anchor]\n', encoding="utf-8", newline="\n")
            result = subprocess.run([sys.executable, str(ROOT / "typst.py"), "compile", str(source),
                                     "--site-root", str(root / "site")], cwd=ROOT, capture_output=True)
            self.assertEqual(result.returncode, 0, result.stderr.decode(errors="replace"))
            html = (root / "site" / root.name / "a" / "index.html").read_text(encoding="utf-8")
            self.assertIn('b.typ?x=1&amp;y=2#part', html)
            self.assertIn('file.pdf', html)
            self.assertIn('https://example.com/a.typ', html)
            self.assertIn('href="#part"', html)

    def test_preview_control_navigation_and_quit(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            page = root / "index.html"
            page.write_text("<html><body>Preview</body></html>", encoding="utf-8")
            stop = threading.Event()
            server = wrapper.serve(root, page, 0, source_root=root, stop=stop)
            url = f"http://127.0.0.1:{server.server_address[1]}"
            def post(data, origin=None):
                headers = {"Content-Type": "application/json"}
                if origin:
                    headers["Origin"] = origin
                request = urllib.request.Request(url + "/_control", data=json.dumps(data).encode(), headers=headers)
                return urllib.request.urlopen(request, timeout=2)
            try:
                html = urllib.request.urlopen(url, timeout=2).read().decode()
                self.assertIn("new EventSource('/__typst_events')", html)
                events = urllib.request.urlopen(url + "/__typst_events", timeout=3)
                self.assertEqual(events.headers.get_content_type(), "text/event-stream")
                def event():
                    line = events.readline().decode()
                    self.assertTrue(line.startswith("data: "))
                    self.assertEqual(events.readline(), b"\n")
                    return json.loads(line[6:])
                initial = event()
                self.assertEqual(initial["seq"], 0)
                with post({"action": "navigate", "line": 12}) as response:
                    self.assertEqual(response.status, 200)
                self.assertEqual(event()["hash"], "#L12")
                state = json.load(urllib.request.urlopen(url + "/_control", timeout=2))
                self.assertEqual(state, {"seq": 1, "hash": "#L12"})
                with post({"action": "navigate", "line": 12}):
                    pass
                self.assertEqual(event()["seq"], 2)
                self.assertEqual(json.load(urllib.request.urlopen(url + "/_control"))["seq"], 2)
                page.write_text("<html><body>Updated</body></html>", encoding="utf-8")
                os.utime(page, ns=(int(initial["stamp"]) + 2_000_000_000,) * 2)
                self.assertNotEqual(event()["stamp"], initial["stamp"])
                for bad in ({"action": "navigate", "line": 0}, {"action": "navigate", "line": True}, {"action": "unknown"}):
                    with self.assertRaises(urllib.error.HTTPError) as error:
                        post(bad)
                    self.assertEqual(error.exception.code, 400)
                with self.assertRaises(urllib.error.HTTPError) as error:
                    post({"action": "quit"}, origin="https://example.com")
                self.assertEqual(error.exception.code, 403)
                self.assertFalse(stop.is_set())
                with post({"action": "quit"}):
                    pass
                self.assertTrue(stop.wait(2))
                events.close()
            finally:
                server.shutdown()
                server.server_close()

    def test_document_metadata_error_removes_temporary_source(self):
        with tempfile.TemporaryDirectory(dir=ROOT) as directory:
            root = Path(directory)
            source = root / "note.typ"
            source.write_text('#import "../lib.typ": template\n#show: template\n'
                              '#set document(title: "Metadata test")\n\n= Heading\n',
                              encoding="utf-8", newline="\n")
            for site in (False, True):
                output = root / ("page" if site else "out.html")
                command = [sys.executable, str(ROOT / "typst.py"), "compile", str(source), str(output)]
                if site:
                    command.extend(["--site-root", str(root / "site")])
                result = subprocess.run(command + ["--", "--root", str(ROOT)], cwd=ROOT, capture_output=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('line 3: #set document must appear before #show: template', result.stderr.decode())
                self.assertFalse(list(root.glob('.typst-bundle-*.typ')))

    def test_compiler_error_removes_temporary_source(self):
        with tempfile.TemporaryDirectory(dir=ROOT) as directory:
            root = Path(directory)
            source = root / "note.typ"
            source.write_text('#unknown_function()\n', encoding="utf-8", newline="\n")
            result = subprocess.run([sys.executable, str(ROOT / "typst.py"), "compile",
                                     str(source), str(root / "out.html"), "--", "--root", str(ROOT)],
                                    cwd=ROOT, capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse(list(root.glob('.typst-bundle-*.typ')))

    @unittest.skipUnless(sys.platform == "win32", "Windows delete-on-close semantics")
    def test_forced_termination_removes_temporary_source(self):
        with tempfile.TemporaryDirectory(dir=ROOT) as directory:
            source = Path(directory) / "note.typ"
            source.write_text("Text.\n", encoding="utf-8", newline="\n")
            script = (
                "import sys, time; from pathlib import Path; "
                "from typst import make_compiler_source; "
                "entry, _ = make_compiler_source(Path(sys.argv[1]), False); "
                "print(entry.name, flush=True); time.sleep(60)"
            )
            process = subprocess.Popen([sys.executable, "-c", script, str(source)],
                                       cwd=ROOT, stdout=subprocess.PIPE, text=True)
            try:
                name = process.stdout.readline().strip()
                self.assertTrue(name)
                entry = Path(name)
                self.assertTrue(entry.is_file())
                process.kill()
                process.wait(timeout=10)
                self.assertFalse(entry.exists())
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait(timeout=10)
                process.stdout.close()

    def test_short_diagnostics_are_default_and_overridable(self):
        self.assertEqual(wrapper.diagnostic_defaults([]), ["--diagnostic-format", "short"])
        self.assertEqual(wrapper.diagnostic_defaults(["--diagnostic-format", "human"]), [])
        self.assertEqual(wrapper.diagnostic_defaults(["--diagnostic-format=human"]), [])

    def test_site_shares_merged_styles_scripts_and_optional_pele(self):
        with tempfile.TemporaryDirectory(dir=ROOT) as directory:
            root = Path(directory)
            source = root / "note.typ"
            source.write_text('#import "../lib.typ": template\n#show: template\n\n= Heading\n\n'
                              '```mermaid\ngraph TD\n A --> B\n```\n', encoding="utf-8", newline="\n")
            result = subprocess.run([sys.executable, str(ROOT / "typst.py"), "compile", str(source),
                                     "--site-root", str(root / "site"), "--", "--root", str(ROOT)],
                                    cwd=ROOT, capture_output=True)
            self.assertEqual(result.returncode, 0, result.stderr.decode(errors="replace"))
            html = (root / "site" / root.name / "note" / "index.html").read_text(encoding="utf-8")
            self.assertNotIn('const sameDocumentFragment =', html)
            self.assertNotIn('source-line-anchor[data-line]', html)
            self.assertIn('theme.css', html)
            self.assertIn('theme.js', html)
            self.assertIn('pele.js', html)
            self.assertNotIn('footnotes.css', html)
            self.assertNotIn('src="links.js"', html)
            shared = list((root / "site" / "_shared" / "typst").iterdir())[0]
            script = (shared / "theme.js").read_text(encoding="utf-8")
            self.assertIn('const sameDocumentFragment =', script)
            self.assertIn('source-line-anchor[data-line]', script)
            self.assertIn('window.pele.mount', (shared / "pele.js").read_text(encoding="utf-8"))
            self.assertIn('.sidenote-marker', (shared / "theme.css").read_text(encoding="utf-8"))
            self.assertEqual(wrapper.shared_assets(root / "site"), shared)
            source.write_text('#import "../lib.typ": template\n#show: template\nPlain text.\n',
                              encoding="utf-8", newline="\n")
            result = subprocess.run([sys.executable, str(ROOT / "typst.py"), "compile", str(source),
                                     "--site-root", str(root / "site"), "--", "--root", str(ROOT)],
                                    cwd=ROOT, capture_output=True)
            self.assertEqual(result.returncode, 0, result.stderr.decode(errors="replace"))
            html = (root / "site" / root.name / "note" / "index.html").read_text(encoding="utf-8")
            self.assertNotIn('pele.js', html)
            self.assertIn('theme.js', html)

    def test_native_html_embeds_resources_and_pele_only_when_needed(self):
        with tempfile.TemporaryDirectory(dir=ROOT) as directory:
            root = Path(directory)
            source, output = root / "note.typ", root / "out.html"
            prefix = '#import "../lib.typ": template\n#show: template\n\n= Heading\n\nText.\n'
            for mermaid in (False, True):
                text = prefix + ('\n```mermaid\ngraph TD\n A --> B\n```\n' if mermaid else '')
                source.write_text(text, encoding="utf-8", newline="\n")
                result = subprocess.run([sys.executable, str(ROOT / "typst.py"), "compile",
                                         str(source), str(output), "--", "--root", str(ROOT)],
                                        capture_output=True)
                self.assertEqual(result.returncode, 0, result.stderr.decode(errors="replace"))
                html = output.read_text(encoding="utf-8")
                self.assertIn('id="L4"', html)
                self.assertIn('const sameDocumentFragment =', html)
                self.assertIn('source-line-anchor[data-line]', html)
                self.assertEqual('data-lang="mermaid"' in html, mermaid)
                self.assertEqual('Copyright (c) 2026 kepano' in html, mermaid)

    def test_native_watch_updates_temporary_source(self):
        with tempfile.TemporaryDirectory(dir=ROOT) as directory:
            root = Path(directory)
            source, include = root / "note.typ", root / "part.typ"
            svg = '<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"></svg>'
            (root / "main.svg").write_text(svg, encoding="utf-8")
            (root / "nested.svg").write_text(svg, encoding="utf-8")
            source.write_text('Original text.\n\n#image("main.svg", width: 70%)\n\n#include "part.typ"\n', encoding="utf-8", newline="\n")
            include.write_text('Included text.\n\n#image("nested.svg")\n', encoding="utf-8", newline="\n")
            child = []
            original_popen = subprocess.Popen
            with socket.socket() as sock:
                sock.bind(("127.0.0.1", 0))
                port = sock.getsockname()[1]

            def start(*args, **kwargs):
                process = original_popen(*args, **kwargs, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                child.append(process)
                return process

            with patch.object(wrapper.subprocess, "Popen", side_effect=start), \
                 patch.object(sys, "argv", ["typst.py", "watch", str(source),
                                           "--interval", "0.1", "--port", str(port)]):
                thread = threading.Thread(target=wrapper.main)
                thread.start()
                try:
                    deadline = time.monotonic() + 15
                    url = f"http://127.0.0.1:{port}/"
                    def page():
                        try:
                            return urllib.request.urlopen(url, timeout=2).read().decode("utf-8")
                        except (OSError, urllib.error.HTTPError):
                            return ""
                    while time.monotonic() < deadline and "Original text" not in page():
                        time.sleep(0.1)
                    html = page()
                    self.assertIn("Original text", html)
                    self.assertIn(f'/__source/{root.name}/main.svg', html)
                    self.assertIn('width: 70%', html)
                    self.assertIn('data:image/svg+xml;base64,', html)
                    with urllib.request.urlopen(f'http://127.0.0.1:{port}/__source/{root.name}/main.svg') as response:
                        self.assertEqual(response.read().decode(), svg)
                    source.write_text('Updated text.\n\n#image("main.svg", width: 70%)\n\n#include "part.typ"\n', encoding="utf-8", newline="\n")
                    while time.monotonic() < deadline and "Updated text" not in page():
                        time.sleep(0.1)
                    self.assertIn("Updated text", page())
                    include.write_text('Changed include.\n', encoding="utf-8", newline="\n")
                    while time.monotonic() < deadline and "Changed include" not in page():
                        time.sleep(0.1)
                    self.assertIn("Changed include", page())
                    self.assertFalse((root / "note.html").exists())
                finally:
                    if child:
                        child[0].terminate()
                    thread.join(timeout=10)
                self.assertFalse(thread.is_alive())
                self.assertFalse(list(root.glob('.typst-line-anchors-*')))


if __name__ == "__main__":
    unittest.main()
