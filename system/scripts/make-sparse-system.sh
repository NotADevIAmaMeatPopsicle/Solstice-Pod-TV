#!/usr/bin/env bash
set -euo pipefail
POD_DONOR_DIR="${POD_DONOR_DIR:-$PWD/work/h850-donor}"
POD_OUT_DIR="${POD_OUT_DIR:-$PWD/out/images}"
SRC_IMG="${POD_SYSTEM_IMG:-$POD_DONOR_DIR/pod-los22-system.img}"
mkdir -p "$POD_OUT_DIR"
if ! command -v img2simg >/dev/null 2>&1; then
  echo "Installing android-sdk-libsparse-utils..."
  apt-get update -qq && apt-get install -y -qq android-sdk-libsparse-utils
fi
echo "Converting $SRC_IMG to sparse image..."
img2simg "$SRC_IMG" "$POD_OUT_DIR/pod-los22-system.simg"
ls -lh "$POD_OUT_DIR/pod-los22-system.simg"
sha256sum "$POD_OUT_DIR/pod-los22-system.simg"
