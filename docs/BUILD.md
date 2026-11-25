# Building from Source

This guide reproduces the full **SP8096 Android 15 (LineageOS 22.2)** boot, recovery, and system images on **Ubuntu 24.04** (native Linux or WSL2).

---

## 1. Host Prerequisites (Ubuntu 24.04 / WSL2)

```bash
sudo apt-get update
sudo apt-get install -y \
  git bc bison flex libssl-dev device-tree-compiler \
  gcc-aarch64-linux-gnu binutils-aarch64-linux-gnu \
  python3 cpio gzip xz-utils e2fsprogs \
  android-sdk-libsparse-utils
```

## 2. Upstream Donor Inputs (Not Stored in This Repo)

Download or extract these upstream inputs into a local working directory (e.g. `export POD_WORK="$HOME/sp8096-build"`):

1. **Kernel source:** LineageOS `android_kernel_lge_msm8996` (Linux 4.4.302)
   ```bash
   git clone --depth 1 https://github.com/LineageOS/android_kernel_lge_msm8996.git "$POD_WORK/android_kernel_lge_msm8996"
   ```
2. **System donor:** Official **LineageOS 22.2** ZIP for **LG G5 (`h850`)** — extract `system.img` and the boot ramdisk into `$POD_DONOR_DIR`.
3. **QCA6174A Wi-Fi & BT HAL donor:** Official **LineageOS 22.2** ZIP for **Xiaomi Mi 5 (`gemini`)** — extract QTI Wi-Fi/BT HALs using `system/scripts/extract-gemini-wifi-hal.sh` and `system/scripts/extract-gemini-bt-hal.sh`.
4. **Upstream QCA6174A Wi-Fi firmware (`linux-firmware`):**
   - `ath10k/QCA6174/hw3.0/firmware-6.bin` (`WLAN.RM.4.4.1-00309`) → install as `firmware-5.bin`
   - `ath10k/QCA6174/hw3.0/board-2.bin` and `board.bin`
5. **Your Pod's own stock `boot-backup.img`** (for boot header geometry/pagesize/cmdline reference).

---

## 3. Build the Kernel, DTB, and Matched Modules

```bash
export POD_KERNEL_SRC="$POD_WORK/android_kernel_lge_msm8996"
export POD_OUT="$POD_WORK/build-pod-minimal"
export POD_JOBS="$(nproc)"

# Apply Pod patches 0001..0020 and copy shims
cd "$POD_KERNEL_SRC"
for p in /path/to/repo/kernel/patches/*.patch; do
  git apply "$p"
done
cp /path/to/repo/kernel/shims/wlan.c drivers/misc/wlan.c
grep -q "wlan.o" drivers/misc/Makefile || echo "obj-m += wlan.o" >> drivers/misc/Makefile

# Build Image.gz and loadable modules (ath10k_pci, bluetooth-power, wlan, etc.)
bash /path/to/repo/kernel/scripts/build-minimal-diagnostic.sh

# Build DTB, enforce < 0x2000000 layout ceiling, pack & verify signed boot image
bash /path/to/repo/kernel/scripts/rebuild-dtb-and-pack-los22.sh
```

> [!IMPORTANT]
> Always verify `check-image-layout.py` and `verify-packed-diagnostic.py` pass before RAM-booting (`fastboot boot`) the resulting boot image.

---

## 4. Build the Patched LineageOS 22.2 System Image

```bash
export POD_DONOR_DIR="$POD_WORK/h850-donor"
export POD_OUT_DIR="$POD_WORK/out"

bash /path/to/repo/system/scripts/prepare-pod-los22-system.sh
bash /path/to/repo/system/scripts/patch-los22-pod-hals.sh
```

This produces `$POD_OUT_DIR/pod-los22-system.simg` ready for `installer/flash-new-pod.ps1`.
