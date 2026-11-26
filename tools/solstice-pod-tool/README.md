# Pod Patcher (Pure Rust Native App)

A single-file, cross-platform native desktop application for **Windows, macOS, and Linux** built in pure Rust (`eframe` / `egui`). No Python, no local HTTP server, no browser tab, and no external prerequisites.

## Features

- **Zero-prereq setup:** Live startup self-checks verify `adb`/`fastboot`, OS images, and Windows fastboot USB drivers. If Platform Tools are missing, click **Download Platform Tools** to fetch and extract Google's official release into a space-free local directory automatically.
- **Easy & Advanced Modes:**
  - **Easy Mode (default):** Clean, safe TV defaults (skips setup wizard, hides battery icon and phone status bar, locks landscape, disables leftover phone apps; Wireless ADB and USB MTP stay off by default, and Backup/Restore protects factory partitions).
  - **Advanced Mode:** Unlocks granular per-feature toggles (including opt-in **Wireless ADB on port 5555** and **USB MTP + ADB**), ADB-only provisioning, per-partition backup selection, and confirmed factory-partition restore.
- **Live Pod Detection (USB & Wireless ADB):** Detects connected Pods in Fastboot or Android (ADB) mode every 2 seconds, plus a built-in **Wireless ADB** connect bar (`192.168.x.x:5555`) so you can manage, install apps on, or back up already-enclosed Pods over Wi-Fi.
- **OS Target Selector:** Supports **Android 15 TV** today, with a built-in roadmap slot for **Linux / Docker** on SP8096.
- **Install, Upgrade & Auto-Resume:** Clean install (never touches `persist`, `modem`, or `bt_firmware`), keep-data OS upgrade, and pass/fail verification checklist.
- **Vetted App Picker (33 apps):** Vanilla default checks only **Projectivy Launcher** and **Aptoide TV**; 31 additional vetted, actively maintained open-source/official apps (`Just Video Player`, `Wholphin`, `TizenTubeCobalt`, `Nuvio`, `WG Tunnel`, `Termux`, `Tailscale`, `Syncthing`, `Home Assistant`, `Jellyfin`, `Nova Video Player`, `RetroArch`, `DuckStation`, `LocalSend`, `Material Files`, `Obtainium`, `F-Droid`, `FCast`, etc.) are opt-in and fetched directly from official upstream releases (zero APKs stored in this repo).
- **Backup & Restore:** ADB root partition backup with `SHA-256` manifest (`backup-manifest.json`), plus Fastboot restore with `SHA-256` verification and a safety lock protecting sensitive factory partitions (`persist`, `modem`, `bluetooth`).

## Unsigned Binary Note (SmartScreen / Gatekeeper)

Prebuilt release binaries are open-source and unsigned (no paid Authenticode/Apple Developer certificate):
- **Windows:** If Microsoft Defender SmartScreen shows *"Windows protected your PC"*, click **More info -> Run anyway** (or build from source below).
- **macOS:** Right-click `PodPatcher` and choose **Open** (or build from source below).

## Building from Source

```bash
cd tools/solstice-pod-tool
cargo build --release
```

The standalone executable is written to `target/release/PodPatcher` (or `PodPatcher.exe` on Windows).
