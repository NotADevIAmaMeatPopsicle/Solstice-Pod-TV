# Safety & Non-Brick Rules

The Mersive Solstice Pod Gen3 (`SP8096`) is a sealed appliance with **no external fastboot/reset pinhole**. If a flashed `boot` image hangs before `adbd` starts, recovering fastboot requires disassembling the enclosure to hold the internal PCB Volume-Down button during power-on.

Follow these rules on every build:

1. **Back up stock partitions first:** Before modifying any Pod, back up `boot`, `recovery`, `persist`, `modem`, and `bluetooth` to local private storage. Never overwrite `persist`, `modem`, or `bluetooth` with another unit's image.
2. **Enforce the bootloader kernel ceiling (`< 0x2000000`):** The APQ8096 Little Kernel (`aboot`) loads the uncompressed kernel + DTB below `0x2000000` (32 MB). Always run:
   - `kernel/scripts/check-image-layout.py`
   - `kernel/scripts/verify-packed-diagnostic.py`
   before booting or flashing any new `boot.img`.
3. **ALWAYS RAM-boot first (`fastboot boot`):** Never `fastboot flash boot` an untested kernel or DTB. Confirm it boots to `adb root` via `fastboot boot` first — if a RAM-booted kernel panics, power-cycling automatically returns to the known-good `boot` partition on eMMC.
4. **NEVER put untested `insmod` commands in `on boot`:** Test new `.ko` modules live from `/data/local/tmp/` via `adb root` while streaming `dmesg -w`. Only automate verified modules inside `on property:sys.boot_completed=1` so `adbd` is always reachable.
5. **Keep `watchdog_v2.enable=0` only during active bring-up:** Once all remaining drivers (3.5 mm audio / HDMI-in) are finalized, re-enable `msm_watchdog_v2` for production stability.
