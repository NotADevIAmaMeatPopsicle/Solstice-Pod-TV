#!/usr/bin/env bash
set -euo pipefail

POD_DONOR_DIR="${POD_DONOR_DIR:-$PWD/work/h850-donor}"
POD_OUT_DIR="${POD_OUT_DIR:-$PWD/out/images}"
POD_STAGING_DIR="${POD_STAGING_DIR:-$PWD/out/staging}"
IMG="${POD_SYSTEM_IMG:-$POD_DONOR_DIR/pod-los22-system.img}"
MNT="${POD_SYSTEM_MNT:-/mnt/pod-los22-sys}"
mkdir -p "$MNT" "$POD_OUT_DIR"

cleanup() {
  if mountpoint -q "$MNT"; then
    umount "$MNT" || true
  fi
}
trap cleanup EXIT

mount -o loop,rw "$IMG" "$MNT"
VDIR="$MNT/system/vendor"
if [ ! -d "$VDIR" ]; then
  VDIR="$MNT/vendor"
fi
echo "Patching vendor dir: $VDIR"

# 1. Remove phone-only HALs from vendor/etc/vintf/manifest.xml cleanly via Python XML parser
python3 - "$VDIR/etc/vintf/manifest.xml" <<'PYEOF'
import sys, xml.etree.ElementTree as ET

path = sys.argv[1]
drop = {
    "android.hardware.light",
    "android.hardware.nfc",
    "android.hardware.ir",
    "android.hardware.camera.provider",
}
tree = ET.parse(path)
root = tree.getroot()
removed = []
for hal in list(root.findall("hal")):
    name_el = hal.find("name")
    if name_el is not None and name_el.text and name_el.text.strip() in drop:
        removed.append(name_el.text.strip())
        root.remove(hal)
tree.write(path, encoding="utf-8", xml_declaration=True)
print("Removed from manifest.xml:", removed)
PYEOF

