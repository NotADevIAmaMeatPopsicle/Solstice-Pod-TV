# Audio (3.5 mm Jack) & HDMI Input (`TC358840`) Roadmap

## 1. 3.5 mm Audio Jack (`J6 AUDIO`, WCD9335)
- **Current state:** **Working (stereo L+R verified).**
- **Fix applied:** Routed `SND_DEVICE_OUT_HEADPHONES` / `SND_DEVICE_OUT_LINE` to `SLIMBUS_6_RX` in `audio_platform_info.xml` and configured `SLIM RX2 MUX` / `SLIM RX3 MUX` (`AIF4_PB`, `SLIM_6_RX Channels = Two`, `RX INT1_2 MUX = RX2`, `RX INT2_2 MUX = RX3`) in `mixer_paths_tasha.xml`. Verified with a 48 kHz L/R stereo tone test (440 Hz left, 660 Hz right, 523 Hz both).

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
