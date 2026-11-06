# Template fonts

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

## Release archive

The `0.1.0` release's `ypst-fonts.tar.xz` includes the fonts listed above,
their licenses, this guide, and `download-georgia.py`. Georgia is supplied as
its original `Georgia/georgi32.exe` installer alongside `Georgia/EULA.html`.
From the extracted archive directory, extract it without another download:

```sh
python download-georgia.py --archive Georgia/georgi32.exe --license Georgia/EULA.html --destination Georgia/extracted
```

Linux requires `cabextract` or 7-Zip; Windows uses `expand.exe`. Pass the archive
root to Typst with `--font-path`, which also searches the extracted Georgia
subdirectory. For HTML, install the fonts on the viewing device.
