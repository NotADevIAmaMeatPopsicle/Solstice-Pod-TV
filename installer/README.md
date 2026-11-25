# Flashing a Solstice Pod Gen3 (SP8096)

## Prerequisites
1. **Android Platform Tools** (`adb.exe`, `fastboot.exe`) and the **Google USB Driver** (`Android Bootloader Interface` bound to `USB\VID_18D1&PID_D00D` in Windows Device Manager).
2. **Prebuilt images** (built via `kernel/scripts/` and `system/scripts/` from official LineageOS 22.2 `h850` + `gemini` donors):
   - `pod44-los22-boot-signed.img`
   - `pod44-los22-recovery-signed.img`
   - `pod-los22-system.simg`
3. **Optional TV APKs** placed in `tv-apks/` (`tvOS-Launcher`, `FLauncher`, `SmartTube`, `F-Droid` — downloaded from their official releases).

## Steps (~5–10 Minutes per Unit)
1. **Open the enclosure:** Unplug power and all cables, peel back the **4 rubber feet** on the bottom to remove the 4 case screws, and lift off the bottom cover. Remove the **6 screws on the underside of the PCB** (4 corner + 2 inner) and lift the board off the internal metal heatsink to access the top side.
2. **Enter fastboot:** Connect a **USB-C data cable** from the Pod's **front USB-C port** to your PC. **Press and hold `S3` (`VOL-`)** on the PCB, plug in **12 V DC power** (or PoE+), keep holding `VOL-` for **~3–5 seconds**, and release. Confirm detection with `fastboot devices` (or run `adb reboot bootloader` if ADB is already available).
3. **Back up stock partitions** (`boot`, `recovery`, `persist`, `modem`, `bluetooth`) to local storage.
4. **Dry-run verify** your image paths:
   ```powershell
   powershell -ExecutionPolicy Bypass -File .\installer\flash-new-pod.ps1 -ImagesDir .\out\images -PlatformToolsDir .\platform-tools -DryRun
   ```
5. **Flash & provision:**
   ```powershell
   powershell -ExecutionPolicy Bypass -File .\installer\flash-new-pod.ps1 `
     -PodName pod-02 `
     -ImagesDir .\out\images `
     -PlatformToolsDir .\platform-tools `
     -ApksDir .\out\tv-apks `
     -WifiSsid "YOUR_SSID" `
     -WifiPassword "YOUR_PASSWORD"
   ```
6. **Reassemble & boot:** Unplug power and USB-C, reseat the PCB onto the heatsink with the **6 PCB screws**, reinstall the bottom cover with the **4 case screws** and **4 rubber feet**, and connect **`HDMI 1` (rightmost rear HDMI)** to your TV.

The installer never touches `persist`, `modem`, or `bt_firmware`, preserving each Pod's unique factory MAC addresses and calibration.
