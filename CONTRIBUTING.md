# Contributing & Safety Contract

Before opening a PR or testing changes on a reassembled Solstice Pod Gen3 (`SP8096`), follow these rules:

1. **RAM-boot first, always:** Never `fastboot flash boot` an untested kernel. Verify boot-to-ADB via `fastboot boot` first.
2. **Enforce the bootloader image ceiling:** Run `kernel/scripts/check-image-layout.py` (uncompressed kernel extent must remain `< 0x2000000`) and `kernel/scripts/verify-packed-diagnostic.py` before booting or flashing any new boot image.
3. **Never put untested `insmod` commands in `on boot`:** Test new `.ko` modules live from `/data/local/tmp/` via `adb root`. Only automate verified modules in `on property:sys.boot_completed=1` so `adbd` is always reachable if a module faults.
4. **Never commit private or proprietary data:** Keep `pod-backups/`, raw partition dumps, firmware blobs, APKs, device serials, MAC addresses, and Wi-Fi credentials out of git.
5. **Document real hardware tests:** Only mark a subsystem "Working" after a verified live hardware test (including cold self-boot from eMMC for persistent changes).
