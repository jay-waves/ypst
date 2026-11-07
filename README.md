# ypst

ypst is a Typst 0.15+ enhancement specialized for standalone HTML and site output. It adds remote-image downloads, automatic CeTZ adaptation, and shared site assets around the native Typst compiler. Its companion template provides bilingual typography and interactive HTML notes, while retaining PDF support.

## Automated builds

Every branch push builds the Linux x86-64 and Windows x86-64 `ypst` executables and publishes a GitHub release tagged `rolling-YYYYMMDD-<sha6>` (UTC build date and the first six hexadecimal characters of the commit SHA). The workflow also supports manual runs. The downloaded executable embeds the template assets but requires the Typst CLI to compile documents. The release also includes `ypst-fonts.tar.xz`, containing Source Han Serif, Monaspace Neon, New Computer Modern Math, their licenses, and the original Georgia Core Fonts installer and EULA. On Linux, make the downloaded file executable and rename it to `ypst` if desired.

Each successful publication becomes the latest release, then removes previous rolling releases and their tags, including the legacy `0.1.0` release. Failed uploads leave the previous release available. Reruns with the same date and commit reuse the same tag. The workflow needs repository Actions permission to write contents and permission to delete old release tags. Concurrent runs replace earlier in-progress runs. Package versions remain fixed at `0.1.0`.

## Quick start

### Standalone template without ypst

The latest rolling release also provides `ypst-template-0.1.0.zip` and
`ypst-template-0.1.0.tar.xz`, each with a `.sha256` checksum. These archives
contain the template, HTML styles, browser scripts, and usage instructions,
without the ypst executable or compiler runtime. Fonts remain a separate download.

Extract the archive inside your document project and import
`"ypst-template-0.1.0/lib.typ"`, or copy its contents into the local package
directory listed below and import `@local/ypst-template:0.1.0`.
Compile HTML directly with `typst compile note.typ note.html --features html --root .`.
See [standalone template instructions](docs/template-package.md) for dependencies
and differences from ypst compilation.

### Full ypst installation

Install or update the local package from this repository:

```sh
cargo install --path . --locked
ypst install
```

Before compiling, `ypst compile` (including compilation through `ypst serve`) compares the installed `.ypst-commit` with the executable’s build commit. A missing or different commit automatically refreshes the existing `0.1.0` package from the executable’s embedded resources, including HTML styles and scripts. Existing installed fonts are preserved. `ypst --version` shows the build commit. The check uses only the commit ID; after rebuilding uncommitted template edits, run `ypst install` explicitly. Builds outside a Git checkout must set `YPST_BUILD_COMMIT` to the full source commit ID.

| System | Package destination |
| --- | --- |
| Windows | `%APPDATA%/typst/packages/local/ypst-template/0.1.0/` |
| Linux | `${XDG_DATA_HOME:-$HOME/.local/share}/typst/packages/local/ypst-template/0.1.0/` |
| macOS | `~/Library/Application Support/typst/packages/local/ypst-template/0.1.0/` |

