#!/usr/bin/env python3
"""Generate a placeholder .icns for QueryBox. Run from repo root: python3 dev/gen_icon.py"""

import os
import shutil
import struct
import subprocess
import zlib

R, G, B = 0x2B, 0x6C, 0xB0  # placeholder blue

def png_bytes(size):
    def chunk(tag, data):
        c = tag + data
        return struct.pack('>I', len(data)) + c + struct.pack('>I', zlib.crc32(c) & 0xFFFFFFFF)

    sig = b'\x89PNG\r\n\x1a\n'
    ihdr = chunk(b'IHDR', struct.pack('>IIBBBBB', size, size, 8, 2, 0, 0, 0))
    raw = b''.join(b'\x00' + bytes([R, G, B] * size) for _ in range(size))
    idat = chunk(b'IDAT', zlib.compress(raw))
    iend = chunk(b'IEND', b'')
    return sig + ihdr + idat + iend

SIZES = [
    ('icon_16x16.png',      16),
    ('icon_16x16@2x.png',   32),
    ('icon_32x32.png',      32),
    ('icon_32x32@2x.png',   64),
    ('icon_128x128.png',    128),
    ('icon_128x128@2x.png', 256),
    ('icon_256x256.png',    256),
    ('icon_256x256@2x.png', 512),
    ('icon_512x512.png',    512),
    ('icon_512x512@2x.png', 1024),
]

iconset = 'assets/icons/QueryBox.iconset'
os.makedirs(iconset, exist_ok=True)
os.makedirs('assets/icons', exist_ok=True)

for filename, size in SIZES:
    path = os.path.join(iconset, filename)
    with open(path, 'wb') as f:
        f.write(png_bytes(size))
    print(f'  wrote {path} ({size}x{size})')

try:
    subprocess.run(
        ['iconutil', '-c', 'icns', iconset, '-o', 'assets/icons/icon.icns'],
        check=True,
    )
finally:
    shutil.rmtree(iconset, ignore_errors=True)
print('Done: assets/icons/icon.icns')
