#!/usr/bin/env bash
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
POD_WORK="${POD_WORK:-$PWD/work}"
POD_OUT_DIR="${POD_OUT_DIR:-$PWD/out/images}"
mkdir -p "$POD_WORK/los22-boot-ramdisk" "$POD_OUT_DIR"
RAMDISK_RAW="${POD_RAMDISK_RAW:-$POD_WORK/h850-boot-ramdisk.raw}"
(
  cd "$POD_WORK/los22-boot-ramdisk"
  rm -rf *
  xz -dc "$RAMDISK_RAW" | cpio -idmu
)
IMG="${POD_OUT:-$POD_WORK/build-pod-minimal}/arch/arm64/boot/Image.gz"
DTB="${POD_DTB_OUT:-$POD_WORK/dt-analysis/v5-dtb/pod-diagnostic.dtb}"
STOCK="${POD_STOCK_BOOT:-$POD_OUT_DIR/boot-backup.img}"
OUT_DIR="${POD_PACK_DIR:-$POD_WORK/packing-validation}"
mkdir -p "$OUT_DIR"
RAMDISK="$OUT_DIR/los22-pod-boot-ramdisk.gz"
OUT_U="$OUT_DIR/pod44-los22-boot-unsigned.img"
OUT_S="$OUT_DIR/pod44-los22-boot-signed.img"
EXTRA_CMDLINE="console=ttyMSM0,115200n8 earlycon=msm_serial_dm,0x075b0000 ignore_loglevel androidboot.selinux=permissive androidboot.boot_devices=soc/7464900.sdhci loop.max_part=7"

python3 "$REPO_ROOT/kernel/scripts/check-image-layout.py" "$IMG" "$DTB" --boot-header "$STOCK"
(cd "$POD_WORK/los22-boot-ramdisk" && find . | cpio -o -H newc --quiet | gzip -9 > "$RAMDISK")

KEY="${VERITY_KEY:-$(find "$OUT_DIR" -maxdepth 2 \( -name '*verity*.pem' -o -name '*.pem' \) | head -n 1)}"
PACK_SCRIPT="${POD_PACK_SCRIPT:-$POD_WORK/pack-pod-boot.py}"
VERITY_KEY="$KEY" POD_EXTRA_CMDLINE="$EXTRA_CMDLINE" \
  python3 -I "$PACK_SCRIPT" \
  "$IMG" "$DTB" "$STOCK" "$RAMDISK" "$OUT_U" "$OUT_S"

python3 "$REPO_ROOT/kernel/scripts/verify-packed-diagnostic.py" \
  "$OUT_S" "$IMG" "$DTB" "$RAMDISK" "$STOCK" --extra-cmdline "$EXTRA_CMDLINE"

cp "$OUT_S" "$POD_OUT_DIR/pod44-los22-boot-signed.img"
sha256sum "$RAMDISK" "$OUT_S"
echo "=== DONE: staged $POD_OUT_DIR/pod44-los22-boot-signed.img ==="
