import unittest
from tools.html_equations import optimize_equations


class EquationTests(unittest.TestCase):
    def test_theme_colors_and_idempotence(self):
        for theme, color in (("light", "#363733"), ("dark", "#f0f6fc")):
            html = (f'<style data-theme="{theme}"></style><div class="display-equation">'
                    f'<svg xmlns="http://www.w3.org/2000/svg"><g><use fill="{color}"/>'
                    f'<path fill="{color}"/></g></svg></div>').encode()
            optimized = optimize_equations(html)
            self.assertNotIn(b' fill=', optimized)
            self.assertIn(b'data-inherit-fill', optimized)
            self.assertEqual(optimize_equations(optimized), optimized)

    def test_preserves_color_resets_and_other_graphics(self):
        svg = (b'<svg xmlns="http://www.w3.org/2000/svg"><g fill="red">'
               b'<path fill="#363733"/></g><g style="fill: blue">'
               b'<path fill="#363733"/></g><path fill="#363733"/></svg>')
        html = b'<div class="display-equation">' + svg + b'</div>' + svg
        optimized = optimize_equations(html)
        self.assertIn(b'<g fill="red"><path fill="#363733"/>', optimized)
        self.assertIn(b'<g style="fill: blue"><path fill="#363733"/>', optimized)
        self.assertTrue(optimized.endswith(svg))
        self.assertEqual(optimized.count(b'fill="#363733"'), 5)

    def test_leaves_malformed_svg_unchanged(self):
        html = b'<div class="display-equation"><svg><path fill="#363733"></svg>'
        self.assertEqual(optimize_equations(html), html)

    def test_shared_glyphs_short_ids_and_collisions(self):
        glyph = b'g' + b'A' * 32
        formula = (b'<div class="display-equation"><svg xmlns="http://www.w3.org/2000/svg">'
                   b'<use href="#' + glyph + b'" fill="#363733"/>'
                   b'<defs><symbol id="' + glyph + b'" overflow="visible">'
                   b'<path d="M0 0L1 1"/></symbol></defs></svg></div>')
        html = b'<body><span id="eqg0"></span>' + formula * 2 + b'</body>'
        optimized = optimize_equations(html)
        self.assertEqual(optimized.count(b'<symbol '), 1)
        self.assertEqual(optimized.count(b'href="#eqg1"'), 2)
        self.assertNotIn(glyph, optimized)
        self.assertIn(b'id="eqg0"', optimized)
        self.assertEqual(optimize_equations(optimized), optimized)

    def test_shared_glyphs_do_not_merge_distinct_paths_or_move_resources(self):
        glyph = b'g' + b'B' * 32
        def formula(path):
            return (b'<div class="display-equation"><svg xmlns="http://www.w3.org/2000/svg">'
                    b'<use href="#' + glyph + b'"/><defs><symbol id="' + glyph + b'">'
                    + path + b'</symbol></defs></svg></div>')
        optimized = optimize_equations(formula(b'<path d="M0 0"/>') + formula(b'<path d="M1 1"/>'))
        self.assertEqual(optimized.count(b'<symbol '), 2)
        self.assertIn(b'href="#eqg0"', optimized)
        self.assertIn(b'href="#eqg1"', optimized)
        resource = formula(b'<path fill="url(#paint)"/>')
        self.assertEqual(optimize_equations(resource), resource)
