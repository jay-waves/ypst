# ypst

ypst extends Typst 0.15+ with HTML resource handling, site output, and local
preview. Its companion template provides bilingual typography, interactive
notes, and PDF support.

## Install or update

Install the Typst CLI first, then install ypst.

Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/jay-waves/ypst/main/install.sh | sh
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/jay-waves/ypst/main/install.ps1 | iex
```

Installers download the latest rolling release and install the template.
Linux uses `~/.local/bin`; Windows uses `%LOCALAPPDATA%/Programs/ypst`.
Add that directory to PATH if needed. Set `INSTALL_DIR` (PowerShell:
`$env:INSTALL_DIR`) to choose another destination. Rerun to update.
The template package version remains `0.1.0`; `ypst --version` identifies
the executable's build commit. Fonts are installed separately.

## Write and compile

Create `note.typ`:

```typst
#import "@local/ypst-template:0.1.0": template, front-matter, sidenote
#set document(title: "Technical Notes")
#show: template

#front-matter[
  = Technical Notes
  Author and introductory remarks.
]

= Introduction
#sidenote[Main text.][Additional explanation.]
A statement with a note.#footnote[Supporting details.]
```

```sh
ypst compile note.typ note.html --root .
ypst compile note.typ note.pdf
ypst compile note.typ --root . --site-root _site
ypst serve note.typ --root .
ypst stop
```

Site output writes `_site/note.typ/index.html`; deploy the entire `_site`
directory. Write links relative to the output page. Pass native compiler options
after `--`, for example `-- --font-path ./fonts`. Source directories must be
writable for temporary preprocessing copies; original sources are not changed.

The template includes `front-matter`, `sidenote`, `definition`, `theorem`,
`lemma`, `corollary`, and `proof`. Statements, equations, figures, and tables
support chapter-based numbering and native references. Use
`#theorem([Name])[Body] <label>` for a named statement.
HTML offers collapsible headings, expandable footnotes, and automatic system
light/dark colors. PDF defaults to light A4 portrait with one column:

```typst
#show: template.with(theme: "dark", layout: "landscape", columns: 2)
```

Choose one template show rule. Theme, paper, layout, and column options affect
PDF; PDF and HTML layouts may differ.

## Images and drawings

Use native `image` for local images. For remote images:

```typst
#import "@local/ypst-template:0.1.0": imagit
#imagit("https://example.com/image.png", width: 80%, alt: "Example")
```

ypst downloads direct `imagit` calls with literal HTTP(S) URLs; failures stop
compilation. It automatically adapts literal CeTZ package imports for HTML,
including local dependencies. Dynamic imports and imports inside registry
packages are not rewritten. In site mode, set image `width`, `height`, and
`alt` directly; externalized images do not apply native image set/show rules.

## Preview and fragments

`ypst serve INPUT` compiles once, prints the preview URL, and starts or reuses
the local service on port 8765. Rerun it after edits; open pages reload.
The service is shared with cmrak. Use
`ypst serve --site-root _site --port 8765` to host an existing site.

```sh
ypst serve note.typ --fragment L42
ypst serve note.typ --fragment section
```

**Line fragments refer only to the entry file**: `L42` means original line 42
of `note.typ`, never a file reached through `include/import`. HTML anchors
are sparse, at top-level headings and paragraph starts. If no exact anchor
exists, navigation uses the nearest preceding anchor, or the first anchor.
Code, formulas, lists, and reusable content blocks are not individually anchored.
Existing entry-source `L<number>` labels take precedence. Named fragments still
work; failed compilations do not trigger navigation. PDF has no line anchors.

## Fonts

Headings and prose prefer Georgia and Source Han Serif; code uses Monaspace
Neon, and math uses New Computer Modern Math. Download `ypst-fonts.tar.xz`
from the latest release and pass its extracted directory with `--font-path`.
For HTML, install prose fonts on the viewing device. Standalone HTML loads its
code font from a CDN; site output packages a local code font.
Chinese code uses environment-dependent font fallback.

Georgia is supplied as its original installer and EULA. From the extracted
font archive, extract it with:

```sh
python download-georgia.py --archive Georgia/georgi32.exe --license Georgia/EULA.html --destination Georgia/extracted
```

Linux requires `cabextract` or 7-Zip. Keep font licenses with redistributed assets.

## Standalone template

The release archives `ypst-template-0.1.0.zip` and `.tar.xz` work without
ypst. Extract inside your project and replace the import with
`"ypst-template-0.1.0/lib.typ"`, then run:

```sh
typst compile note.typ note.html --features html --root .
typst compile note.typ note.pdf --root .
```

Alternatively, copy the archive contents into the local package directory:

| System | Directory |
| --- | --- |
| Windows | `%APPDATA%/typst/packages/local/ypst-template/0.1.0/` |
| Linux | `${XDG_DATA_HOME:-$HOME/.local/share}/typst/packages/local/ypst-template/0.1.0/` |
| macOS | `~/Library/Application Support/typst/packages/local/ypst-template/0.1.0/` |

Place `typst.toml` directly inside `0.1.0/`. Linux ignores relative
`XDG_DATA_HOME` values. First compilation may download `headcount:0.1.1`
and its dependencies. Fonts remain a separate download.

Native Typst retains the template's styling and interactions, but `imagit`
shows a URL, CeTZ needs `cetz-frame(...)`, and ypst's source-line anchors,
automatic package refresh, site packaging, and SVG deduplication are unavailable.

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

## Further reading

The repository's `examples/` directory contains `typography.typ`,
`components.typ`, and `scientific.typ`. Examples import the checkout directly;
your own documents should import the installed package.
See [YPST.md](YPST.md) for development and maintenance and
[GLOSSARY.md](GLOSSARY.md) for terminology.
