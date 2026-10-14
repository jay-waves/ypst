#!/usr/bin/env python3
"""Download Georgia Core Fonts and extract locally without running the installer.

Windows uses its built-in expand.exe. Linux requires cabextract or 7-Zip.
The destination defaults to the ignored fonts/Georgia directory in this checkout.
This script does not install or register system fonts.
"""
import argparse
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile
import urllib.request

URL = 'https://downloads.sourceforge.net/project/corefonts/the%20fonts/final/georgi32.exe'
LICENSE_URL = 'https://corefonts.sourceforge.net/eula.htm'
FONT_NAMES = ('georgia.ttf', 'georgiab.ttf', 'georgiai.ttf', 'georgiaz.ttf')


def cabinet(data):
    offset = -1
    while True:
        offset = data.find(b'MSCF', offset + 1)
        if offset < 0:
            raise ValueError('No valid CAB archive in Georgia installer')
        if len(data) - offset >= 36 and data[offset + 4:offset + 8] == b'\0' * 4:
            size = struct.unpack_from('<I', data, offset + 8)[0]
            if 36 <= size <= len(data) - offset:
                return data[offset:offset + size]


def download(destination, archive=None, license_file=None):
    destination = Path(destination).resolve()
    if archive:
        data = Path(archive).read_bytes()
    else:
        with urllib.request.urlopen(URL, timeout=60) as response:
            data = response.read(16 * 1024 * 1024 + 1)
    if len(data) > 16 * 1024 * 1024:
        raise ValueError('Georgia installer exceeds download size limit')
    cab = cabinet(data)
    if license_file:
        license_data = Path(license_file).read_bytes()
    else:
        with urllib.request.urlopen(LICENSE_URL, timeout=60) as response:
            license_data = response.read()
    if b'Reproduction and Distribution' not in license_data:
        raise ValueError('Could not retrieve the original Core Fonts EULA')
    with tempfile.TemporaryDirectory(prefix='georgia-') as temporary:
        work = Path(temporary)
        archive_file = work / 'Georgia.cab'
        archive_file.write_bytes(cab)
        extracted = work / 'extracted'
        extracted.mkdir()
        if sys.platform == 'win32':
            extractor = Path(os.environ.get('WINDIR', 'C:/Windows')) / 'System32/expand.exe'
            command = [str(extractor), '-F:*', str(archive_file), str(extracted)]
        elif shutil.which('cabextract'):
            command = [shutil.which('cabextract'), '-q', '-d', str(extracted), str(archive_file)]
        elif extractor := shutil.which('7z') or shutil.which('7zz'):
            command = [extractor, 'e', '-y', f'-o{extracted}', str(archive_file)]
        else:
            raise RuntimeError('Install cabextract or 7-Zip, then run this script again')
        subprocess.run(command, check=True, stdout=subprocess.DEVNULL)
        files = {file.name.lower(): file for file in extracted.iterdir() if file.is_file()}
        for name in FONT_NAMES:
            if name not in files or not files[name].read_bytes().startswith(b'\x00\x01\x00\x00'):
                raise ValueError(f'Installer does not contain valid {name}')
        destination.mkdir(parents=True, exist_ok=True)
        for name, file in files.items():
            shutil.copyfile(file, destination / name)
        (destination / 'georgi32.exe').write_bytes(data)
        (destination / 'EULA.html').write_bytes(license_data)
        (destination / 'SOURCE.txt').write_text(f'Original Microsoft Core Fonts Georgia installer\n{URL}\nExtracted without modification. Original installer and license/support files retained.\n', encoding='utf-8', newline='\n')
    print(f'Georgia downloaded to {destination}')
    return destination


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--destination', type=Path, default=Path(__file__).resolve().parent / 'Georgia')
    parser.add_argument('--archive', type=Path, help='Use an existing georgi32.exe instead of downloading')
    parser.add_argument('--license', type=Path, help='Use an existing Core Fonts EULA.html for offline extraction')
    args = parser.parse_args()
    download(args.destination, args.archive, args.license)
