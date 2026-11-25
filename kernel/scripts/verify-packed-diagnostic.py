#!/usr/bin/env python3
"""Verify a host-only Pod boot image against its inputs and signed footer.

This does not establish hardware compatibility, safety, or bootability.
Requires cryptography. Do not publish the device's own stock boot image.
"""
import argparse
import hashlib
import struct
from pathlib import Path

from cryptography import x509
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.asymmetric import padding


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('signed', type=Path)
    p.add_argument('kernel', type=Path)
    p.add_argument('dtb', type=Path)
    p.add_argument('ramdisk', type=Path)
    p.add_argument('stock', type=Path)
    p.add_argument('--extra-cmdline', help='exact replacement extra command line, when intentionally set')
    args = p.parse_args()
    image = args.signed.read_bytes()
    stock = args.stock.read_bytes()
    if image[:8] != b'ANDROID!' or stock[:8] != b'ANDROID!':
        raise ValueError('invalid Android boot header')
    ksize, kaddr, rsize, raddr, ssize, saddr, taddr, page, dt_size, osver = struct.unpack_from('<10I', image, 8)
    old = struct.unpack_from('<10I', stock, 8)
    if page != 4096 or ssize or dt_size or (kaddr, raddr, saddr, taddr, page, osver) != (old[1], old[3], old[5], old[6], old[7], old[9]):
        raise ValueError('boot addresses, page size, or layout differ from stock')
    expected_extra = stock[608:1632]
    if args.extra_cmdline is not None:
        encoded = args.extra_cmdline.encode('ascii')
        if len(encoded) >= 1024:
            raise ValueError('extra command line is too long')
        expected_extra = (encoded + b'\0').ljust(1024, b'\0')
    if image[48:576] != stock[48:576] or image[608:1632] != expected_extra:
        raise ValueError('boot name or command line differ from stock')
    align = lambda n: (n + page - 1) // page * page
    kernel = args.kernel.read_bytes() + args.dtb.read_bytes()
    ramdisk = args.ramdisk.read_bytes()
    start_r = page + align(ksize)
    body_size = start_r + align(rsize)
    if ksize != len(kernel) or rsize != len(ramdisk) or image[page:page+ksize] != kernel or image[start_r:start_r+rsize] != ramdisk:
        raise ValueError('kernel, DTB, or ramdisk do not match inputs')
    digest = hashlib.sha1()
    for data in (kernel, ramdisk, b'', b''):
        digest.update(data)
        digest.update(struct.pack('<I', len(data)))
    if image[576:608] != digest.digest().ljust(32, b'\0'):
        raise ValueError('Android boot image ID mismatch')
    stock_footer = page + align(old[0]) + align(old[2])
    cert = image[body_size:body_size+1045]
    if cert != stock[stock_footer:stock_footer+1045]:
        raise ValueError('footer certificate differs from stock')
    off = body_size + len(cert)
    if image[off] != 0x30 or image[off+1] >= 0x80:
        raise ValueError('unexpected authenticated-attributes encoding')
    end = off + 2 + image[off+1]
    attrs = image[off:end]
    expected_body = b'\x13\x05/boot\x02\x04' + struct.pack('>I', body_size)
    if attrs != b'\x30' + bytes([len(expected_body)]) + expected_body:
        raise ValueError('unexpected signed boot target or body size')
    if image[end:end+4] != b'\x04\x82\x01\x00' or len(image) != end + 4 + 256:
        raise ValueError('unexpected signature encoding or trailing data')
    # The copied 1045-byte prefix begins with the outer Android VerifiedBoot
    # ASN.1 sequence and version INTEGER; the X.509 certificate is nested.
    if cert[:7] != b'\x30\x82\x05\x24\x02\x01\x01' or cert[7:9] != b'\x30\x82':
        raise ValueError('unexpected verified-boot certificate prefix')
    cert_length = int.from_bytes(cert[9:11], 'big') + 4
    certificate = cert[7:7+cert_length]
    x509.load_der_x509_certificate(certificate).public_key().verify(
        image[end+4:], image[:body_size] + attrs, padding.PKCS1v15(), hashes.SHA256())
    print(f'Payload, boot header, stock certificate and RSA signature verified; {len(image)} bytes.')
    print('Host checks only: not tested on hardware; never infer safety or bootability.')


if __name__ == '__main__':
    main()
