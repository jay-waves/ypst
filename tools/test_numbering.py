"""Compile real documents to cover chapter resets and native references."""
import html
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent.parent
SOURCE = r'''#import "../lib.typ": *
#show: template
#front-matter[= Preface]
= One
#theorem([Named])[First.] <t1>
#theorem[Second.] <t2>
#lemma[Lemma.] <l1>
$ x = 1 $ <e1>
#figure(rect(width: 10pt, height: 10pt), caption: [First image.]) <f1>
#figure(table(columns: 1, [Cell]), caption: [First table.]) <b1>
#front-matter[= Interlude]
#theorem[Third.] <t3>
$ x = 2 $ <e2>
== Subsection
#theorem[Fourth.] <t4>
= Two
#theorem[Restart.] <t5>
$ x = 3 $ <e3>
#figure(rect(width: 10pt, height: 10pt), caption: [Next image.]) <f2>
#figure(table(columns: 1, [Cell]), caption: [Next table.]) <b2>
Refs: @t1; @t2; @l1; @t3; @t4; @t5; @e1; @e2; @e3; @f1; @f2; @b1; @b2.
#theorem(numbering: none)[Unnumbered.]
#theorem[Next.] <t6>
#proof[Done.]
#context {
  assert(counter(figure.where(kind: "theorem")).at(<t6>) == (2,))
  assert(counter(figure.where(kind: "lemma")).at(<l1>) == (1,))
  assert(counter(math.equation).at(<e2>) == (2,))
  assert(counter(math.equation).at(<e3>) == (1,))
  assert(counter(figure.where(kind: image)).at(<f2>) == (1,))
  assert(counter(figure.where(kind: table)).at(<b2>) == (1,))
}
'''


@unittest.skipUnless(shutil.which("typst"), "Typst compiler is required")
class NumberingTests(unittest.TestCase):
    def test_numbering_and_references_in_both_targets(self):
        with tempfile.TemporaryDirectory(prefix=".numbering-test-", dir=ROOT) as folder:
            folder = Path(folder)
            source = folder / "test.typ"
            source.write_text(SOURCE, encoding="utf-8", newline="\n")
            for target in ("html", "pdf"):
                with self.subTest(target=target):
                    output = folder / ("test." + target)
                    options = ["--features", "html"] if target == "html" else []
                    result = subprocess.run(
                        ["typst", "compile", "--root", str(ROOT), *options,
                         str(source), str(output)], capture_output=True, text=True,
                        encoding="utf-8", timeout=60,
                    )
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertNotIn("h was ignored", result.stderr)
                    self.assertGreater(output.stat().st_size, 0)
                    if target == "pdf":
                        continue
                    document = output.read_text(encoding="utf-8")
                    refs = dict(re.findall(r'<a href="#([^"]+)"[^>]*>(.*?)</a>', document))
                    expected = {
                        "t1": "Thm. 1.1", "t2": "Thm. 1.2", "l1": "Lemma 1.1",
                        "t3": "Thm. 1.3", "t4": "Thm. 1.4", "t5": "Thm. 2.1",
                        "e1": "Eq. 1.1", "e2": "Eq. 1.2", "e3": "Eq. 2.1",
                        "f1": "Fig. 1.1", "f2": "Fig. 2.1",
                        "b1": "Tab. 1.1", "b2": "Tab. 2.1",
                    }
                    for label, text in expected.items():
                        self.assertEqual(" ".join(html.unescape(refs[label]).split()), text)
                        self.assertIn('id="' + label + '"', document)
                    self.assertEqual(re.findall(r'class="equation-number">(.*?)</div>', document),
                                     ["(1.1)", "(1.2)", "(2.1)"])
                    self.assertIn('class="proof-qed"', document)
                    self.assertNotIn("<figure><div class=\"theorem-environment", document)

    def test_equation_prefix_follows_target_scope(self):
        source_text = r'''#import "../lib.typ": *
#show: template
#front-matter[
= Preface
$ p = 0 $ <preface>
Forward: @chapter.
]
= Chapter
$ a = 1 $ <chapter>
#front-matter[
= Interlude
$ b = 2 $ <interlude>
Cross: @chapter; @resumed.
]
$ c = 3 $ <resumed>
#[
#set heading(numbering: none)
$ d = 4 $ <plain>
#[
#set math.equation(numbering: none)
$ e = 5 $
]
#[
#set math.equation(numbering: "(A)")
$ f = 6 $ <custom>
]
$ g = 7 $ <plain-next>
]
$ h = 8 $ <after>
= Next chapter
$ i = 9 $ <reset>
Refs: @preface; @chapter; @interlude; @resumed; @plain; @plain-next; @after; @reset; @custom.
'''
        expected_numbers = ["(1)", "(1.1)", "(2)", "(1.3)", "(4)",
                            "(E)", "(6)", "(1.7)", "(2.1)"]
        expected_refs = {"preface": "1", "chapter": "1.1", "interlude": "2",
                         "resumed": "1.3", "plain": "4", "plain-next": "6",
                         "after": "1.7", "reset": "2.1", "custom": "E"}
        with tempfile.TemporaryDirectory(prefix=".numbering-test-", dir=ROOT) as folder:
            folder = Path(folder)
            source = folder / "test.typ"
            source.write_text(source_text, encoding="utf-8")
            for target in ("html", "pdf"):
                with self.subTest(target=target):
                    output = folder / ("test." + target)
                    options = ["--features", "html"] if target == "html" else []
                    result = subprocess.run(
                        ["typst", "compile", "--root", str(ROOT), *options,
                         str(source), str(output)], capture_output=True, text=True, timeout=60,
                    )
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertNotIn("did not converge", result.stderr)
                    if target == "html":
                        document = output.read_text(encoding="utf-8")
                        self.assertEqual(re.findall(r'class="equation-number">(.*?)</div>', document),
                                         expected_numbers)
                        refs = re.findall(r'<a href="#([^"]+)"[^>]*>(.*?)</a>', document)
                        for label, text in refs:
                            if label in expected_refs:
                                self.assertEqual(" ".join(html.unescape(text).split()),
                                                 "Eq. " + expected_refs[label])
                        self.assertTrue(set(expected_refs) <= {label for label, _ in refs})
                    elif shutil.which("pdftotext"):
                        text = subprocess.run(["pdftotext", "-layout", str(output), "-"],
                                              capture_output=True, text=True, check=True).stdout
                        self.assertEqual(re.findall(r"\([\d.A-Z]+\)", text), expected_numbers)
                        normalized = " ".join(text.split())
                        self.assertIn("Forward: Eq. 1.1.", normalized)
                        self.assertIn("Cross: Eq. 1.1; Eq. 1.3.", normalized)
                        self.assertIn("Refs: " + "; ".join("Eq. " + n for n in expected_refs.values()) + ".",
                                      normalized)
