# Audio (3.5 mm Jack) & HDMI Input (`TC358840`) Roadmap

## 1. 3.5 mm Audio Jack (`J6 AUDIO`, WCD9335)
- **Current state:** Left channel works; the `SLIMBUS_6_RX` patch in `audio_platform_info.xml` (`SND_DEVICE_OUT_HEADPHONES`, `SND_DEVICE_OUT_LINE`, etc.) is staged and flashed.
- **Next verification step:** Test stereo headphones on `J6 AUDIO`. If the right channel is still silent, inspect live WCD9335 ALSA mixer controls (`HPHL`/`HPHR`, `RX1`/`RX2`, `COMP1`/`COMP2`, `SLIM RX2`/`RX3`, `RX INT2_2 MUX`) against the Pod's stock `mixer_paths_tasha.xml`.

## 2. HDMI Input (Toshiba `TC358840`)
- **Hardware wiring (confirmed in `kernel/dts/apq8096-v3-sbc-pod.dts`):**
  - Dual 4-lane MIPI CSI-2 bridge wired to APQ8096 `&cci` as `qcom,camera@0` (`csiphy-sd-index = <0>`, `csid-sd-index = <0>`) and `qcom,camera@1` (`csiphy-sd-index = <2>`, `csid-sd-index = <2>`).
  - GPIOs: MCLK0 (`gpio13`), RESET0 (`gpio30`), STANDBY0 (`gpio29`), HPD control (`gpio22`), interrupt (`gpio21`).
- **Constraints on Linux 4.4.302 + Android 15:**
  - `CONFIG_MSMB_CAMERA`, `CONFIG_MSM_CCI`, `CONFIG_MSM_CAMERA_SENSOR`, and `CONFIG_MSM_CPP` are `bool` (built-in only, not `=m`), and the current uncompressed `Image` has ~352 KB (`0x58000`) headroom below `0x2000000`.
  - No `tc358840` driver ships in `android_kernel_lge_msm8996`; stock Android 7.1.1 used Qualcomm's legacy `mm-qcamera-daemon` stack.
- **Planned paths:**
  1. **Native `TC358840` bring-up:** Trim unused built-ins (e.g. `ipa_v2` / `msm_11ad`) to reclaim space under `0x2000000`, enable minimal `MSMB_CAMERA` + `CCI` + `CSIPHY`/`CSID`/`VFE`, port upstream `tc358840` V4L2 support, and expose `/dev/video*` via Android 15's standard external V4L2 camera provider HAL.
  2. **Zero-kernel-growth alternative:** Build `CONFIG_USB_VIDEO_CLASS=m` as a loadable module and use a USB 3.0 UVC HDMI capture dongle on the Pod's USB-A port.