# 2. Remove phone-only VINTF fragment XMLs and init .rc files
rm -f "$VDIR/etc/vintf/manifest/android.hardware.biometrics.fingerprint@2.0-service.xml"
rm -f "$VDIR/etc/vintf/manifest/vendor.qti.hardware.vibrator.service.xml"
rm -f "$VDIR/etc/vintf/manifest/vendor.lineage.health-service.default.xml"
rm -f "$VDIR/etc/init/android.hardware.light@2.0-service.alice.rc"
rm -f "$VDIR"/etc/init/*lineage.health*.rc
rm -f "$VDIR"/etc/init/*fingerprint*.rc
rm -f "$VDIR"/etc/init/*vibrator*.rc
rm -f "$VDIR"/etc/init/*nfc*.rc
rm -f "$VDIR"/etc/init/*camera*.rc
rm -f "$VDIR"/etc/init/*ir*.rc
rm -f "$VDIR/etc/permissions/android.hardware.fingerprint.xml"
rm -f "$VDIR"/etc/permissions/*nfc*.xml
rm -f "$VDIR/etc/permissions/android.hardware.consumerir.xml"
rm -f "$VDIR"/etc/permissions/android.hardware.camera*.xml
rm -f "$VDIR/etc/permissions/com.nxp.mifare.xml"

# 3. Set TV-friendly 1080p density (320 dpi = xhdpi TV standard) & fix Pod FRP partition path
for propfile in $(find "$MNT" -maxdepth 4 \( -name "*.prop" -o -name "prop.default" -o -name "default.prop" \) 2>/dev/null); do
  if [ -f "$propfile" ] && [ ! -L "$propfile" ]; then
    sed -i '/^ro\.sf\.lcd_density=/d' "$propfile"
    if grep -q '^ro\.frp\.pst=' "$propfile" 2>/dev/null; then
      echo "Updating ro.frp.pst in $propfile"
      sed -i 's|^ro\.frp\.pst=.*|ro.frp.pst=/dev/block/bootdevice/by-name/frp|' "$propfile"
    fi
  fi
done
for rcfile in $(find "$VDIR/etc/init" -name "*.rc" 2>/dev/null); do
  if grep -q 'ro\.frp\.pst' "$rcfile" 2>/dev/null; then
    echo "Updating ro.frp.pst in $rcfile"
    sed -i 's|setprop ro\.frp\.pst .*|setprop ro.frp.pst /dev/block/bootdevice/by-name/frp|' "$rcfile"
  fi
done
echo "ro.sf.lcd_density=320" >> "$VDIR/build.prop"
if ! grep -rq '^ro\.frp\.pst=' "$VDIR/build.prop" 2>/dev/null; then
  echo "ro.frp.pst=/dev/block/bootdevice/by-name/frp" >> "$VDIR/build.prop"
fi

# 4. Grant system:system 0660 on Pod eMMC FRP partition (mmcblk0p45) so PersistentDataBlockService succeeds
if [ -f "$VDIR/etc/ueventd.rc" ] && ! grep -q "by-name/frp" "$VDIR/etc/ueventd.rc"; then
  echo "Adding FRP ueventd rule to $VDIR/etc/ueventd.rc"
  printf "\n/dev/block/platform/soc/7464900.sdhci/by-name/frp 0660 system system\n/dev/block/mmcblk0p45 0660 system system\n" >> "$VDIR/etc/ueventd.rc"
fi

# 5. Disable crash-looping phone-only daemons (qcamerasvr, netmgrd, sensors)
rm -f "$VDIR/etc/init/netmgrd.rc"
python3 - "$VDIR/etc/init/hw/init.qcom.rc" "$VDIR/etc/init/hw/init.target.rc" <<'PYEOF'
import sys, re

targets = {"vendor.sensors", "vendor.qcamerasvr"}
for path in sys.argv[1:]:
    try:
        lines = open(path, "r", encoding="utf-8", errors="ignore").readlines()
    except FileNotFoundError:
        continue
    out = []
    in_target = False
    has_disabled = False
    for line in lines:
        m = re.match(r'^service\s+(\S+)\s+', line)
        if m:
            if in_target and not has_disabled:
                out.append("    disabled\n")
            in_target = m.group(1) in targets
            has_disabled = False
            out.append(line)
            continue
        if in_target:
            if line.strip() == "disabled":
                has_disabled = True
            elif line.strip() == "" or (not line.startswith(" ") and not line.startswith("\t")):
                if not has_disabled:
                    out.append("    disabled\n")
                in_target = False
        out.append(line)
    if in_target and not has_disabled:
        out.append("    disabled\n")
    open(path, "w", encoding="utf-8").writelines(out)
    print("Patched crash-looping services in", path)
PYEOF

# 6. Route default Android speaker audio to Pod HDMI 1 (ADV7533 via SEC_MI2S_RX)
python3 - "$VDIR/etc/mixer_paths_tasha.xml" "$VDIR/etc/audio_platform_info.xml" <<'PYEOF'
import sys, re

mixer_path, plat_path = sys.argv[1], sys.argv[2]
txt = open(mixer_path, "r", encoding="utf-8", errors="ignore").read()
# Replace SLIMBUS_0_RX Audio Mixer MultiMedia* with SEC_MI2S_RX Audio Mixer MultiMedia*
txt = re.sub(r'SLIMBUS_0_RX Audio Mixer MultiMedia', 'SEC_MI2S_RX Audio Mixer MultiMedia', txt)
# Ensure SEC_MI2S_RX defaults exist at top of mixer_paths_tasha.xml
if 'SEC_MI2S_RX Channels' not in txt:
    txt = txt.replace(
        '<mixer>',
        '<mixer>\n    <ctl name="SEC_MI2S_RX Channels" value="Two" />\n'
        '    <ctl name="SEC_MI2S_RX SampleRate" value="KHZ_48" />\n'
        '    <ctl name="SEC_MI2S_RX Format" value="S16_LE" />',
        1,
    )
# Replace <path name="speaker">, <path name="speaker-protected">, <path name="speaker-db"> with SEC_MI2S_RX
sec_mi2s_body = (
    '\n        <ctl name="SEC_MI2S_RX Channels" value="Two" />\n'
    '        <ctl name="SEC_MI2S_RX SampleRate" value="KHZ_48" />\n'
    '        <ctl name="SEC_MI2S_RX Format" value="S16_LE" />\n    '
)
for pname in ("speaker", "speaker-protected", "speaker-db"):
    txt = re.sub(
        rf'<path name="{pname}">.*?</path>',
        f'<path name="{pname}">{sec_mi2s_body}</path>',
        txt,
        count=1,
        flags=re.S,
    )
# Also add alias paths for "* speaker" just in case any backend appends " speaker"
if 'name="deep-buffer-playback speaker"' not in txt:
    aliases = (
        '\n    <path name="deep-buffer-playback speaker">\n'
        '        <path name="deep-buffer-playback" />\n'
        '    </path>\n'
        '    <path name="low-latency-playback speaker">\n'
        '        <path name="low-latency-playback" />\n'
        '    </path>\n'
        '    <path name="audio-ull-playback speaker">\n'
        '        <path name="audio-ull-playback" />\n'
        '    </path>\n'
    )
    txt = txt.replace('</mixer>', aliases + '</mixer>', 1)
open(mixer_path, "w", encoding="utf-8").write(txt)
print("Patched SEC_MI2S_RX HDMI 1 audio routing in", mixer_path)

ptxt = open(plat_path, "r", encoding="utf-8", errors="ignore").read()
ptxt = ptxt.replace('<device name="SND_DEVICE_OUT_SPEAKER" bit_width="24"/>',
                    '<device name="SND_DEVICE_OUT_SPEAKER" bit_width="16"/>')
# Remove backend="speaker" override if present so default paths match directly
ptxt = re.sub(r'\s*<device name="SND_DEVICE_OUT_SPEAKER" backend="speaker"[^/]*/>', '', ptxt)
# Match Pod 01 stock audio_platform_info.xml: bind HEADPHONES and LINE explicitly to SLIMBUS_6_RX
ptxt = ptxt.replace('<device name="SND_DEVICE_OUT_HEADPHONES" backend="headphones"/>',
                    '<device name="SND_DEVICE_OUT_HEADPHONES" backend="headphones" interface="SLIMBUS_6_RX"/>')
