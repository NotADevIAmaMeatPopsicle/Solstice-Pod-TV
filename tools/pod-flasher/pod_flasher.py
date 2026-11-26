#!/usr/bin/env python3
"""
Solstice Pod TV Flasher & Manager (Cross-Platform Web UI)
Runs on Windows, macOS, and Linux using Python 3 standard library only.
"""

import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import urllib.parse
import webbrowser
from datetime import datetime
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

APP_DIR = Path(__file__).resolve().parent
REPO_ROOT = APP_DIR.parent.parent
DEFAULT_PORT = 8765

SAFE_RESTORE_PARTITIONS = ["boot", "recovery", "system"]
SENSITIVE_PARTITIONS = ["persist", "modem", "bluetooth", "bt_firmware"]
DEFAULT_BACKUP_PARTITIONS = ["boot", "recovery", "persist", "modem", "bluetooth"]
DISABLE_PHONE_PKGS = [
    "com.android.dialer",
    "com.android.contacts",
    "com.android.messaging",
    "com.android.calculator2",
    "com.android.deskclock",
    "org.lineageos.aperture",
    "org.lineageos.etar",
    "org.lineageos.recorder",
    "org.lineageos.glimpse",
    "org.lineageos.twelve",
    "com.mersive.solstice.webapp",
    "com.splashtop.m360.mersive",
]


def is_windows() -> bool:
    return platform.system().lower() == "windows"


def find_platform_tools(configured_dir: str = "") -> tuple[str | None, str | None, str]:
    """Locate adb and fastboot across Windows, macOS, and Linux.
    On Windows, stages a space-free temp copy if the directory path contains spaces
    (fixes fastboot format failing with CreateProcess error 193)."""
    adb_name = "adb.exe" if is_windows() else "adb"
    fb_name = "fastboot.exe" if is_windows() else "fastboot"

    candidates = []
    if configured_dir:
        candidates.append(Path(configured_dir).expanduser())
    candidates.extend([
        REPO_ROOT / "platform-tools",
        APP_DIR / "platform-tools",
        Path.home() / "platform-tools",
        Path("/opt/homebrew/bin"),
        Path("/usr/local/bin"),
        Path("/usr/bin"),
    ])

    adb_path = None
    fb_path = None
    note = ""

    for d in candidates:
        if d.is_dir():
            a = d / adb_name
            f = d / fb_name
            if a.is_file() and f.is_file():
                adb_path = str(a)
                fb_path = str(f)
                break

    if not adb_path:
        adb_path = shutil.which(adb_name)
    if not fb_path:
        fb_path = shutil.which(fb_name)

    if is_windows() and fb_path and (" " in fb_path):
        src_dir = Path(fb_path).parent
        dst_dir = Path(tempfile.gettempdir()) / "sp8096-platform-tools"
        try:
            if not (dst_dir / fb_name).is_file():
                shutil.copytree(src_dir, dst_dir, dirs_exist_ok=True)
            adb_path = str(dst_dir / adb_name)
            fb_path = str(dst_dir / fb_name)
            note = f"Staged space-free platform-tools to {dst_dir}"
        except Exception as e:
            note = f"Warning: could not stage space-free platform-tools: {e}"

    return adb_path, fb_path, note


def run_cmd(args: list[str], timeout: int = 30) -> tuple[int, str]:
    try:
        creationflags = 0x08000000 if is_windows() else 0  # CREATE_NO_WINDOW
        p = subprocess.run(
            args,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
            creationflags=creationflags,
        )
        return p.returncode, p.stdout.strip()
    except subprocess.TimeoutExpired:
        return 124, f"Command timed out after {timeout}s"
    except Exception as e:
        return 1, str(e)


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


