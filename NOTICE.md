# Legal & Licensing Notice

This repository contains **only original scripts, device tree sources, kernel patches, init configurations, and documentation** for running Android 15 (LineageOS 22.2) and Mainline Linux 6.12 LTS on the Mersive Solstice Pod Gen3 (`SP8096`).

## What Is NOT Included
- **No proprietary Qualcomm, Mersive, LG, or Xiaomi binaries/blobs** (no `NON-HLOS.bin`, `bt_firmware`, ACDB files, proprietary `qcacld-3.0` `wlan.ko`, or QTI HAL `.so` files).
- **No third-party APKs** (users download launchers/apps such as Projectivy Launcher, Aptoide TV, SmartTube, and F-Droid directly from their upstream releases).
- **No device-specific backups, MAC addresses, serial numbers, calibration data (`persist`), or personal credentials.**

## Component Licenses
- Scripts, installer, init configs, and documentation in this repository: **Apache License 2.0** (see `LICENSE`).
- Linux kernel patches (`kernel/patches/`), Device Tree Source (`kernel/dts/`), and kernel shim modules (`kernel/shims/`): **GNU General Public License v2.0 (GPL-2.0)**, matching the upstream Linux 4.4 kernel tree.
- Official LineageOS 22.2 donor builds (`h850` / `gemini`) and `linux-firmware` QCA6174 firmware remain subject to their respective upstream licenses.

## Credits & Prior Art
- Initial Mersive Solstice Pod Gen3 (`SP8096`) Android TV bring-up lead and reference work: [erikb-lab/solstice-pod-gen3-androidtv](https://github.com/erikb-lab/solstice-pod-gen3-androidtv).

## Reference Images & Trademarks
- Front and rear enclosure reference images (`docs/images/solstice-pod-gen3-stock.png` and `docs/images/solstice-pod-gen3-rear.png`) are from Mersive Technologies hardware documentation and are used solely for hardware identification.
- The underside enclosure teardown reference (`docs/images/teardown-bottom-4-feet.jpg`) is adapted from the public FCC equipment authorization filing (`2ADCSSP8096`) for repair and teardown documentation.
- All PCB and bench photographs are original.
- Mersive and Solstice are trademarks of Mersive Technologies. This is an independent community project and is not affiliated with or endorsed by Mersive Technologies.
