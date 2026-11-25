# Software & TV UI (Work in Progress)

> **Heads up:** The software side is still a work in progress, and **the UI has not been prettied up yet.** Right now the goal has been getting the underlying hardware rock-solid (cold self-boot, 4K@60 Hz HDMI, hardware video decode, 5 GHz Wi-Fi, Bluetooth, Ethernet, and IR remote input) before spending time polishing the front-end look and feel.

## What the UI and device details look like right now

| Current TV launcher (work in progress) | Android 15 & LineageOS 22.2 version |
| :---: | :---: |
| ![Current TV launcher screen capture on the Mersive Solstice Pod Gen3](images/ui-launcher-wip.jpg) | ![Android 15 and LineageOS 22.2 version screen on the Mersive Solstice Pod Gen3](images/ui-android15-version.png) |

| Custom kernel (`4.4.302-podmin+`) & build info | Internal storage (16 GB eMMC) |
| :---: | :---: |
| ![Android kernel 4.4.302-podmin+ and build number on the Mersive Solstice Pod Gen3](images/ui-kernel-version.png) | ![Android 15 Storage settings showing 16 GB internal eMMC on the Mersive Solstice Pod Gen3](images/ui-storage.png) |

You can see the launcher is still rough around the edges (default widgets and a handful of leftover stock Android app icons still sitting in the dock), which is next on the list now that the core hardware is behaving.

### Device details at a glance

| Spec | Running on the Pod |
| --- | --- |
| **Processor** | Qualcomm Snapdragon 820 (`APQ8096`), quad-core Kryo (`arm64-v8a`) |
| **GPU & Video** | Adreno 530 + Qualcomm Venus 4K hardware video decode |
| **Memory** | 4 GB LPDDR4 RAM |
| **Storage** | 16 GB internal eMMC (~9.4 GB used with Android 15 + starter TV apps) |
| **OS** | Android 15 (LineageOS `22.2-20261121-NIGHTLY-h850`) |
| **Kernel** | `4.4.302-podmin+` |

## What is running on the Pod right now

Under the hood, the Mersive Solstice Pod Gen3 (`SP8096`) is running **Android 15 (LineageOS 22.2)** on top of a minimal **Linux 4.4.302** kernel (`4.4.302-podmin`).

- **System base:** Official LineageOS 22.2 for the **LG G5 (`h850`)**, which shares the Qualcomm `MSM8996 / APQ8096` platform (Adreno 530 GPU + Venus 4K hardware video decoder). Phone-only HALs (telephony, fingerprint, sensors, NFC, camera) are stripped out.
- **Wireless HALs:** Official LineageOS 22.2 QTI Wi-Fi and Bluetooth HALs pulled from the **Xiaomi Mi 5 (`gemini`)** so the Pod's `QCA6174A / Rome` radio speaks native Android 15.
- **TV launcher & apps:** Because LineageOS `h850` starts out as a phone build, we skip the phone setup wizard, turn off the lockscreen and sleep timeout, and set a TV-friendly launcher as the default home screen. So far we have tested `tvOS-Launcher` (`com.techgenius.tvoslauncher`) as the default home activity, along with `FLauncher`, `SmartTube`, and `F-Droid` as optional side-loaded apps (downloaded directly from their upstream releases; no APKs are bundled in this repo).

## What we have done on the software side so far

1. **Cold self-boot into Android 15:** Everything lives on internal eMMC (`boot`, `recovery`, `system`), so unplugging power and plugging it back in boots straight into Android 15 without needing a PC attached.
2. **Automatic Wi-Fi and Bluetooth after boot (`init.pod.wifi.rc`):** Instead of risking early-boot kernel panics in `on boot`, `system/init/init.pod.wifi.rc` waits for `sys.boot_completed=1`, loads `bluetooth-power.ko` and the upstream `ath10k_pci` Wi-Fi stack (`cfg80211`, `mac80211`, `ath`, `ath10k_core`, `ath10k_pci`, plus our tiny `wlan.ko` shim), and brings 5 GHz Wi-Fi and Bluetooth A2DP up automatically.
3. **4K@60 Hz living-room output:** Driving `TLMM GPIO 127` HIGH on boot wakes up native `HDMI 1` (`9a0000.qcom,hdmi_tx`) at `3840x2160@60` and `4096x2160@60` (`594 MHz` pixel clock), with Adreno 530 rendering and Venus 4K hardware decode working smoothly.
4. **Native HDMI-CEC TV remote & Flirc USB:** Removing LG's `#define BLOCK_CEC` guard (`0021-mdss-hdmi-tx-enable-cec.patch`) brings Qualcomm's `fb1/cec` controller online on `HDMI 1`, and our lightweight `pod-cec-daemon` (`system/cec/pod-cec-daemon.c`) translates standard TV remote presses straight into Linux `/dev/uinput` key events. A **Flirc USB Gen2** + **Rosewill Media Center IR remote** also works out of the box on either rear USB 3.0 port.
5. **TV box defaults in the installer (`installer/flash-new-pod.ps1`):** On first boot, the installer marks the device provisioned, disables the phone Setup Wizard, disables the lockscreen, sets `screen_off_timeout` to max so the box never goes to sleep mid-movie, and optionally installs any TV APKs you drop into `out/tv-apks/`.

## What is still a work in progress

- **Tidying up the TV UI:** Right now it is a functional bring-up UI, not a finished living-room experience. Cleaning up the launcher layout, hiding leftover stock phone apps, tuning 4K TV DPI/overscan defaults, and making the overall look and feel cohesive is next.
- **HDMI Input (`TC358840`):** The Toshiba HDMI-to-CSI-2 bridge has its device-tree nodes drafted, and the next step is trimming kernel size headroom and bringing up the V4L2 capture driver (see [docs/AUDIO-AND-HDMI-IN.md](AUDIO-AND-HDMI-IN.md)).
