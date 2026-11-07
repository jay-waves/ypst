"""Build the standalone Typst template archives without the ypst executable."""
import argparse
import hashlib
from pathlib import Path
import tarfile
import tomllib
import zipfile


ROOT = Path(__file__).resolve().parent.parent
FILES = {
    name: ROOT / name
    for name in (
        "typst.toml",
        "lib.typ",
        "html/html.typ",
        "html/theme.css",
        "html/highlight.css",
        "html/highlight.tmTheme",
        "html/footnotes.css",
        "html/headings.css",
        "html/links.js",
        "html/footnotes.js",
        "html/headings.js",
        "html/line_navigation.js",
        "html/COMPATIBILITY.md",
    )
}
FILES["README.md"] = ROOT / "docs/template-package.md"


def package(output: Path):
    manifest = tomllib.loads((ROOT / "typst.toml").read_text())
    if manifest["package"]["version"] != "0.1.0":
        raise ValueError("The template package must remain at version 0.1.0")
    output.mkdir(parents=True, exist_ok=True)
    stem = "ypst-template-0.1.0"
    archive = output / f"{stem}.tar.xz"
    with tarfile.open(archive, "w:xz") as bundle:
        for name, source in sorted(FILES.items()):
            bundle.add(source, arcname=f"{stem}/{name}", recursive=False)
    archive = output / f"{stem}.zip"
    with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as bundle:
        for name, source in sorted(FILES.items()):
            bundle.write(source, arcname=f"{stem}/{name}")
    for extension in ("tar.xz", "zip"):
        archive = output / f"{stem}.{extension}"
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        archive.with_name(archive.name + ".sha256").write_text(
            f"{digest}  {archive.name}\n", encoding="utf-8"
        )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, help="Directory for release archives")
    package(parser.parse_args().output)