ptxt = ptxt.replace('<device name="SND_DEVICE_OUT_LINE" backend="headphones"/>',
                    '<device name="SND_DEVICE_OUT_LINE" backend="headphones" interface="SLIMBUS_6_RX"/>')
ptxt = ptxt.replace('<device name="SND_DEVICE_OUT_VOICE_HEADPHONES" backend="headphones"/>',
                    '<device name="SND_DEVICE_OUT_VOICE_HEADPHONES" backend="headphones" interface="SLIMBUS_6_RX"/>')
ptxt = ptxt.replace('<device name="SND_DEVICE_OUT_VOICE_LINE" backend="headphones"/>',
                    '<device name="SND_DEVICE_OUT_VOICE_LINE" backend="headphones" interface="SLIMBUS_6_RX"/>')
open(plat_path, "w", encoding="utf-8").write(ptxt)
print("Patched SND_DEVICE_OUT_SPEAKER + SLIMBUS_6_RX headphones/line in", plat_path)
PYEOF

# Disable phone speaker thermal protection (vi-feedback)
for pf in "$VDIR/build.prop" "$MNT/system/build.prop"; do
  if [ -f "$pf" ]; then
    sed -i '/^persist\.speaker\.prot\.enable=/d' "$pf"
    sed -i '/^persist\.vendor\.audio\.speaker\.prot\.enable=/d' "$pf"
  fi
done
echo "persist.speaker.prot.enable=false" >> "$VDIR/build.prop"
echo "persist.vendor.audio.speaker.prot.enable=false" >> "$VDIR/build.prop"

