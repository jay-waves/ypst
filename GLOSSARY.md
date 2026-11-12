# Terminology and naming conventions

Use these terms consistently in documentation, comments, and interfaces.

| Term or keyword | Agreed meaning |
| --- | --- |
| ypst | The Rust command-line enhancement around the native Typst compiler; lowercase in prose and commands. |
| Typst | The upstream language and compiler. |
| ypst-template | The companion Typst package, permanently versioned `0.1.0`. |
| template | The exported `template` show rule, or the companion package when context is clear. |
| entry file / `INPUT` | The source file passed directly to `ypst compile` or `ypst serve`. |
| dependency | A source reached through `include/import`; not the entry file. |
| compilation root / `--root` | The boundary for source and resource access, distinct from the site output directory. |
| preprocessing | Source transformations before invoking Typst; original files remain unchanged. |
| source-line anchor | A generated HTML target carrying the original entry-file line number. |
| fragment | The URL portion after `#`, naming a navigation target. `--fragment` accepts a target with or without `#`. |
| `L<number>` / `L#` | The source-line fragment convention, such as `L42`; entry-file-only, with nearest-anchor fallback. |
| named fragment | A target such as `section`, independent of the source-line convention. |
| standalone HTML | A single HTML output with embedded styles, scripts, and images; its code font may load from a CDN. |
| site output / `--site-root` | HTML pages packaged under an output directory with shared assets. |
| standalone template | Template archives usable with native Typst and without the ypst executable. |
| local package | Typst's `@local/ypst-template:0.1.0` installation. |
| rolling release | A build publication identified by date and commit; does not change the package version. |
| build commit | Source commit embedded in the executable and used for template synchronization. |
| preview service | The local HTTP service shared by ypst and cmrak; serves pages and compilation notifications. |
| cmrak | The companion tool sharing preview infrastructure; not the name of this project's developer manual. |
| `html/` | Canonical repository location for HTML templates, browser styles/scripts, and related web assets. |

Document names are fixed: `README.md` is the concise user manual,
`YPST.md` is the developer manual, and `GLOSSARY.md` owns terminology.
`AGENTS.md` contains repository workflow instructions.
