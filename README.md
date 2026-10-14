# ypst-template

A Typst 0.15+ template for assignments and technical notes, with bilingual typography, light/dark themes, PDF layouts, and interactive HTML notes.

## Quick start

Install or update the local package from this repository:

```sh
python install.py
```

The package version stays at `0.1.0`. First compilation downloads `ctheorems:1.1.3`, `fletcher:0.5.8`, and their dependencies.

```typst
#import "@local/ypst-template:0.1.0": template, front-matter, sidenote
#show: template
#set document(title: "Technical Notes")

#front-matter[
  = Technical Notes
  Author, date, and introductory remarks.
]

= Introduction
#sidenote[Main text.][Additional explanation.]
A statement with a note.#footnote[Supporting details.]
```

```sh
typst compile --font-path ./fonts note.typ note.pdf
typst compile --features html --font-path ./fonts note.typ note.html
```

Defaults are `theme: "light"` and `layout: "portrait"`:

```typst
#show: template.with(theme: "dark", layout: "landscape")
```

`--input theme=dark` also sets the theme when no explicit theme is supplied. Layout affects PDF only.

## Features

- `front-matter`: unnumbered headings without a forced page break.
- `sidenote`: text or image notes; use `side-image: path("image.png")` for caller-relative images.
- `definition`, `theorem`, `lemma`, `corollary`, `proof`: environments with Chinese labels.
- Native equation, figure, and table references. PDF equations use chapter prefixes; HTML equations use continuous numbering.
- `diagram`, `node`, `edge`: Fletcher helpers with theme colors; `fletcher` exposes the full module.

PDF uses native footnotes. HTML embeds CSS and scripts: click a note marker to expand, then its return label to close and return. Headings are collapsible; equations use MathML. PDF and HTML layouts may differ.

## Fonts and CSS

| Content | Preferred font |
| --- | --- |
| Latin prose | Georgia |
| Chinese prose | Source Han Serif |
| Headings | Source Han Sans |
| Code | Iosevka |
| Math | New Computer Modern Math |

The installer does not install fonts. Use system fonts or `--font-path ./fonts` for PDF. `python fonts/download-georgia.py` downloads Georgia; Linux extraction requires `cabextract` or 7-Zip.

For portable HTML, place redistributable font files and their licenses in `ypst-fonts/` beside the exported HTML. Georgia uses the local installation; do not redistribute it. Font declarations are included in `html/theme.css`.

The stylesheet targets standalone HTML; editor preview support is best-effort. See [HTML stylesheet notes](html/COMPATIBILITY.md).