# Also prevent explicit 'start vendor.sensors' from respawning sensors.qti
sed -i 's/^\([[:space:]]*\)start vendor\.sensors/\1# start vendor.sensors/' "$VDIR/etc/init/hw/init.qcom.rc"

# 7. Install Pod 01's own stock ACDB audio calibration files (private, extracted from Pod backup)
STOCK_ACDB="${POD_STOCK_ACDB:-$POD_DONOR_DIR/pod-stock-acdbdata}"
if [ -d "$STOCK_ACDB" ]; then
  for name in Bluetooth_cal.acdb General_cal.acdb Global_cal.acdb Handset_cal.acdb Hdmi_cal.acdb Headset_cal.acdb Speaker_cal.acdb; do
    if [ -f "$STOCK_ACDB/MTP_$name" ]; then
      cp -f "$STOCK_ACDB/MTP_$name" "$VDIR/etc/acdbdata/$name"
      chmod 0644 "$VDIR/etc/acdbdata/$name"
    fi
  done
  echo "Installed Pod 01 stock ACDB files into $VDIR/etc/acdbdata/"
fi

# 8. Install upstream Linux ath10k_pci Wi-Fi modules + QCA6174 hw3.0 firmware + boot load
ATH_MODS="${POD_ATH_MODS:-$POD_STAGING_DIR/ath10k-mods}"
ATH_FW="${POD_ATH_FW:-$POD_STAGING_DIR/ath10k-fw}"
if [ -d "$ATH_MODS" ]; then
  mkdir -p "$VDIR/lib/modules" "$VDIR/firmware/ath10k/QCA6174/hw3.0"
  rm -f "$VDIR/lib/modules/wlan.ko" "$VDIR/firmware/qwlan30.bin" "$VDIR/firmware/otp30.bin"
  for m in ccm.ko mac80211.ko ath.ko ath10k_core.ko ath10k_pci.ko; do
    if [ -f "$ATH_MODS/$m" ]; then
      cp -f "$ATH_MODS/$m" "$VDIR/lib/modules/$m"
      chmod 0644 "$VDIR/lib/modules/$m"
    fi
  done
  if [ -d "$ATH_FW" ]; then
    for fw in firmware-4.bin board.bin board-2.bin; do
      if [ -f "$ATH_FW/$fw" ]; then
        cp -f "$ATH_FW/$fw" "$VDIR/firmware/ath10k/QCA6174/hw3.0/$fw"
        chmod 0644 "$VDIR/firmware/ath10k/QCA6174/hw3.0/$fw"
      fi
    done
    echo "Installed ath10k QCA6174 hw3.0 firmware into $VDIR/firmware/ath10k/QCA6174/hw3.0/"
  fi
  cat > "$VDIR/etc/init/init.pod.wifi.rc" <<'RCEOF'
on boot
    # Enable MER6641 HDMI OUT1 (4K native hdmi_tx / fb1) via TLMM GPIO 127
    write /sys/class/gpio/export 127
    write /sys/class/gpio/gpio127/direction out
    write /sys/class/gpio/gpio127/value 1

    write /sys/bus/msm_subsys/devices/subsys0/restart_level related
    write /sys/bus/msm_subsys/devices/subsys1/restart_level related
    write /sys/bus/msm_subsys/devices/subsys2/restart_level related
    write /sys/bus/msm_subsys/devices/subsys3/restart_level related
    write /sys/bus/msm_subsys/devices/subsys4/restart_level related
    write /sys/bus/msm_subsys/devices/subsys5/restart_level related

    # Load Qualcomm QCA6174A Bluetooth power/rfkill driver before HAL starts
    insmod /vendor/lib/modules/bluetooth-power.ko
    chown bluetooth bluetooth /dev/btpower
    chmod 0660 /dev/btpower
    chown bluetooth net_bt /dev/ttyHS0
    chmod 0660 /dev/ttyHS0
    chown bluetooth bluetooth /sys/module/bluetooth_power/parameters/power
    chmod 0660 /sys/module/bluetooth_power/parameters/power
    chown bluetooth net_bt /sys/class/rfkill/rfkill0/type
    chown bluetooth net_bt /sys/class/rfkill/rfkill0/state
    chown bluetooth net_bt /sys/class/rfkill/rfkill0/device/extldo
    chmod 0660 /sys/class/rfkill/rfkill0/state
    chmod 0660 /sys/class/rfkill/rfkill0/device/extldo
    chown bluetooth net_bt /sys/class/rfkill/rfkill1/type
    chown bluetooth net_bt /sys/class/rfkill/rfkill1/state
    chown bluetooth net_bt /sys/class/rfkill/rfkill1/device/extldo
    chmod 0660 /sys/class/rfkill/rfkill1/state
    chmod 0660 /sys/class/rfkill/rfkill1/device/extldo

