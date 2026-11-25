# Legal & Licensing Notice

This repository contains **only original scripts, device tree sources, kernel patches, init configurations, and documentation** for porting Android 15 (LineageOS 22.2) to the Mersive Solstice Pod Gen3 (`SP8096`).

## What Is NOT Included
- **No proprietary Qualcomm, Mersive, LG, or Xiaomi binaries/blobs** (no `NON-HLOS.bin`, `bt_firmware`, ACDB files, `wlan.ko`, or QTI HAL `.so` files).
- **No third-party APKs** (users download launchers/apps such as FLauncher, tvOS-Launcher, SmartTube, and F-Droid directly from their upstream releases).
- **No device-specific backups, MAC addresses, serial numbers, calibration data (`persist`), or personal credentials.**

## Component Licenses
- Scripts, installer, init configs, and documentation in this repository: **Apache License 2.0** (see `LICENSE`).
- Linux kernel patches (`kernel/patches/`), Device Tree Source (`kernel/dts/`), and kernel shim modules (`kernel/shims/`): **GNU General Public License v2.0 (GPL-2.0)**, matching the upstream Linux 4.4 kernel tree.
- Official LineageOS 22.2 donor builds (`h850` / `gemini`) and `linux-firmware` QCA6174 firmware remain subject to their respective upstream licenses.
