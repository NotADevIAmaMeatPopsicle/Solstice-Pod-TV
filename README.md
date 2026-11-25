# Mersive Solstice Pod Gen3 (SP8096) — Android 15 (LineageOS 22.2) TV Box Port

An experimental **Android 15 (LineageOS 22.2)** port turning the **Mersive Solstice Pod Gen3** (`SP8096` / `MER6641`, Qualcomm Snapdragon 820 `APQ8096`, 4 GB RAM, 16 GB eMMC, dual HDMI out, Gigabit Ethernet, QCA6174A Wi-Fi + Bluetooth) into a standalone 4K Android TV box.

> [!WARNING]
> **Read [docs/SAFETY.md](docs/SAFETY.md) before flashing anything.**
> The Solstice Pod Gen3 has **no external reset pinhole** — physical fastboot recovery requires opening the case to hold the internal PCB Volume-Down button. Always back up your unit's stock partitions first, always verify kernel image layout (`< 0x2000000`), and **always RAM-boot (`fastboot boot`) any untested kernel before flashing `boot`**.

---

## Hardware Support Matrix

| Subsystem | Hardware / Driver | Status | Notes |
| --- | --- | --- | --- |
| **SoC / Boot** | Qualcomm `APQ8096` (Kryo quad-core, 4 GB RAM, 16 GB eMMC) | Working | Self-boots Android 15 (LineageOS 22.2) on Linux 4.4.302 (`-podmin`) |
| **GPU** | Adreno 530 | Working | Full hardware acceleration |
| **HW Video Decode** | Qualcomm Venus (`/dev/video32`/`33`) | Working | Smooth 4K hardware decode (SmartTube, etc.) |
| **HDMI 1 (4K Out)** | Native `9a0000.qcom,hdmi_tx` (`fb1`, TLMM GPIO 127) | Working | Verified up to `4096x2160@60 Hz` / `3840x2160@60 Hz` (`594 MHz` pixel clock), `display.qservice` mode switching |
| **HDMI 2 (1080p Out)** | DSI0 → `ADV7533` bridge (`fb0`) | Working | `1920x1080@60 Hz` |
| **HDMI Audio** | `SEC_MI2S_RX` | Working | Clean playback over HDMI |
| **Wi-Fi (2.4 / 5 GHz)** | QCA6174A hw3.2 (`0000:01:00.0`, upstream `ath10k_pci`) | Working | 5 GHz 802.11ac (up to 866 Mbps Rx), `WLAN.RM.4.4.1` (`firmware-5.bin`), auto-loaded after `sys.boot_completed=1` |
| **Bluetooth** | QCNFA324 / `bt_qca6174` (`bluetooth-power.ko` + QTI BT HAL) | Working | Verified with A2DP audio + simultaneous Wi-Fi coexistence |
| **Gigabit Ethernet** | Atheros `atl1c` (`0002:01:00.0` on `&pcie2`) | Working | 1000 Mbps Full Duplex |
| **USB 3.0 Type-A Host** | `6a00000.dwc3` (`pod-usb-vbus`) | Working | Verified with wireless KB/mouse & Flirc USB Gen2 IR receiver |
| **USB-C ADB / Fastboot** | `7600000.dwc3` (ConfigFS `ffs.adb`) | Working | Persistent `adb root` debugging |
| **3.5 mm Audio Jack** | WCD9335 (`J6 AUDIO`, `SLIMBUS_6_RX`) | Partial | Left channel verified; right-channel mixer patch staged, awaiting headphone re-test |
| **HDMI Input** | Toshiba `TC358840` (HDMI → dual 4-lane MIPI CSI-2) | Not Yet Working | DTB nodes grafted; needs `camera_v2` headroom trim + V4L2 `tc358840` driver (see [docs/AUDIO-AND-HDMI-IN.md](docs/AUDIO-AND-HDMI-IN.md)) |
| **HDMI-CEC** | `qcom,hdmi_tx` CEC | Not Yet Working | Requires porting `android.hardware.tv.cec` HAL; Flirc USB works out of the box as an immediate IR remote alternative |

---

## Repository Layout

```text
sp8096-lineageos22-androidtv/
├── README.md                 # Project overview & hardware matrix
├── CONTRIBUTING.md           # Non-brick safety rules & PR guidelines
├── LICENSE                   # Apache-2.0 (scripts/docs); kernel patches are GPL-2.0
├── NOTICE.md                 # Licensing & proprietary-blob policy
├── docs/
│   ├── ARCHITECTURE.md       # Donor ROM, kernel, HALs, and ath10k bring-up details
│   ├── BUILD.md              # Step-by-step build-from-source guide
│   ├── SAFETY.md             # Bootloader 0x2000000 limit, RAM-boot rule, unbrick notes
│   └── AUDIO-AND-HDMI-IN.md  # 3.5 mm audio & TC358840 HDMI-in roadmap
├── kernel/
│   ├── dts/                  # apq8096-v3-sbc-pod.dts
│   ├── patches/              # Numbered Linux 4.4.302 patches (0001..0020)
│   ├── shims/                # pod-usb-vbus.c & wlan.c (fwpath shim for ath10k_pci)
│   └── scripts/              # Kernel build, layout check, and boot-image verification
├── system/
│   ├── init/                 # init.pod.wifi.rc (GPIO 127, BT, safe boot_completed Wi-Fi)
│   └── scripts/              # System image prep, HAL patching, and sparse conversion
└── installer/
    ├── flash-new-pod.ps1     # Turnkey Windows fastboot flasher & TV app provisioner
    └── README.md             # Step-by-step flashing guide for SP8096 units
```

---

## Quick Start (Flashing a Pod)

This repository intentionally ships **no proprietary firmware blobs, stock dumps, or prebuilt ROM binaries** (see [NOTICE.md](NOTICE.md)). Once you have built your 3 images (`pod44-los22-boot-signed.img`, `pod44-los22-recovery-signed.img`, `pod-los22-system.simg`) from official LineageOS 22.2 donors + this repo's kernel/system scripts:

1. Back up the target Pod's stock `boot`, `recovery`, `persist`, `modem`, and `bluetooth` partitions.
2. Boot the target Pod into fastboot and connect USB-C.
3. Run the turnkey installer (flashes `boot`, `recovery`, `system`, formats `userdata`/`cache`, never touches `persist`/`bt_firmware`/`modem`, and provisions TV settings):
   ```powershell
   powershell -ExecutionPolicy Bypass -File .\installer\flash-new-pod.ps1 -PodName pod-02
   ```

See [installer/README.md](installer/README.md) and [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for full build and flashing instructions.

---

## Credits & Upstream Projects

- **LineageOS** — Official LineageOS 22.2 (`h850` LG G5 system/kernel donor and `gemini` Xiaomi Mi 5 QCA6174A Wi-Fi/BT HAL donor)
- **Linux `ath10k` & `linux-firmware`** — Upstream QCA6174A hw3.2 PCIe Wi-Fi driver and `WLAN.RM.4.4.1` firmware
- **Solstice Pod Gen3 Android TV community** — Prior Android 7.1.1 / 11 bring-up research, boot-signing, and partition mapping
