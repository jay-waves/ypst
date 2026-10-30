# ypst-template

A Typst 0.15+ template for assignments and technical notes, with bilingual typography, light/dark themes, PDF layouts, and interactive HTML notes.

## Quick start

Install or update the local package from this repository:

```sh
python install.py
```

On Linux, use `python3 install.py`. Run it again after updating this checkout;
it refreshes the existing `0.1.0` installation, including HTML styles and scripts.
The script can be called from any working directory.

| System | Package destination |
| --- | --- |
| Windows | `%APPDATA%/typst/packages/local/ypst-template/0.1.0/` |
| Linux | `${XDG_DATA_HOME:-$HOME/.local/share}/typst/packages/local/ypst-template/0.1.0/` |
| macOS | `~/Library/Application Support/typst/packages/local/ypst-template/0.1.0/` |

Linux ignores relative `XDG_DATA_HOME` values and uses the default instead.
These are the [standard local package directories](https://github.com/typst/packages#local-packages).
If your compiler or editor overrides `--package-path` / `TYPST_PACKAGE_PATH`,
point it at the corresponding `typst/packages` directory above. Fonts are installed separately.

The package version stays at `0.1.0`. First compilation downloads `headcount:0.1.1`, `fletcher:0.5.8`, and their dependencies.

```typst
#import "@local/ypst-template:0.1.0": template, front-matter, sidenote
#set document(title: "Technical Notes")
#show: template

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
python -m pip install -r requirements.txt
python typst.py compile note.typ note.html -- --font-path ./fonts
```

Defaults are `theme: "light"` and `layout: "portrait"`:

```typst
#show: template.with(theme: "dark", layout: "landscape")
```

`--input theme=dark` also sets the PDF theme when no explicit theme is supplied. HTML automatically follows the system light/dark preference, including changes while the page is open. Theme and layout parameters affect PDF only.

## GitHub Actions

The composite action installs Typst `0.15.0`, the Python dependencies, and the
local template package. Check out the article repository first, then compile
and deploy it in your own workflow steps:

```yaml
steps:
  - uses: actions/checkout@v4
  - uses: jay-waves/ypst-template@main
  - run: typst compile article.typ article.pdf
```

Use a branch, tag, or commit SHA in the `uses` reference to select the action
revision.

## Features

- `front-matter`: unnumbered headings without a forced page break.
- `sidenote`: text or image notes; use `side-image: path("image.png")` for caller-relative images.
- `definition`, `theorem`, `lemma`, `corollary`, `proof`: environments with English titles. Each statement type has its own counter; proofs are unnumbered and end with a square. Use `#theorem([Name])[Body] <label>` for a named statement or `numbering: none` to omit its number without advancing the counter. PDF block styling options can be passed to the statement; HTML uses dedicated CSS classes. These are template helpers, not the full ctheorems API (`qedhere` and custom counter inheritance are not provided).
- Native equation, figure, and table references. Equations, figures, tables, and statement environments use chapter-prefixed numbering in PDF and HTML; counters restart at numbered top-level headings.
- `diagram`, `node`, `edge`: Fletcher helpers with theme colors; `fletcher` exposes the full module.

PDF uses native footnotes. In HTML, footnotes expand when clicked and headings
are collapsible. Inline equations use MathML; display equations use SVG and
are not selectable as math. PDF and HTML layouts may differ.

HTML documents containing Mermaid code blocks use the bundled Pele 1.6.0
browser build. This affects HTML only; PDF keeps the code block as code. Pele
supports a subset of Mermaid syntax. Its license is in `html/LICENSE-Pele.txt`.

### HTML export

Install the wrapper dependencies once, then compile a standalone HTML file:

```sh
python -m pip install -r requirements.txt
python typst.py compile note.typ                 # writes note.html
python typst.py compile note.typ output.html     # choose an output file
```

The output embeds the theme, scripts, and images. Place `#set document(...)`
before `#show: template`. Pass Typst options after `--`, for example
`-- --root . --diagnostic-format human`.

For a site with shared CSS and scripts, use:

```sh
python typst.py compile note.typ --site-root _site
```

This writes `_site/note/index.html` and shared assets under
`_site/_shared/typst/<content-hash>/`. Source paths relative to the working
directory are preserved; an explicit output selects the page's bundle directory.
Deploy the site directory together. `--assets-url /static/` replaces the relative
shared-asset URL prefix; map that URL to the generated content-hash directory.
Links are emitted as written, so use paths relative to the output page, such as
`../b/` from `a/index.html` to `b/index.html`. Links to `.typ` files are not
converted to site URLs.

For live preview, run:

```sh
python typst.py watch note.typ -- --root .
```

The wrapper emits `{"event":"ready","url":"http://127.0.0.1:3000/"}` on stdout
and reloads the page after recompilation. Watch stdout is newline-delimited JSON;
diagnostics are written to stderr.
It uses port 3000 by default, falling back to a free port if needed; use
`--port 3001` to select a port or `--port 0` to request a free one. Watch uses a
temporary site that is removed on exit. Output paths, `--site-root`,
`--assets-url`, and `--no-serve` are not supported in watch mode. Use `compile`
to produce deployable output.

During preview, literal `image("path")` calls in the main file use local URLs
for existing PNG, JPEG, WebP, and SVG files inside the working directory. One
percentage `width` or `height` argument is supported. Other images retain
Typst's native HTML output; compiled HTML embeds images. Send one JSON command
per line to the watch process's stdin, for example:

```json
{"action":"navigate","line":12}
{"action":"quit"}
```

Each command receives a JSON response on stdout: successful responses contain
`ok: true` and the action; invalid commands contain `ok: false` and an error.
Closing stdin leaves the preview running; send `quit` to stop it.

The wrapper adds source-line anchors such as `#L3` to eligible top-level
headings and paragraphs in the main file. Navigation uses the nearest preceding
anchor and expands collapsed sections. These links change when earlier lines
are edited; the `L<number>` ID namespace is reserved. Use `--no-line-anchors`
to disable instrumentation. The wrapper creates a temporary entry beside the
source, so the source directory must be writable; its filename may also affect
diagnostics or code that depends on the source filename.

## Recommended packages

| Package | Use with this template |
| --- | --- |
| [Physica](https://typst.app/universe/package/physica/) | Derivatives, vector calculus, and other scientific notation. Import as a module to avoid name collisions; see `examples/scientific.typ`. |
| [CeTZ](https://typst.app/universe/package/cetz/) | Custom vector drawings. For HTML, render the canvas through `html.frame`; see `examples/scientific.typ`. |
| [Fletcher](https://typst.app/universe/package/fletcher/) | Arrow and node diagrams. Already exposed as `diagram`, `node`, `edge`, and `fletcher`; use `html.frame` for HTML, as shown in `examples/components.typ`. |
| [headcount](https://typst.app/universe/package/headcount/) | Chapter-dependent native counters. Integrated into equations, figures, tables, and the template’s small theorem environment layer. |

The examples exercise selected package features in PDF and HTML; this is not a guarantee for every package API. Additional packages are downloaded when their examples are first compiled.

## Examples

- [typography.typ](examples/typography.typ): bilingual text, emphasis, heading levels, tables, and code.
- [components.typ](examples/components.typ): front matter, footnotes, text/image sidenotes, theorem environments, cross-references, figures, and diagrams.
- [scientific.typ](examples/scientific.typ): Physica notation and a CeTZ drawing shared between PDF and HTML.

Run from the repository root (replace `components` with another example name):

```sh
typst compile --root . --font-path ./fonts examples/components.typ components.pdf
python typst.py compile examples/components.typ components.html -- --root .
typst compile --root . --font-path ./fonts --input theme=dark --input layout=landscape examples/components.typ components-dark.pdf
```

Examples import the checkout directly. In your own documents, use `@local/ypst-template:0.1.0` after running the installer.

## Fonts and CSS

| Content | Preferred font |
| --- | --- |
| Latin prose | Georgia |
| Chinese prose | Source Han Serif |
| Headings | Georgia / Source Han Serif |
| Code | Iosevka |
| Math | New Computer Modern Math |

The installer does not install fonts. Use system fonts or `--font-path ./fonts` for PDF. `python fonts/download-georgia.py` downloads Georgia; Linux extraction requires `cabextract` or 7-Zip.

HTML uses locally installed fonts and falls back to system fonts when they are unavailable. Install the preferred fonts locally to use them; the stylesheet does not load font files.

The `[tool.ypst-template.assets]` tables in `typst.toml` record bundled asset
versions, files, sources and licenses. They are an inventory of this checkout,
not an automatic installer or updater; fonts remain separately installed. The
font version strings come from the files' embedded metadata. Python dependencies
for the optional HTML wrapper are pinned separately in `requirements.txt`.

The stylesheet targets standalone HTML; editor preview support is best-effort. See [HTML stylesheet notes](html/COMPATIBILITY.md).