on property:sys.boot_completed=1
    exec_background u:r:shell:s0 root root -- /system/bin/sh -c "sleep 5; /system/bin/cmd display enable-display 2"
    exec_background u:r:shell:s0 root root -- /system/bin/sh -c "sleep 2; insmod /vendor/lib/modules/ccm.ko; insmod /vendor/lib/modules/mac80211.ko; insmod /vendor/lib/modules/ath.ko; insmod /vendor/lib/modules/ath10k_core.ko; insmod /vendor/lib/modules/ath10k_pci.ko irq_mode=2 reset_mode=1; insmod /vendor/lib/modules/wlan.ko; chown wifi:wifi /sys/module/wlan/parameters/fwpath; chmod 0666 /sys/module/wlan/parameters/fwpath; sleep 3; /system/bin/svc wifi enable"
RCEOF
  chmod 0644 "$VDIR/etc/init/init.pod.wifi.rc"

  # Replace LG G5 Broadcom Wi-Fi HAL with official LineageOS 22.2 Qualcomm QCA6174A Wi-Fi HAL (gemini)
  QCA_HAL="${POD_WIFI_HAL_OUT:-$POD_DONOR_DIR/gemini-qca-wifi-hal}"
  if [ -d "$QCA_HAL" ]; then
    cp -f "$QCA_HAL/libwifi-hal.so" "$VDIR/lib64/libwifi-hal.so"
    chmod 0644 "$VDIR/lib64/libwifi-hal.so"
    cp -f "$QCA_HAL/android.hardware.wifi-service" "$VDIR/bin/hw/android.hardware.wifi-service"
    chmod 0755 "$VDIR/bin/hw/android.hardware.wifi-service"
    if [ -f "$QCA_HAL/libcld80211.so" ]; then
      cp -f "$QCA_HAL/libcld80211.so" "$VDIR/lib64/libcld80211.so"
      chmod 0644 "$VDIR/lib64/libcld80211.so"
    fi
    if [ -d "$QCA_HAL/wifi" ]; then
      rm -f "$VDIR/etc/wifi/bcmdhd.cal"
      cp -f "$QCA_HAL/wifi/"*.conf "$VDIR/etc/wifi/" 2>/dev/null || true
    fi
    echo "Installed official LineageOS 22.2 Qualcomm QCA6174A Wi-Fi HAL (gemini)"
  fi
  echo "Installed upstream ath10k_pci modules + firmware + init.pod.wifi.rc"
fi

# 9. Install Pod 01 QCA6174A Bluetooth (bluetooth-power.ko + /bt_firmware mount + gemini QTI BT HAL)
BT_MOD="${POD_BT_MOD:-$POD_STAGING_DIR/bluetooth-power.ko}"
QCA_BT_HAL="${POD_BT_HAL_OUT:-$POD_DONOR_DIR/gemini-qca-bt-hal}"
if [ -f "$BT_MOD" ]; then
  mkdir -p "$VDIR/lib/modules"
  cp -f "$BT_MOD" "$VDIR/lib/modules/bluetooth-power.ko"
  chmod 0644 "$VDIR/lib/modules/bluetooth-power.ko"
  echo "Installed bluetooth-power.ko"
