import unittest

from html_line_anchors import check_document_settings, inject


class AnchorTests(unittest.TestCase):
    def test_document_setting_after_template_reports_line(self):
        source = '#show: template\n#let title = "Test"\n#set document(title: title)\n'
        with self.assertRaisesRegex(ValueError, 'line 3:.*before #show: template'):
            check_document_settings(source)

    def test_other_show_rules_and_correct_order_are_allowed(self):
        for source in ('#set document(title: "Test")\n#show: template\n',
                       '#show heading: strong\n#set document(title: "Test")\n'):
            check_document_settings(source)

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
