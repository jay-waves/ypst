"""Compact Typst equation SVGs without changing paths or coordinates."""
import re
from pathlib import Path
from xml.parsers import expat

_EQUATION = re.compile(rb'(<div class="display-equation">)(<svg\b.*?</svg>)', re.S)
_THEME = re.compile(rb'<(?:style|link)\b[^>]*\bdata-theme="([^"]+)"')


def optimize_equations(html):
    theme = _THEME.search(html)
    color = b"#f0f6fc" if theme and theme[1] == b"dark" else b"#363733"

    def equation(match):
        svg = match[2]
        # Leave previously optimized equations unchanged.
        if b'data-inherit-fill=""' in svg:
            return match[0]
        edits = []
        inherited = [color.decode()]
        parser = expat.ParserCreate()

        def start(name, attrs):
            parent_fill = inherited[-1]
            fill = attrs.get("fill", parent_fill)
            # Inline styles can override presentation attributes. Be conservative
            # beneath them, and preserve explicit resets inside colored groups.
            if "style" in attrs and "fill" in attrs["style"]:
                fill = None
            if attrs.get("fill") == color.decode() and parent_fill == color.decode() and fill is not None:
                offset = parser.CurrentByteIndex
                end = svg.index(b">", offset)
                attribute = re.search(rb'\s+fill="' + re.escape(color) + rb'"', svg[offset:end])
                if attribute:
                    edits.append((offset + attribute.start(), offset + attribute.end(), b""))
            inherited.append(fill)

        def end(name):
            inherited.pop()

        parser.StartElementHandler = start
        parser.EndElementHandler = end
        try:
            parser.Parse(svg, True)
        except expat.ExpatError:
            return match[0]
        if not edits:
            return match[0]
        for begin, end, replacement in reversed(edits):
            svg = svg[:begin] + replacement + svg[end:]
        svg = svg.replace(b"<svg", b'<svg data-inherit-fill=""', 1)
        return match[1] + svg

    return share_glyphs(_EQUATION.sub(equation, html))


_SYMBOL = re.compile(rb'<symbol\b[^>]*\bid="(g[0-9A-Fa-f]{32})"[^>]*>.*?</symbol>', re.S)
_DEFS = re.compile(rb'<defs>(.*?)</defs>', re.S)


# TODO: Revisit/remove this fallback after Typst releases native cross-frame
# SVG definition sharing; verify output and retain CSS color optimization as needed.
# https://github.com/typst/typst/issues/8830
# https://github.com/typst/typst/pull/8833
def share_glyphs(html):
    """Pool self-contained glyph symbols; keep other SVG definitions local."""
    if b'data-equation-glyphs=""' in html:
        return html
    used_ids = set(re.findall(rb'\bid="([^"]+)"', html))
    symbols = {}
    pooled = []
    serial = 0

    def equation(match):
        nonlocal serial
        svg = match[2]
        # Do not rewrite malformed or nested SVGs.
        try:
            expat.ParserCreate().Parse(svg, True)
        except expat.ExpatError:
            return match[0]
        names = {}

        def definitions(block):
            def symbol(item):
                nonlocal serial
                original = item[0]
                # Referenced resources may be scoped to this particular SVG.
                if re.search(rb'(?:href=|url\(|<style\b)', original):
                    return original
                key = original.replace(b' id="' + item[1] + b'"', b'', 1)
                if key not in symbols:
                    while True:
                        identifier = f"eqg{serial:x}".encode()
                        serial += 1
                        if identifier not in used_ids:
                            break
                    used_ids.add(identifier)
                    symbols[key] = identifier
                    pooled.append(original.replace(b' id="' + item[1] + b'"',
                                                   b' id="' + identifier + b'"', 1))
                names[item[1]] = symbols[key]
                return b''
            remaining = _SYMBOL.sub(symbol, block[1])
            return b'<defs>' + remaining + b'</defs>' if remaining.strip() else b''

        svg = _DEFS.sub(definitions, svg)
        svg = re.sub(rb'((?:xlink:)?href)="#([^"]+)"',
                     lambda ref: ref[1] + b'="#' + names.get(ref[2], ref[2]) + b'"', svg)
        return match[1] + svg

    result = _EQUATION.sub(equation, html)
    if not pooled:
        return result
    pool = (b'<svg data-equation-glyphs="" xmlns="http://www.w3.org/2000/svg" '
            b'aria-hidden="true" width="0" height="0" '
            b'style="position:absolute;overflow:hidden"><defs>' + b''.join(pooled) + b'</defs></svg>')
    body = re.search(rb'<body\b[^>]*>', result)
    offset = body.end() if body else 0
    return result[:offset] + pool + result[offset:]


def optimize_file(path):
    path = Path(path)
    original = path.read_bytes()
    optimized = optimize_equations(original)
    if optimized != original:
        # Replace the output atomically after optimization.
        import tempfile
        with tempfile.NamedTemporaryFile(dir=path.parent, prefix=".equation-", delete=False) as stream:
            temporary = Path(stream.name)
            stream.write(optimized)
        try:
            temporary.replace(path)
        finally:
            temporary.unlink(missing_ok=True)
