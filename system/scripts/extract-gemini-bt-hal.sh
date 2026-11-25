#!/usr/bin/env bash
set -euo pipefail
ZIP="/mnt/e/SP8096-Android15/donor-images/lineage-22.2-20261118-nightly-gemini-signed.zip"
POD_DONOR_DIR="${POD_DONOR_DIR:-$PWD/work/h850-donor}"
OUT="${POD_BT_HAL_OUT:-$POD_DONOR_DIR/gemini-qca-bt-hal}"
TMP="/tmp/gemini-bt-extract"
MNT="/mnt/gemini-bt-vendor"
mkdir -p "$OUT" "$TMP" "$MNT"

cleanup() {
  if mountpoint -q "$MNT"; then umount "$MNT" || true; fi
  rm -rf "$TMP"
}
trap cleanup EXIT

PART="vendor"
echo "Extracting $PART from gemini ZIP..."
unzip -o -q "$ZIP" "$PART.new.dat.br" "$PART.transfer.list" -d "$TMP"
brotli -d "$TMP/$PART.new.dat.br" -o "$TMP/$PART.new.dat"

python3 - "$TMP/$PART.transfer.list" "$TMP/$PART.new.dat" "$TMP/$PART.img" <<'PYEOF'
import sys
tl_path, dat_path, out_path = sys.argv[1], sys.argv[2], sys.argv[3]
BLOCK_SIZE = 4096
with open(tl_path) as f:
    lines = [l.strip() for l in f if l.strip()]
version = int(lines[0])
idx = 2 if version == 1 else 4
commands = []
max_block = 0
for line in lines[idx:]:
    parts = line.split(' ')
    cmd = parts[0]
    if cmd in ('new', 'erase', 'zero'):
        nums = [int(x) for x in parts[1].split(',')]
        cnt = nums[0]
        ranges = [(nums[i], nums[i+1]) for i in range(1, cnt + 1, 2)]
        for s, e in ranges:
            if e > max_block:
                max_block = e
        commands.append((cmd, ranges))
with open(out_path, 'wb') as out, open(dat_path, 'rb') as dat:
    out.truncate(max_block * BLOCK_SIZE)
    for cmd, ranges in commands:
        if cmd == 'new':
            for s, e in ranges:
                out.seek(s * BLOCK_SIZE)
                n = (e - s) * BLOCK_SIZE
                while n > 0:
                    chunk = dat.read(min(n, 4 * 1024 * 1024))
                    if not chunk:
                        break
                    out.write(chunk)
                    n -= len(chunk)
PYEOF

mount -o loop,ro "$TMP/$PART.img" "$MNT"
echo "=== Top-level of $MNT ==="
ls -la "$MNT"
VDIR="$MNT"
if [ -d "$MNT/system/vendor" ]; then
  VDIR="$MNT/system/vendor"
elif [ -d "$MNT/vendor" ]; then
  VDIR="$MNT/vendor"
fi
echo "VDIR=$VDIR"
ls -la "$VDIR" || true

echo "=== All bluetooth/qti/wcnss/hci files under $MNT ==="
find "$MNT" \( -iname "*bluetooth*" -o -iname "*libbt*" -o -iname "*wcnss_filter*" -o -iname "*btnv*" -o -iname "*hci_qcomm*" \) 2>/dev/null | sort || true

echo "=== Gemini BT build.prop ==="
grep -iE "bluetooth|bt\.|qcom\.bt|ro\.bt|soc\.type" "$MNT/system/build.prop" "$VDIR/build.prop" 2>/dev/null || true

echo "=== Gemini BT init & vintf ==="
grep -rn -i "bluetooth\|wcnss_filter\|hci_qcomm" "$VDIR/etc/init/" "$MNT/system/etc/init/" 2>/dev/null | head -n 40 || true
grep -n -C 8 "bluetooth" "$VDIR/etc/vintf/manifest.xml" "$MNT/system/etc/vintf/manifest.xml" 2>/dev/null || true

echo "=== Copying Gemini BT HAL files to $OUT ==="
mkdir -p "$OUT/bin/hw" "$OUT/lib64/hw" "$OUT/etc/init"
for root in "$VDIR" "$MNT/system"; do
  for f in "$root/bin/hw/"*bluetooth* "$root/bin/wcnss_filter" "$root/bin/btnvtool" "$root/bin/hci_qcomm_init"; do
    [ -e "$f" ] && cp -af "$f" "$OUT/bin/" || true
  done
  for f in "$root/bin/hw/"*bluetooth*; do
    [ -e "$f" ] && cp -af "$f" "$OUT/bin/hw/" || true
  done
  for f in \
    "$root/lib64/hw/"*bluetooth* \
    "$root/lib64/libbt-"* \
    "$root/lib64/libbtnv.so" \
    "$root/lib64/libbase_shim.so" \
    "$root/lib64/libdiag.so" \
    "$root/lib64/libhidlbase-v32.so" \
    "$root/lib64/vendor.qti.hardware.fm@1.0_vendor.so" \
    "$root/lib64/com.qualcomm.qti.ant@1.0_vendor.so" \
    "$root/lib64/"*btconfigstore* \
    "$root/lib64/"*bluetooth_audio*; do
    [ -e "$f" ] && cp -af "$f" "$OUT/lib64/" || true
  done
  for f in "$root/lib64/hw/"*bluetooth*; do
    [ -e "$f" ] && cp -af "$f" "$OUT/lib64/hw/" || true
  done
  for f in "$root/etc/init/"*blue* "$root/etc/init/"*bt*; do
    [ -e "$f" ] && cp -af "$f" "$OUT/etc/init/" || true
  done
done
ls -laR "$OUT"
