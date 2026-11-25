#!/usr/bin/env bash
set -euo pipefail
ZIP="/mnt/e/SP8096-Android15/donor-images/lineage-22.2-20261118-nightly-gemini-signed.zip"
POD_DONOR_DIR="${POD_DONOR_DIR:-$PWD/work/h850-donor}"
OUT="${POD_WIFI_HAL_OUT:-$POD_DONOR_DIR/gemini-qca-wifi-hal}"
TMP="/tmp/gemini-wifi-extract"
MNT="/mnt/gemini-vendor"
mkdir -p "$OUT" "$TMP" "$MNT"

cleanup() {
  if mountpoint -q "$MNT"; then umount "$MNT" || true; fi
  rm -rf "$TMP"
}
trap cleanup EXIT

unzip -l "$ZIP" | head -n 30
# Check if vendor.new.dat.br or system.new.dat.br
if unzip -l "$ZIP" | grep -q "vendor.new.dat.br"; then
  PART="vendor"
else
  PART="system"
fi
if ! command -v brotli >/dev/null 2>&1; then
  apt-get update -qq && apt-get install -y -qq brotli
fi
echo "Extracting $PART.new.dat.br + $PART.transfer.list..."
unzip -o -q "$ZIP" "$PART.new.dat.br" "$PART.transfer.list" -d "$TMP"
brotli -d "$TMP/$PART.new.dat.br" -o "$TMP/$PART.new.dat"

# Use existing sdat2img.py or python inline converter
python3 - "$TMP/$PART.transfer.list" "$TMP/$PART.new.dat" "$TMP/$PART.img" <<'PYEOF'
import sys, os
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
print(f"Reconstructed {out_path} ({max_block * BLOCK_SIZE} bytes)")
PYEOF

mount -o loop,ro "$TMP/$PART.img" "$MNT"
VDIR="$MNT"
if [ -d "$MNT/system/vendor" ]; then
  VDIR="$MNT/system/vendor"
elif [ -d "$MNT/vendor" ]; then
  VDIR="$MNT/vendor"
fi
echo "Copying QCA Wi-Fi HAL files from $VDIR..."
cp -f "$VDIR/lib64/libwifi-hal.so" "$OUT/"
cp -f "$VDIR/bin/hw/android.hardware.wifi-service" "$OUT/"
if [ -f "$VDIR/lib64/libcld80211.so" ]; then cp -f "$VDIR/lib64/libcld80211.so" "$OUT/"; fi
if [ -d "$VDIR/etc/wifi" ]; then cp -rf "$VDIR/etc/wifi" "$OUT/"; fi
ls -lh "$OUT"
strings "$OUT/libwifi-hal.so" | grep -iE "qca|qcom|bcmdhd|wlan" | head -n 10
