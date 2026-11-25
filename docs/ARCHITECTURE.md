# Port Architecture

## 1. Donor & Kernel Strategy
Instead of maintaining a full AOSP fork from scratch, this port combines:
- **System / Userspace Donor:** Official **LineageOS 22.2 (Android 15)** for the **LG G5 (`h850`)** (`msm8996` Snapdragon 820, 64-bit, Adreno 530, Venus video decode).
- **QCA6174A Wi-Fi & Bluetooth HAL Donor:** Official **LineageOS 22.2** for the **Xiaomi Mi 5 (`gemini`)**, which shares the exact **QCA6174A / Rome** combo chip (`android.hardware.wifi-service`, `libwifi-hal.so`, `android.hardware.bluetooth@1.0-service-qti`, `android.hardware.bluetooth@1.0-impl-qti.so`).
- **Kernel:** Minimal **Linux 4.4.302** (`android_kernel_lge_msm8996`) configured from Qualcomm's generic `msm_defconfig` (zero LG phone drivers, zero UFS/battery/charger bloat, `msm_watchdog_v2` enabled) + Pod Gen3 DTB (`kernel/dts/apq8096-v3-sbc-pod.dts`) + patches `0001` through `0021`.

## 2. Key Subsystem Solutions

### Dual HDMI Output (4K HDMI 1 + 1080p HDMI 2) & Hot-Plug Reconnect
- **HDMI 2 (`fb0`, primary):** Driven via `DSI0 → ADV7533` bridge (`0x39`, IRQ GPIO 95) at `1920x1080@60 Hz` with `SEC_MI2S_RX` audio.
- **HDMI 1 (`fb1`, native 4K):** Driven by APQ8096's native `9a0000.qcom,hdmi_tx`. Enabled on boot by driving **TLMM GPIO 127 HIGH** in `system/init/init.pod.wifi.rc`. Because Android 15 assigns a new incrementing `displayId` on every HDMI hot-plug or TV input switch and leaves it in `state OFF` waiting for a mirror prompt, a supervised `pod_hdmi_hotplug` service (`system/scripts/pod-hdmi-hotplug.sh`) automatically enables any connected external display within 1 to 2 seconds. Verified locking `4096x2160@60 Hz` and `3840x2160@60 Hz` (`594 MHz` pixel clock).

### Native HDMI-CEC Remote (`fb1/cec` + `pod-cec-daemon`)
- Patch `0021-mdss-hdmi-tx-enable-cec.patch` removes LG's hardcoded `#define BLOCK_CEC` in `drivers/video/fbdev/msm/mdss_hdmi_tx.c`, bringing Qualcomm's `mdss_hdmi_cec` sysfs interface online at `/sys/devices/virtual/graphics/fb1/cec/` (`GPIO_31` muxed to `hdmi_cec`).
- `system/cec/pod-cec-daemon.c` runs as a supervised `init` service (`pod_cec`), claims logical address **`8`** (`Playback Device 2`, avoiding collisions with game consoles on `LA 4`), stays polite on `0x85` (`<Request Active Source>`) so it never steals TV inputs when switching away, and injects standard TV remote key presses via `/dev/uinput`.

### 3.5 mm Stereo Audio (`J6 AUDIO`, WCD9335)
- Routed `SND_DEVICE_OUT_HEADPHONES` / `SND_DEVICE_OUT_LINE` to `SLIMBUS_6_RX` in `audio_platform_info.xml` and mapped `SLIM RX2` / `SLIM RX3` (`AIF4_PB`, `SLIM_6_RX Channels = Two`, `RX INT1_2 MUX = RX2`, `RX INT2_2 MUX = RX3`) in `mixer_paths_tasha.xml`. Verified stereo left + right output.

### Wi-Fi (`QCA6174A hw3.2` via Upstream `ath10k_pci`)
Stock `qcacld-3.0` (`wlan.ko`) was replaced with upstream **`ath10k_pci`** for stability on Linux 4.4.302:
1. **Clean PCIe RC0 without `cnss`:** Disabled `CONFIG_CNSS` / `CONFIG_QCA_CLD_WLAN` (freeing 68 KB of kernel headroom) and removed `qcom,boot-option` / `qcom,l1ss-supported` from `&pcie0` so `0000:01:00.0` (`168c:003e`) enumerates cleanly at boot.
2. **CAF `ce.c` NULL-pointer fix (`0017`):** Guarded the Qualcomm `WCN3990` `vaddr_rri_on_ddr` dereference in `ath10k_ce_send_nolock()` with `if (ar->rri_on_ddr)` (otherwise QCA6174A PCI crashed at `0x0000001c` during BMI probe).
3. **PCIe MSI & power fixes (`0018`):** Closed the edge-triggered MSI unmask race in `ath10k_pci_napi_poll()`, kept PCIe ASPM disabled in `ath10k_pci_hif_start()`, and set `ar_pci->pci_ps = false` for wall-powered APQ8096.
4. **Firmware:** Uses upstream `linux-firmware` `QCA6174/hw3.0/firmware-6.bin` (`WLAN.RM.4.4.1-00309`) installed as `firmware-5.bin` (API 5) alongside `board-2.bin` (`bmi_id 0:1`).
5. **Android 15 Wi-Fi HAL compatibility (`kernel/shims/wlan.c`):** A minimal 8-line `wlan.ko` module exposes `/sys/module/wlan/parameters/fwpath` (`0660`, chowned `wifi:wifi` + `0666` on `sys.boot_completed=1`) so `android.hardware.wifi-service` configures `wlan0` cleanly.

### Bluetooth (`QCNFA324` / `bt_qca6174`)
- Built `CONFIG_BT=m` and `CONFIG_MSM_BT_POWER=m` (`bluetooth-power.ko`), patched with `0019` (`extldo` writable sysfs attr + `#ifdef CONFIG_BTFM_SLIM` guard).
- Mounts the Pod's stock `/bt_firmware` partition read-only and runs LineageOS 22.2 `gemini`'s QTI Bluetooth HAL (`qcom.bluetooth.soc=rome`).

### USB 3.0 Host & IR Remote
- `pod-usb-vbus.c` (`0002`, `0014`) + `0015` (DWC3 null workqueue fix) bring up USB 3.0 Type-A host cleanly alongside USB-C ConfigFS ADB.
- Standard USB HID keyboards, mice, and **Flirc USB Gen2** IR receivers work natively without extra drivers.