Linux ignores relative `XDG_DATA_HOME` values and uses the default instead.
These are the [standard local package directories](https://github.com/typst/packages#local-packages).
The automatic refresh respects `--package-path` / `TYPST_PACKAGE_PATH`; `ypst install` uses the default directory above. Fonts are installed separately.

The package version stays at `0.1.0`. First compilation downloads `headcount:0.1.1` and its dependencies.

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
cargo install --path . --locked
ypst compile note.typ note.html -- --font-path ./fonts
```

Defaults are `theme: "light"`, `paper: "a4"`, `layout: "portrait"`, and `columns: auto`. Automatic columns means one column in portrait and two in landscape; an explicit positive integer overrides that choice:

```typst
#show: template.with(theme: "dark", layout: "landscape")

// A4 portrait, single column
#show: template.with(paper: "a4")

// A4 portrait, two columns
#show: template.with(paper: "a4", columns: 2)

// A4 landscape, single column
#show: template.with(paper: "a4", layout: "landscape", columns: 1)
```

Choose one of the show rules above. Two-column PDF layouts include a central divider in either orientation.

`--input theme=dark` also sets the PDF theme when no explicit theme is supplied. HTML automatically follows the system light/dark preference, including changes while the page is open. Theme and layout parameters affect PDF only.

## GitHub Actions

The composite action installs Typst `0.15.0`, downloads the latest `ypst` rolling release executable, and installs the local template package. It supports Linux and Windows x64 runners and requires GitHub CLI (`gh`), which is preinstalled on GitHub-hosted runners. Check out the article repository first, then compile and deploy it in your own workflow steps:

```yaml
steps:
  - uses: actions/checkout@v4
  - uses: jay-waves/ypst-template@main
  - run: ypst compile article.typ article.pdf
```

Use a branch, tag, or commit SHA in the `uses` reference to select the action
revision. The `ypst` binary is always downloaded from the latest rolling release, independently of the selected action revision.

## Features

- `front-matter`: unnumbered headings without a forced page break.
- `sidenote`: text or image notes; use `side-image: path("image.png")` for caller-relative images.
- `definition`, `theorem`, `lemma`, `corollary`, `proof`: environments with English titles. Each statement type has its own counter; proofs are unnumbered and end with a square. Use `#theorem([Name])[Body] <label>` for a named statement or `numbering: none` to omit its number without advancing the counter. PDF block styling options can be passed to the statement; HTML uses dedicated CSS classes. These are template helpers, not the full ctheorems API (`qedhere` and custom counter inheritance are not provided).
- Native equation, figure, and table references. Equations, figures, tables, and statement environments use chapter-prefixed numbering in PDF and HTML; counters restart at numbered top-level headings. Equations automatically omit the chapter prefix inside `#set heading(numbering: none)` scopes, including `front-matter`; references follow the target equation’s scope. Entering or leaving these scopes does not reset the equation counter. Explicit equation numbering settings, including `none`, remain unchanged.

PDF uses native footnotes. In HTML, footnotes expand when clicked and headings
are collapsible. Inline and display equations use SVG; display equations
are not selectable as math. PDF and HTML layouts may differ.

Preprocessing follows literal local `#include` and `#import` paths within `--root`, reuses repeated references, and rejects cycles. Changed files use temporary sibling copies so relative paths keep their meaning; source directories must be writable. Original files are not modified.

Code highlighting is handled entirely by Typst, including inline code and dynamically generated `raw(...)`. HTML uses `html/highlight.tmTheme` with a small light/dark CSS palette in `html/highlight.css`; keep their colors in sync. Use Typst's language names or extensions (for example, `dockerfile`, not `docker`). Unsupported languages remain plain text; optional grammars can be supplied with native `raw(syntaxes: ...)`. Custom `raw(theme: ...)` themes are supported, but colors outside the bundled palette keep their original values. ypst bundles no separate highlighter or syntax database.

### HTML export

The template works with native Typst. ypst adds automatic resource handling and
site packaging. Both require Typst 0.15+; the comparison below assumes the same
template and `#show: template`.

```sh
typst compile note.typ note.html --features html --root .  # native Typst
ypst compile note.typ note.html --root .                  # enhanced HTML
ypst compile note.typ --root . --site-root _site          # site output
```

| Behavior | Native Typst + template | ypst |
| --- | --- | --- |
| Theme and interactions | Includes styles, footnotes, collapsible headings, and sidenotes | Same template behavior |
| Equations and references | Renders equations, numbering, and references | Same template behavior; deduplicates SVG glyphs when this reduces file size |
| Code blocks | Native Typst highlighting with adaptive HTML colors | Same template behavior |
| Remote images via `imagit` | Displays a clickable URL | Downloads and renders direct calls with literal HTTP(S) URLs; download failures stop compilation |
| CeTZ drawings | Requires an explicit `cetz-frame(...)` wrapper | Automatically adapts literal `@preview/cetz:…` imports |
| Template updates | Uses the installed or directly imported template | Refreshes the local package when its recorded commit differs from the executable |
| Site resources | Arrange shared resources and output paths yourself | Packages pages with shared styles, scripts, a code font, and supported images |

For remote images, import `imagit` from the template:

```typst
#import "@local/ypst-template:0.1.0": imagit
#imagit("https://example.com/image.png", width: 80%, alt: "Example")
```

Use native `image` for local files; `image(URL)` is not downloaded by ypst.

Site output writes `_site/note.typ/index.html`; deploy the entire `_site`
directory. Links are preserved as written, so use paths relative to the output
page. Images externalized by site mode do not apply native image set/show rules;
set `width`, `height`, and `alt` directly on those calls.

Standalone HTML embeds styles, scripts, and images; its code font loads from a
CDN with local fallbacks. PDF remains available with `ypst compile note.typ note.pdf`.

## Recommended packages

| Package | Use with this template |
| --- | --- |
| [Physica](https://typst.app/universe/package/physica/) | Derivatives, vector calculus, and other scientific notation. Import as a module to avoid name collisions; see `examples/scientific.typ`. |
| [CeTZ](https://typst.app/universe/package/cetz/) | Custom vector drawings. `ypst` automatically adapts literal `@preview/cetz:...` imports for HTML, rendering canvases as SVG. Module aliases, named/wildcard imports, and local include/import dependencies are supported; PDF keeps native drawings. See `examples/scientific.typ`. |
| [headcount](https://typst.app/universe/package/headcount/) | Chapter-dependent native counters. Integrated into equations, figures, tables, and the template’s small theorem environment layer. |

CeTZ adaptation preserves the imported package's other exports and does not modify source files. Existing `html.frame` wrappers remain supported. With plain `typst compile`, wrap drawings in the template's `cetz-frame(...)` helper (or `html.frame` for HTML only). Dynamic package imports and CeTZ imports inside other registry packages are not rewritten.

The examples exercise selected package features in PDF and HTML; this is not a guarantee for every package API. Additional packages are downloaded when their examples are first compiled.

## Examples

- [typography.typ](examples/typography.typ): bilingual text, emphasis, heading levels, tables, and code.
- [components.typ](examples/components.typ): front matter, footnotes, text/image sidenotes, theorem environments, cross-references and figures.
- [scientific.typ](examples/scientific.typ): Physica notation and a CeTZ drawing shared between PDF and HTML.

Run from the repository root (replace `components` with another example name):

```sh
typst compile --root . --font-path ./fonts examples/components.typ components.pdf
ypst compile examples/components.typ components.html -- --root .
typst compile --root . --font-path ./fonts --input theme=dark --input layout=landscape examples/components.typ components-dark.pdf
```

Examples import the checkout directly. In your own documents, use `@local/ypst-template:0.1.0` after running the installer.

## Design style timeline

Entries are chronological. Exploratory ideas describe possibilities, not mandatory
rules; adopted choices describe the current template.

### 2026-11-07

- **Possible typography rule:** Sans-serif headings paired with serif body text
  can distinguish the document hierarchy. This is one possible approach, not a
  requirement for this template.
- **Adopted typography:** Headings and prose share Georgia for Latin text and
  Source Han Serif (Songti) for Chinese in both PDF and HTML. Headings use bold
  (700); size, spacing, and accent color retain the visual hierarchy.
- **Specialized content:** Code retains Monaspace Neon with automatic Chinese font fallback;
  formulas retain New Computer Modern Math.
- **Font distribution:** The font archive includes the serif and specialized
  fonts, their licenses, and the Georgia extraction helper. Source Han Sans is
  removed; Georgia remains supplied as its original installer and EULA.

## Fonts and CSS

| Content | Preferred font |
| --- | --- |
| Latin prose | Georgia |
| Chinese prose | Source Han Serif |
| Headings | Georgia / Source Han Serif |
| Code | Monaspace Neon |
| Math | New Computer Modern Math |

The installer does not install fonts. Download `ypst-fonts.tar.xz` from the latest rolling release and extract it, or use the checkout fonts; pass the resulting font directory with `--font-path` for PDF. The archive includes Georgia as the original `georgi32.exe` installer with its EULA, not as extracted font files. Review its terms before use or redistribution. To extract Georgia from the release archive without downloading it again, run `python <extracted-dir>/download-georgia.py --archive <extracted-dir>/Georgia/georgi32.exe --license <extracted-dir>/Georgia/EULA.html --destination <extracted-dir>/Georgia/extracted`. Linux extraction requires `cabextract` or 7-Zip. The release also provides `ypst-fonts.tar.xz.sha256` for integrity checking.

Standalone HTML loads Monaspace Neon for code from jsDelivr and falls back to Consolas, Menlo, or a system monospace font. Site output uses a shared local font file. Other fonts use locally installed faces and fall back to system fonts when unavailable. Code uses 13px in HTML and 6.25pt in PDF.

**Chinese code font limitation:** No Chinese font is explicitly selected for code,
and no dedicated Chinese code font is distributed. Typst resolves missing glyphs
from fonts available to the compiler, including system fonts and `--font-path`.
The result is environment-dependent: it may be serif or sans-serif, and glyph
coverage, metrics, line wrapping, and pagination can differ across machines.
A suitable installed font is required for Chinese glyphs. Generated PDFs embed
the selected glyphs, so viewing the same PDF elsewhere preserves its appearance.
HTML uses browser/system fallback on the viewing device and can look different
across devices. Headings and prose retain their explicit serif font settings.

The `[tool.ypst-template.assets]` tables in `typst.toml` record bundled asset
versions, files, sources and licenses. They are an inventory of this checkout,
not an automatic installer or updater; fonts remain separately installed. The
font version strings come from the files' embedded metadata.

The stylesheet targets standalone HTML; editor preview support is best-effort. See [HTML stylesheet notes](html/COMPATIBILITY.md).

## Native local preview

```sh
ypst serve note.typ --root .
ypst serve --site-root _site --port 8765
ypst stop
```

`serve INPUT` compiles once into `<system temp>/cmrak-preview/<project>/typ` and prints only its preview URL to stdout on success. Compiler errors go to stderr, with short diagnostics mapped to the original source. It starts or reuses the native service on `127.0.0.1:8765`, shared with `cmrak`; both tools can stop that service. The browser reloads when the generated HTML changes. Pass `--fragment section` (with or without `#`) to navigate open preview pages to that URL fragment after a successful compilation via SSE. Failed compilations send no notification. `serve` hosts an existing directory in the foreground. A service hosting another directory on the same port is not reused.

This follows tmcompiler's stream convention. The server uses SSE for compilation notifications and HTTP revision polling for other file changes. Typst's human-format diagnostics are passed through and do not have temporary paths remapped.

## One-command install or update

Always downloads and replaces the current executable from the maintained release (Linux x86_64 / Windows x64). Then runs `ypst install` to refresh the template. Typst CLI must be installed separately. The default installation requires no administrator privileges.

Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/jay-waves/ypst/main/install.sh | sh
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/jay-waves/ypst/main/install.ps1 | iex
```

Linux installs into `~/.local/bin`; Windows installs into `%LOCALAPPDATA%/Programs/ypst`. Neither script modifies PATH; add the installation directory yourself if needed. Set `INSTALL_DIR` to override the destination. Downloads use GitHub’s stable `releases/latest/download/` URLs, so rerunning installs the newest published build even though the package version stays at `0.1.0`.

To choose a Windows destination while using `iex`:

```powershell
$env:INSTALL_DIR = 'D:\Tools'
irm https://raw.githubusercontent.com/jay-waves/ypst/main/install.ps1 | iex
```
