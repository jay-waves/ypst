import importlib.util
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
    def test_short_diagnostics_are_default_and_overridable(self):
        self.assertEqual(wrapper.diagnostic_defaults([]), ["--diagnostic-format", "short"])
        self.assertEqual(wrapper.diagnostic_defaults(["--diagnostic-format", "human"]), [])
        self.assertEqual(wrapper.diagnostic_defaults(["--diagnostic-format=human"]), [])

    def test_site_inlines_small_scripts_and_shares_pele(self):
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
            self.assertIn('const sameDocumentFragment =', html)
            self.assertIn('source-line-anchor[data-line]', html)
            self.assertIn('pele.global.js', html)
            self.assertNotIn('src="links.js"', html)
            self.assertNotIn('src="mermaid.js"', html)
            shared = list((root / "site" / "_shared" / "typst").iterdir())[0]
            self.assertTrue((shared / "pele.global.js").is_file())
            self.assertFalse((shared / "links.js").exists())

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
