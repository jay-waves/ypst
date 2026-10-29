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

The package version stays at `0.1.0`. First compilation downloads `ctheorems:1.1.3`, `fletcher:0.5.8`, and their dependencies.

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

`--input theme=dark` also sets the theme when no explicit theme is supplied. Layout affects PDF only.

## Features

- `front-matter`: unnumbered headings without a forced page break.
- `sidenote`: text or image notes; use `side-image: path("image.png")` for caller-relative images.
- `definition`, `theorem`, `lemma`, `corollary`, `proof`: environments with Chinese labels.
- Native equation, figure, and table references. PDF equations use chapter prefixes; HTML equations use continuous numbering.
- `diagram`, `node`, `edge`: Fletcher helpers with theme colors; `fletcher` exposes the full module.

PDF uses native footnotes. HTML embeds the theme and interactive scripts by default. Click a footnote marker to expand or close it; opening another footnote closes the previous one. Headings are collapsible; equations use MathML. PDF and HTML layouts may differ.

HTML documents containing Mermaid code blocks use the bundled Pele 1.6.0
browser build. This affects HTML only; PDF keeps the code block as code. Pele
supports a subset of Mermaid syntax. Its license is in `html/LICENSE-Pele.txt`.

### HTML export

The Python wrapper adds source-line markers without changing the source file.
By default it exports a standalone HTML file containing the theme, scripts and
images:

```sh
python -m pip install -r requirements.txt
python typst.py compile note.typ
python typst.py watch examples/components.typ -- --root .
python typst.py compile note.typ --site-root _site
```

The default output for `note.typ` is `note.html`; an explicit output selects a
different HTML file. With `--site-root _site`, the wrapper uses bundle export:
`note.typ` produces `_site/note/index.html`, with source paths relative to the
current working directory preserved. CSS and the large Pele JavaScript library
are stored under `_site/_shared/typst/<content-hash>/` and reused across builds;
small interactive scripts are embedded in each HTML page. An explicit output in
site mode selects a bundle directory. Compile output references shared assets by
relative URL; deploy the site directory together. Use `--assets-url /static/`
with `--site-root` to specify a public URL prefix for the shared directory. The
public URL must be mapped to the generated content-hash directory by your site.

In watch mode, `typst.py` creates a temporary bundle in the system temporary
directory and serves it on `127.0.0.1:3000`; if that port is already in use, it
picks a free port. It prints a preview URL and reloads after recompilation. Use
`--port 3001` to select a port, or `--port 0` to request a free port. Watch
does not write an HTML file or site to the project; the temporary site is
removed when watch exits. Output paths, `--site-root`, `--assets-url`, and
`--no-serve` are not supported in watch mode. Use `compile` for a deployable
site or standalone HTML.

Typst diagnostics use `short` format by default. Pass `--diagnostic-format
human` after `--` to select the detailed format. The preview address is printed
as `Preview: http://127.0.0.1:PORT/`.

For watch previews only, simple literal `image("path")` calls in the main
file are rewritten to local HTTP URLs for existing PNG, JPEG, WebP and SVG
files inside the current working directory. One percentage `width` or `height`
argument is supported. Other arguments, dynamic paths, images from included
files, and generated images keep Typst's native HTML output. The preview server
serves project files from the current working directory; images stay in place.
In `compile` mode images remain embedded by Typst.
`--no-line-anchors` bypasses source instrumentation. Keep the installed Typst
package in sync with the checkout using `python install.py`.

The helper writes one temporary Typst entry beside the source and removes it
when the process ends, so that the source directory must be writable. Markers
such as `<span id="L3" data-line="3">`
refer to one-based lines in the original file. Markers appear at the start of
their heading or paragraph. The embedded navigation script resolves `#L3` to
the closest preceding marker (or the first marker), expanding collapsed
sections before scrolling. These links are not
persistent identifiers: editing earlier lines changes them. The `L<number>`
namespace is reserved; avoid IDs in that form elsewhere in the document.

The helper uses Tree-sitter to identify top-level headings and paragraphs that
start with text, including multiline paragraphs and inline markup. It skips
paragraphs starting with markup or code, standalone calls, and content nested
inside calls. It does not instrument included files. The language pack may
download its Typst grammar on first use.
Temporary filenames can affect diagnostics and code that depends on the source
filename. Use `--no-line-anchors` when those limitations matter.

## Recommended packages

| Package | Use with this template |
| --- | --- |
| [Physica](https://typst.app/universe/package/physica/) | Derivatives, vector calculus, and other scientific notation. Import as a module to avoid name collisions; see `examples/scientific.typ`. |
| [CeTZ](https://typst.app/universe/package/cetz/) | Custom vector drawings. For HTML, render the canvas through `html.frame`; see `examples/scientific.typ`. |
| [Fletcher](https://typst.app/universe/package/fletcher/) | Arrow and node diagrams. Already exposed as `diagram`, `node`, `edge`, and `fletcher`; use `html.frame` for HTML, as shown in `examples/components.typ`. |
| [ctheorems](https://typst.app/universe/package/ctheorems/) | Theorem numbering and environments. Already integrated through `definition`, `theorem`, `lemma`, `corollary`, and `proof`. |

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
