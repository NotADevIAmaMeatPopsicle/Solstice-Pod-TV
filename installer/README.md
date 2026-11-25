# Flashing a Solstice Pod Gen3 (SP8096)

## Prerequisites
1. **Android Platform Tools** (`adb.exe`, `fastboot.exe`) and the **Google USB Driver** (`Android Bootloader Interface` bound to `USB\VID_18D1&PID_D00D` in Windows Device Manager).
2. **Prebuilt images** (built via `kernel/scripts/` and `system/scripts/` from official LineageOS 22.2 `h850` + `gemini` donors):
   - `pod44-los22-boot-signed.img`
   - `pod44-los22-recovery-signed.img`
   - `pod-los22-system.simg`
3. **Optional TV APKs** placed in `tv-apks/` (**Projectivy Launcher**, `FLauncher`, `SmartTube`, `F-Droid`, downloaded from their official releases).

## Steps (~5–10 Minutes per Unit)
1. **Open the case and pull the board out:** Unplug power and any cables, flip the Pod over, peel back the **4 rubber feet**, take out the 4 case screws underneath, and lift off the bottom cover. You'll see **6 screws** holding the board in place (4 in the corners and 2 in the middle). Take those out and the board lifts right out. The aluminum heatsink block isn't bolted to the board (it's only held on with a light layer of thermal paste and can easily slip off), so just set it aside so you can see and reach the buttons on top.
2. **Enter fastboot:** Plug a **USB-C data cable** into the Pod's **front USB-C port** and connect it to your PC. **Press and hold `VOL-` (`S3`)** on the board, plug in **12 V power** (or PoE+), keep holding it for **about 3–5 seconds**, and let go. Check that it shows up with `fastboot devices` (or run `adb reboot bootloader` if ADB is already working).
3. **Back up stock partitions** (`boot`, `recovery`, `persist`, `modem`, `bluetooth`) to local storage.
4. **Test your image paths** with `-DryRun`:
   ```powershell
   powershell -ExecutionPolicy Bypass -File .\installer\flash-new-pod.ps1 -ImagesDir .\out\images -PlatformToolsDir .\platform-tools -DryRun
   ```
5. **Flash Android 15:**
   ```powershell
   powershell -ExecutionPolicy Bypass -File .\installer\flash-new-pod.ps1 `
     -PodName pod-02 `
     -ImagesDir .\out\images `
     -PlatformToolsDir .\platform-tools `
     -ApksDir .\out\tv-apks `
     -WifiSsid "YOUR_SSID" `
     -WifiPassword "YOUR_PASSWORD"
   ```
6. **Put it back together and boot:** Unplug power and USB-C, set the aluminum heatsink block back onto the board, put the **6 board screws** back in, reinstall the bottom cover with the **4 case screws** and **4 rubber feet**, and plug **`HDMI 1` (rightmost rear HDMI)** into your TV.

The installer never touches `persist`, `modem`, or `bt_firmware`, preserving each Pod's unique factory MAC addresses and calibration.
