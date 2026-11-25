#!/usr/bin/env bash
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
POD_WORK="${POD_WORK:-$PWD/work}"
KDIR="${POD_KERNEL_SRC:-$POD_WORK/android_kernel_lge_msm8996}"
KOUT="${POD_OUT:-$POD_WORK/build-pod-minimal}"
DTB_OUT="${POD_DTB_OUT:-$POD_WORK/dt-analysis/v5-dtb/pod-diagnostic.dtb}"

cp "$REPO_ROOT/kernel/dts/apq8096-v3-sbc-pod.dts" "$KDIR/arch/arm64/boot/dts/qcom/apq8096-v3-sbc-pod.dts"
mkdir -p "$(dirname "$DTB_OUT")"
cpp -nostdinc -I "$KDIR/arch/arm64/boot/dts" -I "$KDIR/arch/arm64/boot/dts/qcom" -I "$KDIR/include" \
  -undef -x assembler-with-cpp "$KDIR/arch/arm64/boot/dts/qcom/apq8096-v3-sbc-pod.dts" \
  > /tmp/pod-v6.dts.pre
"$KOUT/scripts/dtc/dtc" -O dtb -o "$DTB_OUT" -b 0 -i "$KDIR/arch/arm64/boot/dts/qcom" /tmp/pod-v6.dts.pre
sha256sum "$DTB_OUT"

bash "$REPO_ROOT/kernel/scripts/pack-los22-boot.sh"
