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
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, output, include = root / "note.typ", root / "out.html", root / "part.typ"
            source.write_text('Original text.\n\n#include "part.typ"\n', encoding="utf-8", newline="\n")
            include.write_text('Included text.\n', encoding="utf-8", newline="\n")
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
                 patch.object(sys, "argv", ["typst.py", "watch", str(source), str(output),
                                           "--interval", "0.1", "--", "--port", str(port)]):
                thread = threading.Thread(target=wrapper.main)
                thread.start()
                try:
                    deadline = time.monotonic() + 15
                    while time.monotonic() < deadline and not output.exists():
                        time.sleep(0.1)
                    self.assertTrue(output.exists())
                    with urllib.request.urlopen(f"http://127.0.0.1:{port}/", timeout=5) as response:
                        self.assertIn("Original text", response.read().decode("utf-8"))
                    source.write_text('Updated text.\n\n#include "part.typ"\n', encoding="utf-8", newline="\n")
                    while time.monotonic() < deadline and "Updated text" not in output.read_text(encoding="utf-8"):
                        time.sleep(0.1)
                    self.assertIn("Updated text", output.read_text(encoding="utf-8"))
                    with urllib.request.urlopen(f"http://127.0.0.1:{port}/", timeout=5) as response:
                        self.assertIn("Updated text", response.read().decode("utf-8"))
                    include.write_text('Changed include.\n', encoding="utf-8", newline="\n")
                    while time.monotonic() < deadline and "Changed include" not in output.read_text(encoding="utf-8"):
                        time.sleep(0.1)
                    self.assertIn("Changed include", output.read_text(encoding="utf-8"))
                finally:
                    if child:
                        child[0].terminate()
                    thread.join(timeout=10)
                self.assertFalse(thread.is_alive())
                self.assertFalse(list(root.glob('.typst-line-anchors-*')))


if __name__ == "__main__":
    unittest.main()