class JobManager:
    def __init__(self):
        self.lock = threading.Lock()
        self.state_file = REPO_ROOT / "flash-state.json"
        self.config_file = APP_DIR / "config.json"
        self.config = self._load_config()
        self.flash_state = self._load_state()
        self.job = {
            "running": False,
            "kind": "",
            "serial": "",
            "step": "Idle",
            "progress": 0,
            "status": "idle",  # idle | running | done | error
            "error": "",
            "checks": [],
            "logs": [],
            "started_at": None,
            "finished_at": None,
        }
        self._cancel = threading.Event()

    def _load_config(self) -> dict:
        default = {
            "platform_tools_dir": "",
            "images_dir": str(REPO_ROOT / "out" / "images"),
            "apks_dir": str(REPO_ROOT / "out" / "tv-apks"),
            "backups_dir": str(REPO_ROOT / "backups"),
            "wifi_ssid": "",
            "wifi_password": "",
        }
        if self.config_file.is_file():
            try:
                default.update(json.loads(self.config_file.read_text(encoding="utf-8")))
            except Exception:
                pass
        return default

    def save_config(self, updates: dict) -> dict:
        with self.lock:
            for k in ("platform_tools_dir", "images_dir", "apks_dir", "backups_dir", "wifi_ssid", "wifi_password"):
                if k in updates:
                    self.config[k] = str(updates[k]).strip()
            self.config_file.write_text(json.dumps(self.config, indent=2), encoding="utf-8")
            return dict(self.config)

    def _load_state(self) -> dict:
        if self.state_file.is_file():
            try:
                return json.loads(self.state_file.read_text(encoding="utf-8"))
            except Exception:
                pass
        return {}

    def _save_state(self):
        try:
            self.state_file.write_text(json.dumps(self.flash_state, indent=2), encoding="utf-8")
        except Exception:
            pass

    def log(self, msg: str, level: str = "info"):
        ts = datetime.now().strftime("%H:%M:%S")
        with self.lock:
            self.job["logs"].append({"ts": ts, "level": level, "msg": msg})
            if len(self.job["logs"]) > 500:
                self.job["logs"] = self.job["logs"][-500:]

    def set_progress(self, pct: int, step: str):
        with self.lock:
            self.job["progress"] = max(0, min(100, pct))
            self.job["step"] = step

    def set_checks(self, checks: list[dict]):
        with self.lock:
            self.job["checks"] = checks

    def get_tools(self):
        return find_platform_tools(self.config.get("platform_tools_dir", ""))

    def detect_devices(self) -> dict:
        adb, fb, note = self.get_tools()
        devices = []
        driver_warnings = []

        if fb:
            rc, out = run_cmd([fb, "devices"], timeout=5)
            if rc == 0:
                for line in out.splitlines():
                    m = re.match(r"^(\S+)\s+fastboot", line.strip())
                    if m:
                        ser = m.group(1)
                        info = {
                            "serial": ser,
                            "mode": "fastboot",
                            "saved_state": self.flash_state.get(ser, ""),
                            "details": {},
                        }
                        _, prod = run_cmd([fb, "-s", ser, "getvar", "product"], timeout=4)
                        pm = re.search(r"product:\s*(\S+)", prod)
                        if pm:
                            info["details"]["product"] = pm.group(1)
                        devices.append(info)

        if adb:
            rc, out = run_cmd([adb, "devices", "-l"], timeout=5)
            if rc == 0:
                for line in out.splitlines()[1:]:
                    parts = line.strip().split()
                    if len(parts) >= 2 and parts[1] in ("device", "recovery", "unauthorized"):
                        ser, st = parts[0], parts[1]
                        info = {
                            "serial": ser,
                            "mode": "android" if st == "device" else st,
                            "saved_state": self.flash_state.get(ser, ""),
                            "details": {},
                        }
                        if st == "device":
                            _, probe = run_cmd(
                                [
                                    adb, "-s", ser, "shell",
                                    "echo K=$(uname -r); "
                                    "echo A=$(getprop ro.build.version.release); "
                                    "echo B=$(getprop sys.boot_completed); "
                                    "echo IP=$(ip -4 addr show wlan0 2>/dev/null | grep -o 'inet [0-9.]*' | awk '{print $2}'); "
                                    "echo CEC=$(pidof pod-cec-daemon 2>/dev/null); "
                                    "echo HP=$(ps -ef 2>/dev/null | grep pod-hdmi-hotplug | grep -v grep | awk '{print $2}' | head -n1); "
                                    "echo HOME=$(cmd role get-role-holders --user 0 android.app.role.HOME 2>/dev/null); "
                                    "echo DF=$(df -h /data 2>/dev/null | tail -n1 | awk '{print $4 \" free of \" $2}')"
                                ],
                                timeout=6,
                            )
                            for kv in probe.splitlines():
                                if "=" in kv:
                                    k, v = kv.split("=", 1)
                                    info["details"][k.strip()] = v.strip()
                        devices.append(info)

        # Windows-only helper: detect fastboot Pods whose USB driver isn't bound yet
        if is_windows():
            rc, out = run_cmd(
                [
                    "powershell", "-NoProfile", "-Command",
                    "Get-PnpDevice -PresentOnly -ErrorAction SilentlyContinue | "
                    "Where-Object { $_.InstanceId -match 'VID_18D1&PID_D00D' -and $_.Status -ne 'OK' } | "
                    "ForEach-Object { $_.InstanceId }"
                ],
                timeout=6,
            )
            if rc == 0 and out:
                for line in out.splitlines():
                    if line.strip():
                        driver_warnings.append(line.strip())

        return {
            "os": platform.system(),
            "adb": adb or "",
            "fastboot": fb or "",
            "tools_note": note,
            "devices": devices,
            "driver_warnings": driver_warnings,
            "flash_state": self.flash_state,
        }

    def list_backups(self) -> list[dict]:
        bdir = Path(self.config.get("backups_dir", str(REPO_ROOT / "backups"))).expanduser()
        items = []
        if not bdir.is_dir():
            return items
        for sub in sorted(bdir.iterdir(), reverse=True):
            if not sub.is_dir():
                continue
            manifest_path = sub / "backup-manifest.json"
            if manifest_path.is_file():
                try:
                    data = json.loads(manifest_path.read_text(encoding="utf-8"))
                    data["path"] = str(sub)
                    data["name"] = sub.name
                    items.append(data)
                    continue
                except Exception:
                    pass
            imgs = [p.name for p in sub.glob("*.img")]
            if imgs:
                items.append({
                    "name": sub.name,
                    "path": str(sub),
                    "serial": sub.name.split("-")[0],
                    "created_at": datetime.fromtimestamp(sub.stat().st_mtime).isoformat(timespec="seconds"),
                    "files": {n: {} for n in imgs},
                })
        return items

    def start_job(self, kind: str, payload: dict) -> tuple[bool, str]:
        with self.lock:
            if self.job["running"]:
                return False, f"Another job ({self.job['kind']}) is already running."
            self._cancel.clear()
            self.job = {
                "running": True,
                "kind": kind,
                "serial": payload.get("serial", ""),
                "step": "Starting...",
                "progress": 1,
                "status": "running",
                "error": "",
                "checks": [],
                "logs": [],
                "started_at": datetime.now().isoformat(timespec="seconds"),
                "finished_at": None,
            }

        target = {
            "install": self._run_install,
            "upgrade": self._run_upgrade,
            "provision": self._run_provision_only,
            "backup": self._run_backup,
            "restore": self._run_restore,
        }.get(kind)

        if not target:
            with self.lock:
                self.job["running"] = False
                self.job["status"] = "error"
                self.job["error"] = f"Unknown job type: {kind}"
            return False, f"Unknown job type: {kind}"

        t = threading.Thread(target=self._wrapper, args=(target, payload), daemon=True)
        t.start()
        return True, "Started"

    def _wrapper(self, fn, payload: dict):
        try:
            fn(payload)
            with self.lock:
                self.job["status"] = "done"
                self.job["progress"] = 100
                self.job["step"] = "Completed"
        except Exception as e:
            self.log(f"ERROR: {e}", "error")
            with self.lock:
                self.job["status"] = "error"
                self.job["error"] = str(e)
                self.job["step"] = "Failed"
        finally:
            with self.lock:
                self.job["running"] = False
                self.job["finished_at"] = datetime.now().isoformat(timespec="seconds")

    def _stream_cmd(self, args: list[str], timeout: int = 600) -> int:
        self.log(f"$ {' '.join(args)}")
        creationflags = 0x08000000 if is_windows() else 0
        p = subprocess.Popen(
            args,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
            errors="replace",
            creationflags=creationflags,
        )
        start = time.time()
        assert p.stdout is not None
        for line in p.stdout:
            txt = line.rstrip()
            if txt and "fetch:super" not in txt and "avb footer" not in txt:
                self.log(txt)
            if time.time() - start > timeout:
                p.kill()
                raise RuntimeError(f"Command timed out after {timeout}s")
        return p.wait()

    def _wait_boot(self, adb: str, serial: str):
        self.set_progress(60, "Waiting for Android 15 boot...")
        self.log("Waiting for ADB device...")
        rc, _ = run_cmd([adb, "-s", serial, "wait-for-device"], timeout=180)
        if rc != 0:
            raise RuntimeError("Timed out waiting for ADB device after reboot")
        for _ in range(120):
            rc, out = run_cmd([adb, "-s", serial, "shell", "getprop sys.boot_completed"], timeout=6)
            if rc == 0 and out.strip() == "1":
                self.log("sys.boot_completed=1")
                time.sleep(4)
                return
            time.sleep(2)
        raise RuntimeError("Timed out waiting for sys.boot_completed=1")

    def _flash_images(self, fb: str, serial: str, wipe_userdata: bool):
        im_dir = Path(self.config["images_dir"]).expanduser()
        boot_img = im_dir / "pod44-los22-boot-signed.img"
        rec_img = im_dir / "pod44-los22-recovery-signed.img"
        sys_img = im_dir / "pod-los22-system.simg"

        for f in (boot_img, rec_img, sys_img):
            if not f.is_file():
                raise RuntimeError(f"Missing image file: {f}")

        steps = [
            (10, "Flashing boot...", ["flash", "boot", str(boot_img)]),
            (20, "Flashing recovery...", ["flash", "recovery", str(rec_img)]),
            (45, "Flashing system (2.3 GB)...", ["flash", "system", str(sys_img)]),
        ]
        if wipe_userdata:
            steps.extend([
                (52, "Formatting userdata (ext4)...", ["format:ext4", "userdata"]),
                (55, "Formatting cache (ext4)...", ["format:ext4", "cache"]),
            ])
        steps.append((58, "Rebooting into Android 15...", ["reboot"]))

        for pct, label, cmd in steps:
            self.set_progress(pct, label)
            rc = self._stream_cmd([fb, "-s", serial] + cmd, timeout=360)
            if rc != 0:
                raise RuntimeError(f"fastboot {' '.join(cmd)} failed (exit {rc})")

    def _provision(self, adb: str, serial: str, install_apps: bool = True):
        self._wait_boot(adb, serial)

        self.set_progress(68, "Checking /vendor services...")
        _, has_cec = run_cmd([adb, "-s", serial, "shell", "ls /vendor/bin/pod-cec-daemon 2>/dev/null"], timeout=6)
        _, has_hp = run_cmd([adb, "-s", serial, "shell", "ls /vendor/bin/pod-hdmi-hotplug.sh 2>/dev/null"], timeout=6)
        has_baked = ("pod-cec-daemon" in has_cec) and ("hotplug" in has_hp)
        if has_baked:
            self.log("Detected baked CEC daemon and HDMI hot-plug service in /vendor.")
        else:
            self.log("Note: /vendor does not have baked CEC/hot-plug files; continuing with TV defaults.", "warn")

        self.set_progress(74, "Applying TV defaults & hiding phone apps...")
        run_cmd([
            adb, "-s", serial, "shell",
            "settings put global device_provisioned 1; "
            "settings put secure user_setup_complete 1; "
            "settings put secure icon_blacklist battery; "
            "dumpsys battery set present 0 2>/dev/null || true; "
            "settings put system screen_off_timeout 2147483647; "
            "settings put system accelerometer_rotation 0; "
            "settings put system user_rotation 0; "
            "settings put global policy_control immersive.status=*; "
            "wm fixed-to-user-rotation enabled; "
            "wm user-rotation lock 0; "
            "locksettings set-disabled true 2>/dev/null || true; "
            "pm disable-user --user 0 org.lineageos.setupwizard 2>/dev/null || true"
        ], timeout=20)
        for pkg in DISABLE_PHONE_PKGS:
            run_cmd([adb, "-s", serial, "shell", f"pm disable-user --user 0 {pkg} 2>/dev/null || true"], timeout=8)

        if install_apps:
            apks_dir = Path(self.config.get("apks_dir", "")).expanduser()
            if apks_dir.is_dir():
                apks = sorted(apks_dir.glob("*.apk"))
                for idx, apk in enumerate(apks):
                    pct = 78 + int((idx + 1) * 14 / max(1, len(apks)))
                    self.set_progress(pct, f"Installing {apk.name}...")
                    rc = self._stream_cmd([adb, "-s", serial, "install", "-r", str(apk)], timeout=120)
                    if rc != 0:
                        raise RuntimeError(f"Failed installing {apk.name}")

        _, has_proj = run_cmd([adb, "-s", serial, "shell", "pm list packages com.spocky.projengmenu"], timeout=8)
        if "com.spocky.projengmenu" in has_proj:
            run_cmd([adb, "-s", serial, "shell", "cmd package set-home-activity com.spocky.projengmenu/.ui.home.MainActivity"], timeout=8)

        ssid = self.config.get("wifi_ssid", "")
        pw = self.config.get("wifi_password", "")
        if ssid and pw:
            self.set_progress(93, f"Connecting Wi-Fi ({ssid})...")
            run_cmd([adb, "-s", serial, "shell", f"svc wifi enable; sleep 3; cmd wifi connect-network '{ssid}' wpa2 '{pw}'"], timeout=20)

        run_cmd([adb, "-s", serial, "shell", "input keyevent KEYCODE_HOME; sync"], timeout=8)

        # Verification checklist
        self.set_progress(96, "Running verification checklist...")
        checks = []

        def check(name: str, cmd: str, pattern: str):
            _, out = run_cmd([adb, "-s", serial, "shell", cmd], timeout=8)
            ok = bool(re.search(pattern, out))
            checks.append({"name": name, "ok": ok, "detail": out.splitlines()[0] if out else ""})
            self.log(f"[{'PASS' if ok else 'FAIL'}] {name}" + (f" ({out.splitlines()[0]})" if out else ""), "ok" if ok else "error")

        check("Kernel 4.4.302-podmin", "uname -r", r"4\.4\.302-podmin")
        check("Android boot completed", "getprop sys.boot_completed", r"^1$")
        check("Wi-Fi interface (wlan0)", "ip link show wlan0 2>/dev/null", r"wlan0")
        if has_baked:
            check("HDMI-CEC daemon running", "pidof pod-cec-daemon", r"\d+")
            check("HDMI hot-plug running", "ps -ef | grep pod-hdmi-hotplug | grep -v grep", r"pod-hdmi")
        if "com.spocky.projengmenu" in has_proj:
            check("Projectivy default home", "cmd role get-role-holders --user 0 android.app.role.HOME", r"spocky")

        self.set_checks(checks)
        failed = [c["name"] for c in checks if not c["ok"]]
        if failed:
            self.flash_state[serial] = f"failed: {', '.join(failed)}"
            self._save_state()
            raise RuntimeError(f"Verification failed: {', '.join(failed)}")

        self.flash_state[serial] = "done"
        self._save_state()
        self.log(f"Pod {serial} verified and marked DONE!", "ok")

    def _run_install(self, payload: dict):
        adb, fb, note = self.get_tools()
        if note:
            self.log(note)
        serial = payload.get("serial", "").strip()
        mode = payload.get("mode", "fastboot")
        if not serial:
            raise RuntimeError("No device serial selected.")
        if mode == "fastboot":
            if not fb:
                raise RuntimeError("fastboot not found. Configure Platform Tools path.")
            self._flash_images(fb, serial, wipe_userdata=True)
        else:
            self.log("Device is already booted in Android; resuming at provisioning step.")
        if not adb:
            raise RuntimeError("adb not found. Configure Platform Tools path.")
        self._provision(adb, serial, install_apps=bool(payload.get("install_apps", True)))

    def _run_upgrade(self, payload: dict):
        adb, fb, note = self.get_tools()
        if note:
            self.log(note)
        serial = payload.get("serial", "").strip()
        mode = payload.get("mode", "fastboot")
        if not serial:
            raise RuntimeError("No device serial selected.")
        if mode == "android" and adb:
            self.set_progress(5, "Rebooting Pod into fastboot for upgrade...")
            run_cmd([adb, "-s", serial, "reboot", "bootloader"], timeout=10)
            time.sleep(6)
        if not fb:
            raise RuntimeError("fastboot not found.")
        self.log("Upgrade mode: preserving userdata and installed apps (no wipe).")
        self._flash_images(fb, serial, wipe_userdata=False)
        self._provision(adb, serial, install_apps=False)

    def _run_provision_only(self, payload: dict):
        adb, _, note = self.get_tools()
        if note:
            self.log(note)
        serial = payload.get("serial", "").strip()
        if not serial or not adb:
            raise RuntimeError("Select a booted Android device and ensure adb is configured.")
        self._provision(adb, serial, install_apps=bool(payload.get("install_apps", True)))

    def _run_backup(self, payload: dict):
        adb, _, _ = self.get_tools()
        serial = payload.get("serial", "").strip()
        partitions = payload.get("partitions") or DEFAULT_BACKUP_PARTITIONS
        if not serial or not adb:
            raise RuntimeError("Backup requires a booted Pod connected over ADB (with root access).")

        self.set_progress(5, "Checking ADB root access...")
        run_cmd([adb, "-s", serial, "root"], timeout=10)
        time.sleep(2)
        run_cmd([adb, "-s", serial, "wait-for-device"], timeout=15)
        _, uid = run_cmd([adb, "-s", serial, "shell", "id -u"], timeout=6)
        if uid.strip() != "0":
            raise RuntimeError(
                "ADB root is required to read raw partitions. Enable 'Rooted debugging' in "
                "Settings -> System -> Developer options on the Pod, then retry."
            )

        bdir = Path(self.config.get("backups_dir", str(REPO_ROOT / "backups"))).expanduser()
        stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
        out_dir = bdir / f"{serial}-{stamp}"
        out_dir.mkdir(parents=True, exist_ok=True)
        self.log(f"Saving backup to {out_dir}")

        manifest = {
            "serial": serial,
            "created_at": datetime.now().isoformat(timespec="seconds"),
            "files": {},
        }

        for idx, part in enumerate(partitions):
            pct = 10 + int((idx + 1) * 85 / max(1, len(partitions)))
            self.set_progress(pct, f"Backing up {part}...")
            rc, blk = run_cmd(
                [adb, "-s", serial, "shell", f"ls /dev/block/bootdevice/by-name/{part} 2>/dev/null"],
                timeout=8,
            )
            if rc != 0 or not blk.strip():
                self.log(f"Skipping {part}: partition node not found.", "warn")
                continue
            remote_tmp = f"/data/local/tmp/bk_{part}.img"
            rc, out = run_cmd([adb, "-s", serial, "shell", f"dd if={blk.strip()} of={remote_tmp} bs=4M 2>&1"], timeout=120)
            if rc != 0:
                raise RuntimeError(f"dd failed on {part}: {out}")
            local_img = out_dir / f"{part}.img"
            rc = self._stream_cmd([adb, "-s", serial, "pull", remote_tmp, str(local_img)], timeout=180)
            run_cmd([adb, "-s", serial, "shell", f"rm -f {remote_tmp}"], timeout=8)
            if rc != 0 or not local_img.is_file():
                raise RuntimeError(f"adb pull failed for {part}")
            digest = sha256_file(local_img)
            size = local_img.stat().st_size
            manifest["files"][f"{part}.img"] = {"partition": part, "size": size, "sha256": digest}
            self.log(f"Backed up {part}.img ({size:,} bytes, sha256 {digest[:12]}...)", "ok")

        (out_dir / "backup-manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
        self.log(f"Backup complete: {out_dir}", "ok")

    def _run_restore(self, payload: dict):
        _, fb, _ = self.get_tools()
        serial = payload.get("serial", "").strip()
        backup_path = Path(payload.get("backup_path", "")).expanduser()
        include_sensitive = bool(payload.get("include_sensitive", False))

        if not serial or not fb:
            raise RuntimeError("Restore requires the Pod to be in fastboot mode.")
        if not backup_path.is_dir():
            raise RuntimeError(f"Backup folder not found: {backup_path}")

        manifest_path = backup_path / "backup-manifest.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8")) if manifest_path.is_file() else {}
        files_meta = manifest.get("files", {})

        to_flash = []
        for img in sorted(backup_path.glob("*.img")):
            part = img.stem
            if part in SENSITIVE_PARTITIONS and not include_sensitive:
                self.log(f"Skipping sensitive partition {part} (safety lock enabled).")
                continue
            if img.name in files_meta and files_meta[img.name].get("sha256"):
                self.set_progress(10, f"Verifying SHA-256 for {img.name}...")
                actual = sha256_file(img)
                if actual != files_meta[img.name]["sha256"]:
                    raise RuntimeError(f"SHA-256 mismatch for {img.name}! Aborting restore.")
                self.log(f"SHA-256 verified for {img.name}", "ok")
            to_flash.append((part, img))

        if not to_flash:
            raise RuntimeError("No eligible .img files selected to restore.")

        for idx, (part, img) in enumerate(to_flash):
            pct = 20 + int((idx + 1) * 70 / len(to_flash))
            self.set_progress(pct, f"Restoring {part}...")
            rc = self._stream_cmd([fb, "-s", serial, "flash", part, str(img)], timeout=300)
            if rc != 0:
                raise RuntimeError(f"fastboot flash {part} failed (exit {rc})")

        self.set_progress(95, "Rebooting Pod...")
        self._stream_cmd([fb, "-s", serial, "reboot"], timeout=30)
        self.log("Restore finished and Pod rebooted.", "ok")

    def quick_action(self, action: str, serial: str) -> tuple[bool, str]:
        adb, fb, _ = self.get_tools()
        if not serial:
            return False, "No device selected."
        if action == "reboot_fastboot" and adb:
            rc, out = run_cmd([adb, "-s", serial, "reboot", "bootloader"], timeout=10)
            return rc == 0, out or "Rebooting into fastboot..."
        if action == "reboot_android":
            if fb:
                rc, out = run_cmd([fb, "-s", serial, "reboot"], timeout=10)
                if rc == 0:
                    return True, "Rebooting from fastboot..."
            if adb:
                rc, out = run_cmd([adb, "-s", serial, "reboot"], timeout=10)
                return rc == 0, out or "Rebooting Android..."
        if action == "enable_wifi_adb" and adb:
            rc, out = run_cmd([adb, "-s", serial, "tcpip", "5555"], timeout=10)
            return rc == 0, out or "Wireless ADB enabled on port 5555."
        if action == "enable_mtp" and adb:
            rc, out = run_cmd([adb, "-s", serial, "shell", "svc usb setFunctions mtp true"], timeout=10)
            return rc == 0, out or "Switched USB to MTP + ADB."
        return False, f"Action '{action}' not available for current device state."


