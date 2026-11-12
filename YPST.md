# ypst developer manual

This document owns architecture, maintenance, and delivery details. User-facing
instructions belong in [README.md](README.md); shared terms belong in
[GLOSSARY.md](GLOSSARY.md). Follow [AGENTS.md](AGENTS.md) when making changes.

## Repository layout

| Location | Responsibility |
| --- | --- |
| `src/` | Rust CLI, preprocessing, resources, and preview service |
| `lib.typ` | Public template entry point and PDF/shared template behavior |
| `html/` | HTML template, styles, browser scripts, highlighting theme, and web font |
| `fonts/` | Font assets, licenses, notices, and Georgia extraction helper |
| `examples/` | Executable Typst examples and their image assets |
| `tools/` | Packaging and focused verification scripts |
| `.github/workflows/` | Build and release automation |
| `typst.toml` | Fixed package identity and bundled-asset inventory |

The root `lib.typ` is the public template entry point, not a duplicate browser
stylesheet. Keep browser presentation in `html/`.

## Development workflow

Build with `cargo build --locked`; install a checkout with
`cargo install --path . --locked` followed by `ypst install`.
After Rust edits, run `cargo fmt`, `cargo fmt --check`, and relevant tests.
Prefer targeted tests such as `cargo test preprocess::tests`; tests that compile
documents need the Typst CLI. Check the asset inventory with
`python3 tools/test_asset_manifest.py`. Before committing, run
`git diff --check` and review the staged diff for unrelated files and secrets.
Push only when requested, using SSH key authentication.

For browser checks, reuse Playwright at
`/home/jay-waves/.local/lib/node_modules/playwright` with system Chrome via
`chromium.launch({ channel: "chrome" })`. Do not add project dependencies or
download browsers for ad hoc checks without approval.

## Preprocessing and source-line navigation

`src/preprocess.rs` parses sources with `typst-syntax`, follows literal local
include/import paths inside the compilation root, caches repeated references,
and rejects cycles. Changed dependencies use temporary sibling copies to retain
caller-relative paths; original files are not modified. Source directories must
be writable. Registry packages and dynamic paths are not recursively rewritten,
except for literal CeTZ imports, which receive an HTML adapter.

HTML line anchors are generated from the entry file's original syntax tree.
Top-level headings and paragraph starts receive sparse `L<number>` IDs; blank
paragraph breaks reset the paragraph-start state. Reusable content blocks,
raw code, formulas, and list internals are not individually anchored.
Included/imported files receive no line anchors. Existing entry-source labels
reserve their IDs. Anchor insertions and import rewrites are applied in reverse
source-offset order so rewriting does not change recorded line numbers.
PDF preprocessing does not inject anchors.

`html/line_navigation.js` resolves exact fragments first, then falls back to
the preceding source-line anchor (or the first one). It opens containing
collapsed sections and scrolls after font/page loading. The `L#` convention
has no file identity; it cannot address lines in dependencies.

## Package synchronization

The package stays at `0.1.0`. Before compilation, ypst compares the installed
`.ypst-commit` with the executable's build commit and refreshes missing or
different embedded resources while preserving installed fonts. The automatic
refresh respects `--package-path` / `TYPST_PACKAGE_PATH`; `ypst install`
uses the default local package directory. After rebuilding uncommitted template
changes, run `ypst install` explicitly because the commit ID has not changed.
Builds outside Git must supply the full source commit in `YPST_BUILD_COMMIT`.

## Template and rendering behavior


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

The stylesheet targets standalone HTML; editor preview support is best-effort. See the stylesheet maintenance section below.

## Stylesheet maintenance


`theme.css` primarily targets standalone Typst and Markdown HTML. It styles standard elements and includes local font preferences, light/dark colors, and Typst note panels.

```html
<link rel="stylesheet" href="theme.css">
```

Set `data-theme="dark"` on `<html>` or `<body>` for dark mode. Typst exports use `data-theme="auto"` and follow `prefers-color-scheme`, independently of the PDF theme.

Fonts must be installed locally; unavailable fonts fall back to system fonts. The stylesheet does not load font files. Highlighting and math-rendering resources remain the host's responsibility.

### Editor previews

Basic theme classes are recognized on a best-effort basis. Editor compatibility and application-style isolation are not guaranteed; the stylesheet uses page-level `body` rules.

- VS Code: load it through `markdown.styles`.
- Obsidian: prefer a separate, scoped snippet containing only the rules you need. Loading this entire file can affect the application layout.
- markdown-it: load it in a standalone HTML page containing the rendered output.

Typst note panels and collapsible heading styles are in `footnotes.css` and `headings.css`; load them after `theme.css` when using those features outside Typst. Basic heading typography remains in `theme.css`. CSS alone does not add interactivity. Typst HTML export embeds these styles and their scripts.

