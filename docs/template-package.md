# ypst-template 0.1.0 — standalone template

This is the companion template for ypst, a Typst enhancement specialized for
HTML and site output. This archive works with Typst 0.15+ without the ypst
executable. It contains the template and its HTML styles and browser scripts.

## Use without installation

Extract the archive inside your document project, then create `note.typ`:

```typst
#import "ypst-template-0.1.0/lib.typ": template
#set document(title: "My notes")
#show: template

= Introduction
Hello, Typst!
```

Compile from the project directory:

```sh
typst compile note.typ note.html --features html --root .
typst compile note.typ note.pdf --root .
```

## Install as a local package

Copy the contents of `ypst-template-0.1.0/` into the appropriate directory:

| System | Destination |
| --- | --- |
| Windows | `%APPDATA%/typst/packages/local/ypst-template/0.1.0/` |
| Linux | `${XDG_DATA_HOME:-$HOME/.local/share}/typst/packages/local/ypst-template/0.1.0/` |
| macOS | `~/Library/Application Support/typst/packages/local/ypst-template/0.1.0/` |

Place `typst.toml` directly inside `0.1.0/`, then import with:

```typst
#import "@local/ypst-template:0.1.0": template
#show: template
```

## Included behavior and dependencies

The theme, footnotes, collapsible headings, sidenotes, equations, and numbering
work with native Typst. The first compilation may download `headcount:0.1.1`
and its dependencies from Typst Universe. Fonts are distributed separately in
`ypst-fonts.tar.xz`; use `--font-path` to make them available to Typst. Standalone
HTML loads Monaspace Neon from a CDN, with local fallback fonts.

Compared with compiling through ypst, `imagit` displays a link instead of
downloading the image, HTML code blocks use native Typst highlighting, and CeTZ
drawings need an explicit `cetz-frame(...)` wrapper. Automatic site asset
packaging and SVG glyph deduplication are not included.

The asset inventory in `typst.toml` describes the source checkout, including
fonts and build resources distributed separately; it is not an archive file list.
This archive contains only the standalone template, without the ypst binary.
The package version remains `0.1.0`; later releases may update its contents.
