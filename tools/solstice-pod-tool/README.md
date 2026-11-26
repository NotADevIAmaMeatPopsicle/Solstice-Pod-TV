# Pod Patcher (Pure Rust Native App)

A single-file, cross-platform native desktop application for **Windows, macOS, and Linux** built in pure Rust (`eframe` / `egui`). No Python, no local HTTP server, no browser tab, and no external prerequisites.

## Features

- **Zero-prereq setup:** Live startup self-checks verify `adb`/`fastboot`, OS images, and Windows fastboot USB drivers. If Platform Tools are missing, click **Download Platform Tools** to fetch and extract Google's official release into a space-free local directory automatically.
- **OS Target selector:** Supports **Android 15 TV** today, with a built-in roadmap slot for **Linux / Docker** on SP8096.
- **Live Pod detection (every 2 s):** Detects connected Pods in Fastboot or Android (ADB) mode with serial, kernel, Android version, Wi-Fi IP, `/data` storage, HDMI-CEC, and hot-plug status.
- **Install, Upgrade & Auto-Resume:** Clean install (never touches `persist`, `modem`, or `bt_firmware`), keep-data OS upgrade, or ADB-only provisioning + pass/fail verification checklist.
- **Vetted App Picker (33 apps):** Vanilla default checks only **Projectivy Launcher** and **Aptoide TV**; 31 additional vetted, actively maintained open-source/official apps (`Just Video Player`, `Wholphin`, `TizenTubeCobalt`, `Nuvio`, `WG Tunnel`, `Termux`, `Tailscale`, `Syncthing`, `Home Assistant`, `Jellyfin`, `Nova Video Player`, `RetroArch`, `DuckStation`, `LocalSend`, `Material Files`, `Obtainium`, `F-Droid`, `FCast`, etc.) are opt-in and fetched directly from official upstream releases (zero APKs stored in this repo).
- **Backup & Restore:** ADB root partition backup with `SHA-256` manifest (`backup-manifest.json`), plus Fastboot restore with `SHA-256` verification and a safety lock protecting sensitive factory partitions (`persist`, `modem`, `bluetooth`).

## Building from source

```bash
cd tools/solstice-pod-tool
cargo build --release
```

The standalone executable is written to `target/release/solstice-pod-tool` (or `solstice-pod-tool.exe` on Windows).
