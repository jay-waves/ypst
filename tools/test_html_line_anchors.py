import unittest

from html_line_anchors import inject


class AnchorTests(unittest.TestCase):
    def test_nested_code_and_multiline_paragraph(self):
        source = ('#let x = [Nested\n\nText]\n\n= Heading\n\n'
                  'First #strong[bold]\nsecond line.\n\n'
                  '```typ\n= not a heading\n```\n\nLast paragraph.\n')
        result = inject(source)
        self.assertIn('id: "L5"', result)
        self.assertIn('id: "L7"', result)
        self.assertIn('id: "L14"', result)
        self.assertNotIn('id: "L2"', result)
        self.assertNotIn('id: "L11"', result)

    def test_rejects_invalid_syntax_and_label_collision(self):
        with self.assertRaises(ValueError):
            inject('#let x = [unfinished')
        with self.assertRaises(ValueError):
            inject('= Heading <L1>\n')


if __name__ == "__main__":
    unittest.main()