MANAGER = JobManager()


class Handler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        return  # keep console clean

    def _send_json(self, data: dict, code: int = 200):
        body = json.dumps(data).encode("utf-8")
        self.send_response(code)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        parsed = urllib.parse.urlparse(self.path)
        if parsed.path in ("/", "/index.html"):
            html = (APP_DIR / "ui.html").read_bytes()
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(html)))
            self.end_headers()
            self.wfile.write(html)
            return
        if parsed.path == "/api/status":
            with MANAGER.lock:
                job_snap = dict(MANAGER.job)
            self._send_json({
                "devices_info": MANAGER.detect_devices(),
                "config": MANAGER.config,
                "job": job_snap,
                "backups": MANAGER.list_backups(),
            })
            return
        self.send_error(404)

    def do_POST(self):
        length = int(self.headers.get("Content-Length", "0"))
        raw = self.rfile.read(length) if length else b"{}"
        try:
            payload = json.loads(raw.decode("utf-8"))
        except Exception:
            self._send_json({"ok": False, "error": "Invalid JSON"}, 400)
            return

        path = urllib.parse.urlparse(self.path).path
        if path == "/api/config":
            cfg = MANAGER.save_config(payload)
            self._send_json({"ok": True, "config": cfg})
            return
        if path == "/api/job":
            ok, msg = MANAGER.start_job(payload.get("kind", ""), payload)
            self._send_json({"ok": ok, "message": msg}, 200 if ok else 409)
            return
        if path == "/api/action":
            ok, msg = MANAGER.quick_action(payload.get("action", ""), payload.get("serial", ""))
            self._send_json({"ok": ok, "message": msg}, 200 if ok else 400)
            return
        self.send_error(404)


def main():
    port = int(os.environ.get("POD_FLASHER_PORT", str(DEFAULT_PORT)))
    server = ThreadingHTTPServer(("127.0.0.1", port), Handler)
    url = f"http://127.0.0.1:{port}"
    print(f"Solstice Pod TV Flasher running at {url} (Press Ctrl+C to stop)")
    if "--no-browser" not in sys.argv:
        threading.Timer(0.6, lambda: webbrowser.open(url)).start()
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nShutting down.")
        server.server_close()


if __name__ == "__main__":
    main()