fi
mkdir -p "$MNT/bt_firmware"
for fstab in "$VDIR/etc/fstab"*; do
  if [ -f "$fstab" ] && ! grep -q "/bt_firmware" "$fstab"; then
    echo "/dev/block/bootdevice/by-name/bluetooth               /bt_firmware      vfat    ro,shortname=lower,uid=1000,gid=1000,dmask=222,fmask=333,context=u:object_r:bt_firmware_file:s0               wait" >> "$fstab"
    echo "Added /bt_firmware mount to $fstab"
  fi
done
if [ -f "$VDIR/etc/ueventd.rc" ] && ! grep -q "/dev/btpower" "$VDIR/etc/ueventd.rc"; then
  echo "/dev/btpower              0660   bluetooth  bluetooth" >> "$VDIR/etc/ueventd.rc"
fi
if [ -d "$QCA_BT_HAL" ]; then
  rm -f "$VDIR/etc/init/android.hardware.bluetooth@1.0-service.rc" \
        "$VDIR/bin/hw/android.hardware.bluetooth@1.0-service" \
        "$VDIR/lib64/hw/android.hardware.bluetooth@1.0-impl.so"
  cp -f "$QCA_BT_HAL/bin/hw/android.hardware.bluetooth@1.0-service-qti" "$VDIR/bin/hw/"
  chmod 0755 "$VDIR/bin/hw/android.hardware.bluetooth@1.0-service-qti"
  cp -f "$QCA_BT_HAL/lib64/hw/android.hardware.bluetooth@1.0-impl-qti.so" "$VDIR/lib64/hw/"
  chmod 0644 "$VDIR/lib64/hw/android.hardware.bluetooth@1.0-impl-qti.so"
  cp -f "$QCA_BT_HAL/etc/init/android.hardware.bluetooth@1.0-service-qti.rc" "$VDIR/etc/init/"
  chmod 0644 "$VDIR/etc/init/android.hardware.bluetooth@1.0-service-qti.rc"
  for lib in libbtnv.so libbase_shim.so libhidlbase-v32.so vendor.qti.hardware.fm@1.0_vendor.so com.qualcomm.qti.ant@1.0_vendor.so libbt-hidlclient.so; do
    if [ -f "$QCA_BT_HAL/lib64/$lib" ]; then
      cp -f "$QCA_BT_HAL/lib64/$lib" "$VDIR/lib64/$lib"
      chmod 0644 "$VDIR/lib64/$lib"
    fi
  done
  if [ ! -f "$VDIR/lib64/libdiag.so" ] && [ -f "$QCA_BT_HAL/lib64/libdiag.so" ]; then
    cp -f "$QCA_BT_HAL/lib64/libdiag.so" "$VDIR/lib64/libdiag.so"
    chmod 0644 "$VDIR/lib64/libdiag.so"
  fi
  sed -i '/^qcom\.bluetooth\.soc=/d' "$VDIR/build.prop"
  sed -i '/^persist\.vendor\.qcom\.bluetooth\.soc=/d' "$VDIR/build.prop"
  sed -i '/^vendor\.qcom\.bluetooth\.soc=/d' "$VDIR/build.prop"
  sed -i '/^bluetooth\.device\.default_name=/d' "$VDIR/build.prop"
  {
    echo "qcom.bluetooth.soc=rome"
    echo "persist.vendor.qcom.bluetooth.soc=rome"
    echo "vendor.qcom.bluetooth.soc=rome"
    echo "bluetooth.device.default_name=Solstice Pod"
  } >> "$VDIR/build.prop"
  echo "Installed official LineageOS 22.2 Qualcomm QCA6174A Bluetooth HAL (gemini)"
fi

sync
umount "$MNT"
e2fsck -fy "$IMG" || true

echo "Converting updated system.img to sparse $POD_OUT_DIR/pod-los22-system.simg..."
img2simg "$IMG" "$POD_OUT_DIR/pod-los22-system.simg"
ls -lh "$POD_OUT_DIR/pod-los22-system.simg"
sha256sum "$POD_OUT_DIR/pod-los22-system.simg"
echo "=== DONE ==="
