# Solstice Pod TV Flasher & Manager (GUI)

A lightweight, cross-platform flashing, upgrading, and backup/restore UI for the **Mersive Solstice Pod Gen3 (`SP8096`)** that runs on **Windows, macOS, and Linux** using only the Python 3 standard library (no `pip install`, no Electron, no external runtime).

## Quick start

1. Install **Python 3.10+** and **Android Platform Tools** (`adb` and `fastboot`):
   - **macOS:** `brew install --cask android-platform-tools`
   - **Linux (Ubuntu/Debian):** `sudo apt install adb fastboot`
   - **Windows:** Extract Android Platform Tools into `.\platform-tools` (or set the folder in the **Paths & Settings** tab) and install the Google `Android Bootloader Interface` USB driver.
2. Launch the UI:
   - **Windows:** double-click `launch.bat` or run `python tools\pod-flasher\pod_flasher.py`
   - **macOS / Linux:** run `bash tools/pod-flasher/launch.sh` or `python3 tools/pod-flasher/pod_flasher.py`
3. Your browser opens automatically at `http://127.0.0.1:8765`.

## What it does

- **Live Pod detection & status (every 2 s):** Detects connected Pods in **Fastboot** or **Android (ADB)** mode and shows serial, OS version, kernel (`uname -r`), Wi-Fi IP, free `/data` storage, HDMI-CEC / hot-plug service status, and default home launcher. On Windows, it also flags fastboot Pods that still need the `Android Bootloader Interface` driver bound in Device Manager.
- **Full Clean Install (resumable):** Flashes `boot`, `recovery`, and `system`, formats `userdata` and `cache` as `ext4`, applies TV defaults (hides battery icon and phone status bar, locks landscape, disables leftover phone apps), installs optional APKs, and runs a pass/fail verification checklist. If a Pod is already booted in Android, it skips flashing and resumes provisioning automatically.
- **Upgrade (keep data & apps):** Flashes updated `boot`, `recovery`, and `system` images without wiping `userdata` or your installed apps.
- **Partition Backup & Restore:**
  - **Backup (ADB Root):** Dumps selected partitions (`boot`, `recovery`, `persist`, `modem`, `bluetooth`, `aboot`) to a timestamped per-serial folder with a `SHA-256` manifest (`backup-manifest.json`).
  - **Restore (Fastboot):** Verifies every file's `SHA-256` hash before flashing. Sensitive factory partitions (`persist`, `modem`, `bluetooth`) are skipped by default unless explicitly unlocked so you never accidentally overwrite another Pod's MAC addresses.
- **Quick actions:** One-click `Reboot to Fastboot`, `Reboot to Android`, `Enable Wi-Fi ADB (5555)`, and `Enable MTP + ADB`.
