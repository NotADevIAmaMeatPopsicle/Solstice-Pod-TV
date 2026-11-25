#!/usr/bin/env python3
"""Check basic ARM64 Image/DTB sizes for the Pod's reported boot layout.

Host-only structural check; it cannot establish that an image is safe to boot.
"""
import argparse
import gzip
import struct
import sys
from pathlib import Path

LIMIT = 0x2000000  # Reported kernel-to-DTB distance in the Pod project.


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('image_gz', type=Path)
    parser.add_argument('dtb', type=Path)
    parser.add_argument('--boot-header', type=Path, help='Optional Android boot image for a nominal ramdisk-address comparison')
    args = parser.parse_args()
    with gzip.open(args.image_gz, 'rb') as stream:
        image = stream.read()
    dtb = args.dtb.read_bytes()
    if len(image) < 64 or image[56:60] != b'ARM\x64':
        raise ValueError('Not a recognized little-endian ARM64 Linux Image')
    offset, declared_size = struct.unpack_from('<QQ', image, 8)
    if not declared_size or declared_size < len(image):
        raise ValueError('Missing or inconsistent ARM64 declared image size')
    if len(dtb) < 40:
        raise ValueError('DTB too short')
    magic, total = struct.unpack_from('>II', dtb)
    if magic != 0xd00dfeed or total != len(dtb):
        raise ValueError('Invalid DTB magic or total size')
    end = offset + declared_size
    print(f'ARM64 text offset: 0x{offset:x}; declared memory size: 0x{declared_size:x}')
    print(f'Image extent: 0x{end:x}; reported DTB placement: 0x{LIMIT:x}')
    print(f'DTB size: {total} bytes')
    if end >= LIMIT:
        raise ValueError('Image reaches or exceeds the reported DTB placement')
    print(f'Nominal margin: 0x{LIMIT - end:x} bytes')
    if args.boot_header:
        with args.boot_header.open('rb') as stream:
            header = stream.read(48)
        if len(header) < 48 or header[:8] != b'ANDROID!':
            raise ValueError('Not an Android boot image header')
        _, kernel_addr, ramdisk_size, ramdisk_addr = struct.unpack_from('<4I', header, 8)
        base = kernel_addr & ~0x1fffff
        kernel_start, kernel_end = base + offset, base + end
        ramdisk_end = ramdisk_addr + ramdisk_size
        overlaps = bool(ramdisk_size and kernel_start < ramdisk_end and ramdisk_addr < kernel_end)
        print(f'Assumed 2 MiB-aligned kernel base: 0x{base:x}')
        print(f'Nominal kernel: [0x{kernel_start:x}, 0x{kernel_end:x}); header ramdisk: [0x{ramdisk_addr:x}, 0x{ramdisk_end:x})')
        print(f'Nominal overlap: {"YES" if overlaps else "NO"}')
        print('This is not a runtime layout: bootloader address overrides and relocation are unknown.')
    print('Structure only: no claim of hardware compatibility or safe boot.')


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, struct.error) as error:
        print(f'ERROR: {error}', file=sys.stderr)
        sys.exit(1)
