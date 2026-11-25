# Flashing a Solstice Pod Gen3 (SP8096)

## Prerequisites
1. **Android Platform Tools** (`adb.exe`, `fastboot.exe`) and the **Google USB Driver** (`Android Bootloader Interface` bound to `USB\VID_18D1&PID_D00D` in Windows Device Manager).
2. **Prebuilt images** (built via `kernel/scripts/` and `system/scripts/` from official LineageOS 22.2 `h850` + `gemini` donors):
   - `pod44-los22-boot-signed.img`
   - `pod44-los22-recovery-signed.img`
   - `pod-los22-system.simg`
3. **Optional TV APKs** placed in `tv-apks/` (`tvOS-Launcher`, `FLauncher`, `SmartTube`, `F-Droid` — downloaded from their official releases).

## Steps (~5–8 Minutes per Unit)
1. **Enter fastboot:** Open the enclosure and hold the internal **`S3` (`VOL-`)** tactile button on the PCB during power-on (or run `adb reboot bootloader` if ADB is already available).
2. **Back up stock partitions** (`boot`, `recovery`, `persist`, `modem`, `bluetooth`) to private local storage.
3. **Dry-run verify** your image paths:
   ```powershell
   powershell -ExecutionPolicy Bypass -File .\installer\flash-new-pod.ps1 -ImagesDir .\out\images -PlatformToolsDir .\platform-tools -DryRun
   ```
4. **Flash & provision:**
   ```powershell
   powershell -ExecutionPolicy Bypass -File .\installer\flash-new-pod.ps1 `
     -PodName pod-02 `
     -ImagesDir .\out\images `
     -PlatformToolsDir .\platform-tools `
     -ApksDir .\out\tv-apks `
     -WifiSsid "YOUR_SSID" `
     -WifiPassword "YOUR_PASSWORD"
   ```

The installer never touches `persist`, `modem`, or `bt_firmware`, preserving each Pod's unique factory MAC addresses and calibration.