Explicit sidenote containers and panel contents are generated by `html.typ`. Native `<details>` works without scripts; JavaScript places the inline marker and adds return-focus behavior. Native footnote placement and heading grouping still use JavaScript.

Upstream follow-up (checked 2026-10-14): [nested HTML sections](https://github.com/typst/typst/issues/8121) remain an open request. The open [footnote markup proposal](https://github.com/typst/typst/pull/8845) may change reference selectors; it does not provide inline note panels. Revisit these adapters when those changes land.


## Font archive maintenance


Headings and prose use Georgia for Latin text and Source Han Serif (Songti) for Chinese.
Headings use bold faces. Code retains Monaspace Neon, with automatic fallback for Chinese;
formulas retain New Computer Modern Math. Source Han Sans is no longer distributed.

Chinese code has no explicit font assignment. Typst chooses missing glyphs from
available system fonts and `--font-path`; it does not guarantee a sans-serif face.
A suitable font must be available, and metrics and line wrapping may vary between
compilation environments. PDFs embed the selected glyphs; HTML fallback depends
on the viewing device. Source Han Serif remains explicitly selected for prose
and headings, but is not forced for code.

These files are not copied by `ypst install`. Install them yourself or use `--font-path`.
Install these fonts locally for HTML exports; unavailable fonts fall back to system fonts.

- Source Han Serif: Regular/SemiBold/Bold; LICENSE-SourceHanSerif.txt.
- Monaspace Neon 1.400: Regular/Bold OTF; LICENSE-Monaspace.txt.
- New Computer Modern Math: Regular; NOTICE-NewCMMath.txt.

Keep the unmodified font files and their licenses together when redistributing.

Georgia: original Core Fonts archive and extracted font/license files are kept together.
For Georgia, only the download script is tracked; run python fonts/download-georgia.py.
Downloaded fonts/Georgia is Git-ignored and not copied by `ypst install`.

### Release archive

The latest rolling release’s `ypst-fonts.tar.xz` includes the fonts listed above,
their licenses, the canonical README, and `download-georgia.py`. Georgia is supplied as
its original `Georgia/georgi32.exe` installer alongside `Georgia/EULA.html`.
From the extracted archive directory, extract it without another download:

```sh
python download-georgia.py --archive Georgia/georgi32.exe --license Georgia/EULA.html --destination Georgia/extracted
```

Linux requires `cabextract` or 7-Zip; Windows uses `expand.exe`. Pass the archive
root to Typst with `--font-path`, which also searches the extracted Georgia
subdirectory. For HTML, install the fonts on the viewing device.


## Automated builds

Every branch push builds the Linux x86-64 and Windows x86-64 `ypst` executables and publishes a GitHub release tagged `rolling-YYYYMMDD-<sha6>` (UTC build date and the first six hexadecimal characters of the commit SHA). The workflow also supports manual runs. The downloaded executable embeds the template assets but requires the Typst CLI to compile documents. The release also includes `ypst-fonts.tar.xz`, containing Source Han Serif, Monaspace Neon, New Computer Modern Math, their licenses, and the original Georgia Core Fonts installer and EULA. On Linux, make the downloaded file executable and rename it to `ypst` if desired.

Each successful publication becomes the latest release, then removes previous rolling releases and their tags, including the legacy `0.1.0` release. Failed uploads leave the previous release available. Reruns with the same date and commit reuse the same tag. The workflow needs repository Actions permission to write contents and permission to delete old release tags. Concurrent runs replace earlier in-progress runs. Package versions remain fixed at `0.1.0`.

## Standalone archive packaging

Run `python3 tools/package_template.py OUTPUT_DIR` to create the ZIP and
tar.xz archives and SHA-256 checksums. The package manifest must remain at
`0.1.0`. Archives copy the canonical top-level README, developer manual, and
glossary; no separate documentation source is maintained. The font archive
copies the same documents alongside its licenses and extraction helper.
The template archive contains no compiler binary or fonts. The asset inventory
in `typst.toml` describes the checkout, not the archive file list.

## Preview service protocol


```sh
ypst serve note.typ --root .
ypst serve --site-root _site --port 8765
ypst stop
```

`serve INPUT` compiles once into `<system temp>/cmrak-preview/<project>/typ` and prints only its preview URL to stdout on success. Compiler errors go to stderr, with short diagnostics mapped to the original source. It starts or reuses the native service on `127.0.0.1:8765`, shared with `cmrak`; both tools can stop that service. The browser reloads when the generated HTML changes. Pass `--fragment section` (with or without `#`) to navigate open preview pages to that URL fragment after a successful compilation via SSE. Failed compilations send no notification. `serve` hosts an existing directory in the foreground. A service hosting another directory on the same port is not reused.

This follows tmcompiler's stream convention. The server uses SSE for compilation notifications and HTTP revision polling for other file changes. Typst's human-format diagnostics are passed through and do not have temporary paths remapped.
