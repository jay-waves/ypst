#!/usr/bin/env python3
"""Synchronize this checkout into Typst's local package directory (Python 3)."""
import os
from pathlib import Path, PurePosixPath
import shutil
import sys
import argparse


def package_directory(platform=None, env=None, home=None):
    platform = platform or sys.platform
    env = os.environ if env is None else env
    home = Path.home() if home is None else Path(home)
    if platform == "win32":
        base = Path(env.get("APPDATA") or home / "AppData" / "Roaming")
    elif platform == "darwin":
        base = home / "Library" / "Application Support"
    else:
        # XDG paths must be absolute. Ignore empty/relative values instead of
        # accidentally installing beneath the current working directory.
        data_home = env.get("XDG_DATA_HOME", "")
        base = (Path(data_home) if data_home and PurePosixPath(data_home).is_absolute()
                else home / ".local" / "share")
    return base / "typst" / "packages" / "local" / "ypst-template" / "0.1.0"


def sync_package(source, target):
    source, target = Path(source).resolve(), Path(target).resolve()
    if source != target:
        target.mkdir(parents=True, exist_ok=True)
        for name in ("typst.toml", "lib.typ", "typst.py"):
            shutil.copyfile(source / name, target / name)
        shutil.copytree(source / "html", target / "html", dirs_exist_ok=True)
        (target / "tools").mkdir(exist_ok=True)
        for name in ("html_equations.py", "html_line_anchors.py"):
            shutil.copyfile(source / "tools" / name, target / "tools" / name)
        # Font installation is separate. Never copy or remove destination fonts.
        # Remove only known legacy HTML assets after the new layout is copied.
        for name in ("html.typ", "theme.css", "fonts.css"):
            legacy = target / name
            if legacy.is_file():
                legacy.unlink()
        for name in ("sidenotes.js", "fonts.css"):
            legacy = target / "html" / name
            if legacy.is_file():
                legacy.unlink()
    return target


def install():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()
    source = Path(__file__).resolve().parent
    target = sync_package(source, package_directory())
    print(f"Installed @local/ypst-template:0.1.0 to {target}")


if __name__ == "__main__":
    install()
