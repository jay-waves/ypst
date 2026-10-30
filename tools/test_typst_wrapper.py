import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

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

    def test_short_diagnostics_map_source_and_preserve_imports(self):
        with tempfile.TemporaryDirectory(dir=ROOT) as directory:
            root = Path(directory)
            source, imported = root / "note.typ", root / "part.typ"
            source.write_text('Text.\n\n#unknown_function()\n', encoding="utf-8", newline="\n")
            imported.write_text('#unknown_imported()\n', encoding="utf-8", newline="\n")
            for site in (False, True):
                for diagnostic in ([], ["--diagnostic-format=short"]):
                    output = root / ("page" if site else "out.html")
                    command = [sys.executable, str(ROOT / "typst.py"), "compile", str(source), str(output)]
                    if site:
                        command.extend(["--site-root", str(root / "site")])
                    result = subprocess.run(command + ["--", *diagnostic, "--root", str(ROOT)],
                                            cwd=ROOT, capture_output=True, text=True, encoding="utf-8")
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn(f"{source}:3:1: error: unknown variable", result.stderr)
                    self.assertNotIn(".typst-bundle-", result.stderr)
                source.write_text('#include "part.typ"\n', encoding="utf-8", newline="\n")
                result = subprocess.run(command + ["--", "--root", str(ROOT)], cwd=ROOT,
                                        capture_output=True, text=True, encoding="utf-8")
                self.assertIn("part.typ:1:1: error: unknown variable", result.stderr)
                source.write_text('Text.\n\n#unknown_function()\n', encoding="utf-8", newline="\n")

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
                "entry = make_compiler_source(Path(sys.argv[1]), False); "
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
            self.assertNotIn('id="L4"', html)
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
                self.assertNotIn('id="L4"', html)
                self.assertIn('const sameDocumentFragment =', html)
                self.assertIn('source-line-anchor[data-line]', html)
                self.assertEqual('data-lang="mermaid"' in html, mermaid)
                self.assertEqual('Copyright (c) 2026 kepano' in html, mermaid)



if __name__ == "__main__":
    unittest.main()
