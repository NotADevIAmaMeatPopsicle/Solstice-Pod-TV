#!/usr/bin/env bash
# Host-only minimal 4.4 diagnostic build based on Qualcomm's generic msm_defconfig
# (zero LG phone drivers, zero CONFIG_LGE_DDR_SECTION_SIZE_CHANGE, no battery/charger/UFS).
# Output is not verified safe to boot until tested via temporary fastboot boot.
set -euo pipefail
POD_WORK="${POD_WORK:-$PWD/work}"
POD_KERNEL_SRC="${POD_KERNEL_SRC:-$POD_WORK/android_kernel_lge_msm8996}"
POD_OUT="${POD_OUT:-$POD_WORK/build-pod-minimal}"
POD_JOBS="${POD_JOBS:-$(nproc)}"
POD_LOG="${POD_LOG:-$POD_WORK/pod-minimal-build.log}"

mkdir -p "$POD_OUT" "$(dirname "$POD_LOG")"
exec > >(tee "$POD_LOG") 2>&1

cd "$POD_KERNEL_SRC"
export ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- CROSS_COMPILE_ARM32=arm-linux-gnueabi-
make O="$POD_OUT" msm_defconfig
scripts/config --file "$POD_OUT/.config" \
  -e SERIAL_MSM -e SERIAL_MSM_CONSOLE -e SERIAL_EARLYCON \
  -e CMDLINE_EXTEND -d CMDLINE_FROM_BOOTLOADER \
  --set-str CMDLINE "earlycon=msm_serial_dm,0x075b0000 console=ttyMSM0,115200n8 ignore_loglevel androidboot.selinux=permissive watchdog_v2.enable=0" \
  -e MMC_SDHCI_MSM \
  -e USB_CONFIGFS -e USB_CONFIGFS_F_FS -e USB_CONFIGFS_UEVENT \
  -e NOP_USB_XCEIV \
  -e POD_USB_VBUS \
  -e NET_VENDOR_ATHEROS -e ATL1C \
  -e FB_MSM -e FB_MSM_MDSS -e MSM_DBA -e MSM_DBA_ADV7533 -e FB_MSM_MDSS_HDMI_PANEL \
  -e MEDIA_SUPPORT -e MEDIA_CAMERA_SUPPORT -e MEDIA_CONTROLLER -e VIDEO_DEV -e VIDEO_V4L2 -e V4L_PLATFORM_DRIVERS -e MSM_VIDC_V4L2 \
  -e SOUND -e SND_USE_SEC_MI2S -e SND_POD_SEC_MI2S \
  -e PSTORE -e PSTORE_RAM -e PSTORE_CONSOLE \
  -e TEST_POWER \
  -e SECURITY_SELINUX -e SECURITY_SELINUX_DEVELOP \
  -e SCHED_TUNE -e CGROUP_SCHEDTUNE \
  -e BPF_JIT -e BPF_SYSCALL -e CGROUP_BPF -e CGROUP_WRITEBACK \
  -e MEMCG -e MEMCG_SWAP -e MEMCG_SWAP_ENABLED \
  -e NETFILTER_XT_MATCH_BPF -e NETFILTER_XT_MATCH_OWNER -e NET_ACT_BPF -e NET_CLS_BPF \
  -e OVERLAY_FS \
  -d QPNP_SMBCHARGER -d QPNP_FG \
  -d SCSI_UFSHCD -d SCSI_UFSHCD_PLATFORM -d SCSI_UFS_QCOM \
  -d MSMB_CAMERA -d MSM_SDE_ROTATOR -d SOC_CAMERA -d MEDIA_ANALOG_TV_SUPPORT -d MEDIA_DIGITAL_TV_SUPPORT -d MEDIA_RADIO_SUPPORT -d MEDIA_SDR_SUPPORT -d MEDIA_RC_SUPPORT -d MEDIA_USB_SUPPORT -d NFC -d INPUT_TOUCHSCREEN \
  -d SND_LGE_EFFECT -d SND_LGE_NORMALIZER -d SND_LGE_MABL -d SND_USB -d MSM_ULTRASOUND \
  -d FUNCTION_TRACER -d FTRACE -d TRACING -d CORESIGHT -d KALLSYMS_ALL -d MSM_RTB -d PROFILING \
  -d BTFM_SLIM -m BT -m MSM_BT_POWER \
  -d FB_MSM_MDSS_LIVEDISPLAY \
  -d MODULE_SIG_FORCE \
  -d QCA_CLD_WLAN -d CNSS -d CNSS_UTILS -d CNSS_CRYPTO \
  -m MAC80211 -e MAC80211_RC_MINSTREL -e MAC80211_RC_MINSTREL_HT -m ATH_COMMON -e ATH_CARDS -m ATH10K -m ATH10K_PCI \
  --set-str LOCALVERSION -podmin
make O="$POD_OUT" olddefconfig
make O="$POD_OUT" -j"$POD_JOBS" KCFLAGS="-Wno-error" Image.gz modules
sha256sum "$POD_OUT/arch/arm64/boot/Image.gz" "$POD_OUT/drivers/net/wireless/ath/ath10k/ath10k_pci.ko"
