#!/usr/bin/env bash
set -euo pipefail
POD_DONOR_DIR="${POD_DONOR_DIR:-$PWD/work/h850-donor}"
SRC="${POD_SYSTEM_SRC:-$POD_DONOR_DIR/system.img}"
DST="${POD_SYSTEM_IMG:-$POD_DONOR_DIR/pod-los22-system.img}"
MNT="${POD_SYSTEM_MNT:-/mnt/pod-los22-sys}"
POD_SYS_MAX=3221225472

echo "=== Copying H850 system.img to working Pod copy ==="
cp --sparse=auto "$SRC" "$DST"
SZ=$(stat -c %s "$DST")
echo "Initial size: $SZ bytes (Pod max: $POD_SYS_MAX bytes)"
if (( SZ > POD_SYS_MAX )); then
  echo "Image nominal size ($SZ) > Pod system partition ($POD_SYS_MAX); shrinking filesystem to 2500M..."
  e2fsck -f -p "$DST" || true
  resize2fs "$DST" 2500M
  truncate -s 2500M "$DST"
  SZ=$(stat -c %s "$DST")
  echo "Resized size: $SZ bytes"
  if (( SZ > POD_SYS_MAX )); then
    echo "ERROR: resized image still exceeds Pod system partition!" >&2
    exit 1
  fi
fi

mkdir -p "$MNT"
mount -o loop,rw "$DST" "$MNT"
trap 'umount "$MNT" 2>/dev/null || true' EXIT

VDIR="$MNT/system/vendor"
[[ -d "$VDIR" ]] || VDIR="$MNT/vendor"
echo "Using vendor dir: $VDIR"

echo "=== Patching vendor/etc/fstab.qcom (remove LG sns/drm/mpt) ==="
sed -i '/\/by-name\/sns /d; /\/by-name\/drm /d; /\/by-name\/mpt /d' "$VDIR/etc/fstab.qcom"

echo "=== Patching vendor/etc/init/hw/init.target.rc (remove LG restorecon) ==="
sed -i '/restorecon -RF  *\/persist-lg/d; /restorecon -RF  *\/sns/d; /restorecon -RF  *\/mpt/d' "$VDIR/etc/init/hw/init.target.rc"

echo "=== Setting USB controller & first-boot debug ADB props in vendor/build.prop ==="
VPROP="$VDIR/build.prop"
sed -i '/^sys\.usb\.controller=/d; /^persist\.sys\.usb\.config=/d; /^ro\.adb\.secure=/d' "$VPROP"
cat >> "$VPROP" <<'PROPS'

# SP8096 Pod bring-up overrides
sys.usb.controller=7600000.dwc3
persist.sys.usb.config=adb
ro.adb.secure=0
PROPS

echo "=== Checking patched vendor/etc/fstab.qcom ==="
cat "$VDIR/etc/fstab.qcom"

sync
umount "$MNT"
trap - EXIT

echo "=== Running e2fsck on pod-los22-system.img ==="
e2fsck -f -p "$DST" || true
FINAL_SZ=$(stat -c %s "$DST")
echo "Final size: $FINAL_SZ bytes (Pod max: $POD_SYS_MAX bytes)"
(( FINAL_SZ <= POD_SYS_MAX )) || { echo "ERROR: exceeds Pod system partition!" >&2; exit 1; }
sha256sum "$DST"
echo "=== DONE: $DST ready ==="
