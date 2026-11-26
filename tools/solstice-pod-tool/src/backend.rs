use crate::catalog::{catalog, download_to_file, resolve_download_url};
use chrono::Local;
use directories::ProjectDirs;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

const DISABLE_PHONE_PKGS: &[&str] = &[
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
];

const SENSITIVE_PARTS: &[&str] = &["persist", "modem", "bluetooth", "bt_firmware"];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub platform_tools_dir: String,
    pub images_dir: String,
    pub apks_dir: String,
    pub backups_dir: String,
    pub wifi_ssid: String,
    pub wifi_password: String,
    pub dark_mode: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        let base = app_data_dir();
        Self {
            platform_tools_dir: String::new(),
            images_dir: base.join("images").to_string_lossy().to_string(),
            apks_dir: base.join("tv-apks").to_string_lossy().to_string(),
            backups_dir: base.join("backups").to_string_lossy().to_string(),
            wifi_ssid: String::new(),
            wifi_password: String::new(),
            dark_mode: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeviceMode {
    Fastboot,
    Android,
    Recovery,
    Unauthorized,
}

#[derive(Clone, Debug)]
pub struct PodDevice {
    pub serial: String,
    pub mode: DeviceMode,
    pub product: String,
    pub android_ver: String,
    pub kernel: String,
    pub wifi_ip: String,
    pub storage: String,
    pub cec_on: bool,
    pub hotplug_on: bool,
    pub home_app: String,
    pub saved_state: String,
}

#[derive(Clone, Debug)]
pub struct SelfChecks {
    pub adb_path: Option<PathBuf>,
    pub fastboot_path: Option<PathBuf>,
    pub images_found: usize,
    pub driver_warnings: Vec<String>,
    pub os_name: String,
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub ts: String,
    pub level: LogLevel,
    pub msg: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Ok,
    Warn,
    Error,
}

#[derive(Clone, Debug)]
pub struct CheckResult {
    pub name: String,
    pub ok: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobStatus {
    Idle,
    Running,
    Done,
    Error,
}

#[derive(Clone, Debug)]
pub struct BackupEntry {
    pub name: String,
    pub path: PathBuf,
    pub file_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct BackupManifest {
    serial: String,
    created_at: String,
    files: HashMap<String, BackupFileMeta>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct BackupFileMeta {
    partition: String,
    size: u64,
    sha256: String,
}

pub struct SharedState {
    pub config: AppConfig,
    pub checks: SelfChecks,
    pub devices: Vec<PodDevice>,
    pub backups: Vec<BackupEntry>,
    pub flash_state: HashMap<String, String>,
    pub job_status: JobStatus,
    pub job_step: String,
    pub job_progress: f32,
    pub job_checks: Vec<CheckResult>,
    pub logs: Vec<LogEntry>,
}

impl SharedState {
    pub fn new() -> Self {
        let config = load_config();
        let flash_state = load_flash_state();
        Self {
            config,
            checks: SelfChecks {
                adb_path: None,
                fastboot_path: None,
                images_found: 0,
                driver_warnings: vec![],
                os_name: std::env::consts::OS.to_string(),
            },
            devices: vec![],
            backups: vec![],
            flash_state,
            job_status: JobStatus::Idle,
            job_step: "Ready".into(),
            job_progress: 0.0,
            job_checks: vec![],
            logs: vec![],
        }
    }

    pub fn push_log(&mut self, level: LogLevel, msg: impl Into<String>) {
        let ts = Local::now().format("%H:%M:%S").to_string();
        self.logs.push(LogEntry {
            ts,
            level,
            msg: msg.into(),
        });
        if self.logs.len() > 400 {
            let drain = self.logs.len() - 400;
            self.logs.drain(0..drain);
        }
    }
}

pub fn app_data_dir() -> PathBuf {
    if let Some(pd) = ProjectDirs::from("io", "SolsticePodTV", "SolsticePodTool") {
        let p = pd.data_local_dir().to_path_buf();
        let _ = fs::create_dir_all(&p);
        return p;
    }
    std::env::temp_dir().join("SolsticePodTool")
}

/// Guaranteed space-free directory for platform-tools on Windows (avoids fastboot format error 193).
pub fn managed_platform_tools_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let p = PathBuf::from(r"C:\ProgramData\SolsticePodTool\platform-tools");
        if fs::create_dir_all(&p).is_ok() {
            return p;
        }
        let t = std::env::temp_dir().join("sp8096-pt");
        let _ = fs::create_dir_all(&t);
        t
    }
    #[cfg(not(target_os = "windows"))]
    {
        let p = app_data_dir().join("platform-tools");
        let _ = fs::create_dir_all(&p);
        p
    }
}

fn config_path() -> PathBuf {
    app_data_dir().join("config.json")
}

fn flash_state_path() -> PathBuf {
    app_data_dir().join("flash-state.json")
}

pub fn load_config() -> AppConfig {
    if let Ok(raw) = fs::read_to_string(config_path()) {
        if let Ok(cfg) = serde_json::from_str(&raw) {
            return cfg;
        }
    }
    AppConfig::default()
}

pub fn save_config(cfg: &AppConfig) {
    if let Ok(txt) = serde_json::to_string_pretty(cfg) {
        let _ = fs::write(config_path(), txt);
    }
}

fn load_flash_state() -> HashMap<String, String> {
    if let Ok(raw) = fs::read_to_string(flash_state_path()) {
        if let Ok(m) = serde_json::from_str(&raw) {
            return m;
        }
    }
    HashMap::new()
}

fn save_flash_state(m: &HashMap<String, String>) {
    if let Ok(txt) = serde_json::to_string_pretty(m) {
        let _ = fs::write(flash_state_path(), txt);
    }
}

fn new_cmd(program: &Path) -> Command {
    let mut c = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        c.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    c
}

fn run_capture(program: &Path, args: &[&str]) -> Result<String, String> {
    let out = new_cmd(program)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run {:?}: {}", program, e))?;
    let mut combined = String::from_utf8_lossy(&out.stdout).to_string();
    let err = String::from_utf8_lossy(&out.stderr);
    if !err.is_empty() {
        if !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str(&err);
    }
    if out.status.success() {
        Ok(combined.trim().to_string())
    } else {
        Err(combined.trim().to_string())
    }
}

fn run_stream(
    state: &Arc<Mutex<SharedState>>,
    program: &Path,
    args: &[&str],
) -> Result<(), String> {
    {
        let mut s = state.lock().unwrap();
        s.push_log(LogLevel::Info, format!("$ {} {}", program.display(), args.join(" ")));
    }
    let mut child = new_cmd(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn {:?}: {}", program, e))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let st_out = Arc::clone(state);
    let t1 = thread::spawn(move || {
        if let Some(out) = stdout {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                if !line.trim().is_empty() && !line.contains("fetch:super") && !line.contains("avb footer") {
                    st_out.lock().unwrap().push_log(LogLevel::Info, line);
                }
            }
        }
    });
    let st_err = Arc::clone(state);
    let t2 = thread::spawn(move || {
        if let Some(err) = stderr {
            for line in BufReader::new(err).lines().map_while(Result::ok) {
                if !line.trim().is_empty() && !line.contains("fetch:super") && !line.contains("avb footer") {
                    st_err.lock().unwrap().push_log(LogLevel::Info, line);
                }
            }
        }
    });

    let status = child.wait().map_err(|e| e.to_string())?;
    let _ = t1.join();
    let _ = t2.join();
    if status.success() {
        Ok(())
    } else {
        Err(format!("Command exited with status {}", status))
    }
}

pub fn find_platform_tools(configured: &str) -> (Option<PathBuf>, Option<PathBuf>) {
    let adb_name = if cfg!(windows) { "adb.exe" } else { "adb" };
    let fb_name = if cfg!(windows) { "fastboot.exe" } else { "fastboot" };

    let mut dirs: Vec<PathBuf> = vec![];
    if !configured.trim().is_empty() {
        dirs.push(PathBuf::from(configured.trim()));
    }
    dirs.push(managed_platform_tools_dir());
    dirs.push(managed_platform_tools_dir().join("platform-tools"));
    if let Ok(exe) = std::env::current_exe() {
        if let Some(p) = exe.parent() {
            dirs.push(p.join("platform-tools"));
        }
    }
    dirs.push(PathBuf::from("/opt/homebrew/bin"));
    dirs.push(PathBuf::from("/usr/local/bin"));
    dirs.push(PathBuf::from("/usr/bin"));

    for d in dirs {
        let a = d.join(adb_name);
        let f = d.join(fb_name);
        if a.is_file() && f.is_file() {
            // Stage space-free copy on Windows if needed
            #[cfg(target_os = "windows")]
            if f.to_string_lossy().contains(' ') {
                let dst = std::env::temp_dir().join("sp8096-pt");
                if !dst.join(fb_name).is_file() {
                    let _ = copy_dir_flat(&d, &dst);
                }
                if dst.join(adb_name).is_file() && dst.join(fb_name).is_file() {
                    return (Some(dst.join(adb_name)), Some(dst.join(fb_name)));
                }
            }
            return (Some(a), Some(f));
        }
    }
    (None, None)
}

#[cfg(target_os = "windows")]
fn copy_dir_flat(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let e = entry?;
        if e.file_type()?.is_file() {
            fs::copy(e.path(), dst.join(e.file_name()))?;
        }
    }
    Ok(())
}

fn check_windows_drivers() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        let mut c = Command::new("powershell");
        c.creation_flags(0x08000000);
        if let Ok(out) = c
            .args([
                "-NoProfile",
                "-Command",
                "Get-PnpDevice -PresentOnly -ErrorAction SilentlyContinue | Where-Object { $_.InstanceId -match 'VID_18D1&PID_D00D' -and $_.Status -ne 'OK' } | ForEach-Object { $_.InstanceId }",
            ])
            .output()
        {
            return String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
        }
    }
    vec![]
}

fn count_images(dir: &str) -> usize {
    let p = Path::new(dir);
    let names = [
        "pod44-los22-boot-signed.img",
        "pod44-los22-recovery-signed.img",
        "pod-los22-system.simg",
    ];
    names.iter().filter(|n| p.join(n).is_file()).count()
}

fn scan_backups(dir: &str) -> Vec<BackupEntry> {
    let mut items = vec![];
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                let count = fs::read_dir(&p)
                    .map(|r| {
                        r.flatten()
                            .filter(|x| x.path().extension().and_then(|s| s.to_str()) == Some("img"))
                            .count()
                    })
                    .unwrap_or(0);
                if count > 0 {
                    items.push(BackupEntry {
                        name: e.file_name().to_string_lossy().to_string(),
                        path: p,
                        file_count: count,
                    });
                }
            }
        }
    }
    items.sort_by(|a, b| b.name.cmp(&a.name));
    items
}

pub fn start_poller(state: Arc<Mutex<SharedState>>) {
    thread::spawn(move || loop {
        let (cfg, flash_state, job_running) = {
            let s = state.lock().unwrap();
            (
                s.config.clone(),
                s.flash_state.clone(),
                s.job_status == JobStatus::Running,
            )
        };

        let (adb, fb) = find_platform_tools(&cfg.platform_tools_dir);
        let images_found = count_images(&cfg.images_dir);
        let backups = scan_backups(&cfg.backups_dir);
        let driver_warnings = if job_running { vec![] } else { check_windows_drivers() };

        let mut devices = vec![];
        if !job_running {
            if let Some(ref fb_bin) = fb {
                if let Ok(out) = run_capture(fb_bin, &["devices"]) {
                    for line in out.lines() {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 && parts[1] == "fastboot" {
                            let ser = parts[0].to_string();
                            let product = run_capture(fb_bin, &["-s", &ser, "getvar", "product"])
                                .ok()
                                .and_then(|t| {
                                    Regex::new(r"product:\s*(\S+)")
                                        .ok()
                                        .and_then(|rx| rx.captures(&t))
                                        .map(|c| c[1].to_string())
                                })
                                .unwrap_or_else(|| "SP8096".into());
                            devices.push(PodDevice {
                                saved_state: flash_state.get(&ser).cloned().unwrap_or_default(),
                                serial: ser,
                                mode: DeviceMode::Fastboot,
                                product,
                                android_ver: String::new(),
                                kernel: String::new(),
                                wifi_ip: String::new(),
                                storage: String::new(),
                                cec_on: false,
                                hotplug_on: false,
                                home_app: String::new(),
                            });
                        }
                    }
                }
            }
            if let Some(ref adb_bin) = adb {
                if let Ok(out) = run_capture(adb_bin, &["devices"]) {
                    for line in out.lines().skip(1) {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            let mode = match parts[1] {
                                "device" => DeviceMode::Android,
                                "recovery" => DeviceMode::Recovery,
                                "unauthorized" => DeviceMode::Unauthorized,
                                _ => continue,
                            };
                            let ser = parts[0].to_string();
                            let mut dev = PodDevice {
                                saved_state: flash_state.get(&ser).cloned().unwrap_or_default(),
                                serial: ser.clone(),
                                mode: mode.clone(),
                                product: "SP8096".into(),
                                android_ver: String::new(),
                                kernel: String::new(),
                                wifi_ip: String::new(),
                                storage: String::new(),
                                cec_on: false,
                                hotplug_on: false,
                                home_app: String::new(),
                            };
                            if mode == DeviceMode::Android {
                                if let Ok(probe) = run_capture(
                                    adb_bin,
                                    &[
                                        "-s",
                                        &ser,
                                        "shell",
                                        "echo K=$(uname -r); echo A=$(getprop ro.build.version.release); echo IP=$(ip -4 addr show wlan0 2>/dev/null | grep -o 'inet [0-9.]*' | awk '{print $2}'); echo CEC=$(pidof pod-cec-daemon 2>/dev/null); echo HP=$(ps -ef 2>/dev/null | grep pod-hdmi-hotplug | grep -v grep | awk '{print $2}' | head -n1); echo HOME=$(cmd role get-role-holders --user 0 android.app.role.HOME 2>/dev/null); echo DF=$(df -h /data 2>/dev/null | tail -n1 | awk '{print $4 \" free of \" $2}')",
                                    ],
                                ) {
                                    for kv in probe.lines() {
                                        if let Some((k, v)) = kv.split_once('=') {
                                            let v = v.trim().to_string();
                                            match k.trim() {
                                                "K" => dev.kernel = v,
                                                "A" => dev.android_ver = v,
                                                "IP" => dev.wifi_ip = v,
                                                "CEC" => dev.cec_on = !v.is_empty(),
                                                "HP" => dev.hotplug_on = !v.is_empty(),
                                                "HOME" => dev.home_app = v,
                                                "DF" => dev.storage = v,
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                            }
                            devices.push(dev);
                        }
                    }
                }
            }
        }

        {
            let mut s = state.lock().unwrap();
            s.checks.adb_path = adb;
            s.checks.fastboot_path = fb;
            s.checks.images_found = images_found;
            s.checks.driver_warnings = driver_warnings;
            s.backups = backups;
            if !job_running {
                s.devices = devices;
            }
        }
        thread::sleep(Duration::from_secs(2));
    });
}

fn begin_job(state: &Arc<Mutex<SharedState>>, step: &str) -> bool {
    let mut s = state.lock().unwrap();
    if s.job_status == JobStatus::Running {
        return false;
    }
    s.job_status = JobStatus::Running;
    s.job_step = step.into();
    s.job_progress = 0.02;
    s.job_checks.clear();
    s.logs.clear();
    s.push_log(LogLevel::Info, step);
    true
}

fn finish_job(state: &Arc<Mutex<SharedState>>, res: Result<(), String>) {
    let mut s = state.lock().unwrap();
    match res {
        Ok(()) => {
            s.job_status = JobStatus::Done;
            s.job_progress = 1.0;
            s.job_step = "Completed".into();
            s.push_log(LogLevel::Ok, "Job completed successfully.");
        }
        Err(e) => {
            s.job_status = JobStatus::Error;
            s.job_step = format!("Failed: {}", e);
            s.push_log(LogLevel::Error, format!("ERROR: {}", e));
        }
    }
}

fn set_step(state: &Arc<Mutex<SharedState>>, pct: f32, step: impl Into<String>) {
    let mut s = state.lock().unwrap();
    s.job_progress = pct.clamp(0.0, 1.0);
    s.job_step = step.into();
}

/// 1-click official Google Platform Tools downloader (Windows/macOS/Linux)
pub fn spawn_download_platform_tools(state: Arc<Mutex<SharedState>>) {
    if !begin_job(&state, "Downloading official Google Platform Tools...") {
        return;
    }
    thread::spawn(move || {
        let res = (|| -> Result<(), String> {
            let url = if cfg!(target_os = "windows") {
                "https://dl.google.com/android/repository/platform-tools-latest-windows.zip"
            } else if cfg!(target_os = "macos") {
                "https://dl.google.com/android/repository/platform-tools-latest-darwin.zip"
            } else {
                "https://dl.google.com/android/repository/platform-tools-latest-linux.zip"
            };

            let dst = managed_platform_tools_dir();
            fs::create_dir_all(&dst).map_err(|e| e.to_string())?;
            let zip_path = dst.join("platform-tools.zip");

            set_step(&state, 0.25, "Downloading platform-tools zip from dl.google.com...");
            download_to_file(url, &zip_path)?;

            set_step(&state, 0.70, "Extracting platform-tools...");
            let file = File::open(&zip_path).map_err(|e| e.to_string())?;
            let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
            for i in 0..archive.len() {
                let mut f = archive.by_index(i).map_err(|e| e.to_string())?;
                let outpath = match f.enclosed_name() {
                    Some(p) => dst.join(p),
                    None => continue,
                };
                if f.name().ends_with('/') {
                    fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
                } else {
                    if let Some(p) = outpath.parent() {
                        fs::create_dir_all(p).map_err(|e| e.to_string())?;
                    }
                    let mut outfile = File::create(&outpath).map_err(|e| e.to_string())?;
                    io::copy(&mut f, &mut outfile).map_err(|e| e.to_string())?;
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        if let Some(mode) = f.unix_mode() {
                            let _ = fs::set_permissions(&outpath, fs::Permissions::from_mode(mode));
                        }
                    }
                }
            }
            let _ = fs::remove_file(&zip_path);
            let extracted = dst.join("platform-tools");
            {
                let mut s = state.lock().unwrap();
                s.config.platform_tools_dir = extracted.to_string_lossy().to_string();
                save_config(&s.config);
                s.push_log(LogLevel::Ok, format!("Installed Platform Tools to {}", extracted.display()));
            }
            Ok(())
        })();
        finish_job(&state, res);
    });
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut f = File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 1024 * 64];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn wait_for_boot(state: &Arc<Mutex<SharedState>>, adb: &Path, serial: &str) -> Result<(), String> {
    set_step(state, 0.60, "Waiting for Android 15 boot...");
    let _ = run_capture(adb, &["-s", serial, "wait-for-device"]);
    for _ in 0..120 {
        if let Ok(out) = run_capture(adb, &["-s", serial, "shell", "getprop sys.boot_completed"]) {
            if out.trim() == "1" {
                thread::sleep(Duration::from_secs(4));
                return Ok(());
            }
        }
        thread::sleep(Duration::from_secs(2));
    }
    Err("Timed out waiting for sys.boot_completed=1".into())
}

fn flash_images(
    state: &Arc<Mutex<SharedState>>,
    fb: &Path,
    serial: &str,
    images_dir: &str,
    wipe_userdata: bool,
) -> Result<(), String> {
    let dir = Path::new(images_dir);
    let boot = dir.join("pod44-los22-boot-signed.img");
    let rec = dir.join("pod44-los22-recovery-signed.img");
    let sys = dir.join("pod-los22-system.simg");
    for f in [&boot, &rec, &sys] {
        if !f.is_file() {
            return Err(format!("Missing image: {}", f.display()));
        }
    }

    set_step(state, 0.12, "Flashing boot...");
    run_stream(state, fb, &["-s", serial, "flash", "boot", &boot.to_string_lossy()])?;
    set_step(state, 0.22, "Flashing recovery...");
    run_stream(state, fb, &["-s", serial, "flash", "recovery", &rec.to_string_lossy()])?;
    set_step(state, 0.45, "Flashing system (2.3 GB)...");
    run_stream(state, fb, &["-s", serial, "flash", "system", &sys.to_string_lossy()])?;

    if wipe_userdata {
        set_step(state, 0.52, "Formatting userdata (ext4)...");
        run_stream(state, fb, &["-s", serial, "format:ext4", "userdata"])?;
        set_step(state, 0.55, "Formatting cache (ext4)...");
        run_stream(state, fb, &["-s", serial, "format:ext4", "cache"])?;
    }
    set_step(state, 0.58, "Rebooting into Android 15...");
    run_stream(state, fb, &["-s", serial, "reboot"])?;
    Ok(())
}

fn provision_pod(
    state: &Arc<Mutex<SharedState>>,
    adb: &Path,
    serial: &str,
    cfg: &AppConfig,
    install_apps: bool,
) -> Result<(), String> {
    wait_for_boot(state, adb, serial)?;

    set_step(state, 0.70, "Applying TV defaults & disabling phone apps...");
    let has_cec = run_capture(adb, &["-s", serial, "shell", "ls /vendor/bin/pod-cec-daemon 2>/dev/null"])
        .unwrap_or_default()
        .contains("pod-cec-daemon");
    let has_hp = run_capture(adb, &["-s", serial, "shell", "ls /vendor/bin/pod-hdmi-hotplug.sh 2>/dev/null"])
        .unwrap_or_default()
        .contains("hotplug");
    let has_baked = has_cec && has_hp;

    let _ = run_capture(
        adb,
        &[
            "-s",
            serial,
            "shell",
            "settings put global device_provisioned 1; settings put secure user_setup_complete 1; settings put secure icon_blacklist battery; dumpsys battery set present 0 2>/dev/null || true; settings put system screen_off_timeout 2147483647; settings put system accelerometer_rotation 0; settings put system user_rotation 0; settings put global policy_control immersive.status=*; wm fixed-to-user-rotation enabled; wm user-rotation lock 0; locksettings set-disabled true 2>/dev/null || true; pm disable-user --user 0 org.lineageos.setupwizard 2>/dev/null || true",
        ],
    );
    for pkg in DISABLE_PHONE_PKGS {
        let cmd = format!("pm disable-user --user 0 {} 2>/dev/null || true", pkg);
        let _ = run_capture(adb, &["-s", serial, "shell", &cmd]);
    }

    if install_apps {
        let apks_dir = Path::new(&cfg.apks_dir);
        if apks_dir.is_dir() {
            let mut apks: Vec<PathBuf> = fs::read_dir(apks_dir)
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("apk"))
                .collect();
            apks.sort();
            let total = apks.len().max(1);
            for (i, apk) in apks.iter().enumerate() {
                let name = apk.file_name().unwrap_or_default().to_string_lossy().to_string();
                set_step(state, 0.75 + (i as f32 / total as f32) * 0.16, format!("Installing {}...", name));
                run_stream(state, adb, &["-s", serial, "install", "-r", &apk.to_string_lossy()])?;
            }
        }
    }

    let has_proj = run_capture(adb, &["-s", serial, "shell", "pm list packages com.spocky.projengmenu"])
        .unwrap_or_default()
        .contains("com.spocky.projengmenu");
    if has_proj {
        let _ = run_capture(
            adb,
            &["-s", serial, "shell", "cmd package set-home-activity com.spocky.projengmenu/.ui.home.MainActivity"],
        );
    }

    if !cfg.wifi_ssid.is_empty() && !cfg.wifi_password.is_empty() {
        set_step(state, 0.92, format!("Connecting Wi-Fi ({})...", cfg.wifi_ssid));
        let cmd = format!(
            "svc wifi enable; sleep 3; cmd wifi connect-network '{}' wpa2 '{}'",
            cfg.wifi_ssid, cfg.wifi_password
        );
        let _ = run_capture(adb, &["-s", serial, "shell", &cmd]);
    }
    let _ = run_capture(adb, &["-s", serial, "shell", "input keyevent KEYCODE_HOME; sync"]);

    // Verification checklist
    set_step(state, 0.95, "Running verification checklist...");
    let mut results = vec![];
    let mut check = |name: &str, cmd: &str, pat: &str| {
        let out = run_capture(adb, &["-s", serial, "shell", cmd]).unwrap_or_default();
        let ok = Regex::new(pat).map(|r| r.is_match(&out)).unwrap_or(false);
        results.push(CheckResult { name: name.into(), ok });
        state.lock().unwrap().push_log(
            if ok { LogLevel::Ok } else { LogLevel::Error },
            format!("[{}] {}", if ok { "PASS" } else { "FAIL" }, name),
        );
    };

    check("Kernel 4.4.302-podmin", "uname -r", r"4\.4\.302-podmin");
    check("Android boot completed", "getprop sys.boot_completed", r"^1$");
    check("Wi-Fi interface (wlan0)", "ip link show wlan0 2>/dev/null", r"wlan0");
    if has_baked {
        check("HDMI-CEC daemon running", "pidof pod-cec-daemon", r"\d+");
        check("HDMI hot-plug running", "ps -ef | grep pod-hdmi-hotplug | grep -v grep", r"pod-hdmi");
    }
    if has_proj {
        check("Projectivy default home", "cmd role get-role-holders --user 0 android.app.role.HOME", r"spocky");
    }

    let failed: Vec<String> = results.iter().filter(|r| !r.ok).map(|r| r.name.clone()).collect();
    {
        let mut s = state.lock().unwrap();
        s.job_checks = results;
        if failed.is_empty() {
            s.flash_state.insert(serial.to_string(), "done".into());
        } else {
            s.flash_state.insert(serial.to_string(), format!("failed: {}", failed.join(", ")));
        }
        save_flash_state(&s.flash_state);
    }
    if !failed.is_empty() {
        return Err(format!("Verification failed: {}", failed.join(", ")));
    }
    Ok(())
}

pub fn spawn_install_or_upgrade(
    state: Arc<Mutex<SharedState>>,
    serial: String,
    mode: DeviceMode,
    wipe_userdata: bool,
    install_apps: bool,
    provision_only: bool,
) {
    let label = if provision_only {
        format!("Provisioning Pod {}...", serial)
    } else if wipe_userdata {
        format!("Clean Install / Resume on Pod {}...", serial)
    } else {
        format!("Upgrading Pod {} (keeping data)...", serial)
    };
    if !begin_job(&state, &label) {
        return;
    }
    thread::spawn(move || {
        let res = (|| -> Result<(), String> {
            let (cfg, adb, fb) = {
                let s = state.lock().unwrap();
                (s.config.clone(), s.checks.adb_path.clone(), s.checks.fastboot_path.clone())
            };
            let adb = adb.ok_or("ADB not found. Download Platform Tools first.")?;
            if !provision_only {
                let fb = fb.ok_or("Fastboot not found. Download Platform Tools first.")?;
                if mode == DeviceMode::Fastboot {
                    flash_images(&state, &fb, &serial, &cfg.images_dir, wipe_userdata)?;
                } else if !wipe_userdata {
                    set_step(&state, 0.06, "Rebooting Pod into fastboot for upgrade...");
                    let _ = run_capture(&adb, &["-s", &serial, "reboot", "bootloader"]);
                    thread::sleep(Duration::from_secs(6));
                    flash_images(&state, &fb, &serial, &cfg.images_dir, false)?;
                } else {
                    state.lock().unwrap().push_log(LogLevel::Info, "Pod already booted in Android; resuming provisioning.");
                }
            }
            provision_pod(&state, &adb, &serial, &cfg, install_apps)
        })();
        finish_job(&state, res);
    });
}

pub fn spawn_backup(state: Arc<Mutex<SharedState>>, serial: String, partitions: Vec<String>) {
    if !begin_job(&state, &format!("Backing up partitions from {}...", serial)) {
        return;
    }
    thread::spawn(move || {
        let res = (|| -> Result<(), String> {
            let (cfg, adb) = {
                let s = state.lock().unwrap();
                (s.config.clone(), s.checks.adb_path.clone())
            };
            let adb = adb.ok_or("ADB not found.")?;
            set_step(&state, 0.08, "Requesting ADB root...");
            let _ = run_capture(&adb, &["-s", &serial, "root"]);
            thread::sleep(Duration::from_secs(2));
            let _ = run_capture(&adb, &["-s", &serial, "wait-for-device"]);
            let uid = run_capture(&adb, &["-s", &serial, "shell", "id -u"]).unwrap_or_default();
            if uid.trim() != "0" {
                return Err("ADB root required. Enable 'Rooted debugging' in Developer options on the Pod.".into());
            }

            let stamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
            let out_dir = PathBuf::from(&cfg.backups_dir).join(format!("{}-{}", serial, stamp));
            fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;

            let mut files = HashMap::new();
            let total = partitions.len().max(1);
            for (i, part) in partitions.iter().enumerate() {
                set_step(&state, 0.12 + (i as f32 / total as f32) * 0.80, format!("Backing up {}...", part));
                let blk_cmd = format!("ls /dev/block/bootdevice/by-name/{} 2>/dev/null", part);
                let blk = match run_capture(&adb, &["-s", &serial, "shell", &blk_cmd]) {
                    Ok(b) if !b.trim().is_empty() => b.trim().to_string(),
                    _ => {
                        state.lock().unwrap().push_log(LogLevel::Warn, format!("Skipping {}: not found", part));
                        continue;
                    }
                };
                let remote = format!("/data/local/tmp/bk_{}.img", part);
                let dd = format!("dd if={} of={} bs=4M 2>&1", blk, remote);
                run_capture(&adb, &["-s", &serial, "shell", &dd])?;
                let local = out_dir.join(format!("{}.img", part));
                run_stream(&state, &adb, &["-s", &serial, "pull", &remote, &local.to_string_lossy()])?;
                let _ = run_capture(&adb, &["-s", &serial, "shell", &format!("rm -f {}", remote)]);
                let digest = sha256_file(&local)?;
                let size = fs::metadata(&local).map(|m| m.len()).unwrap_or(0);
                files.insert(
                    format!("{}.img", part),
                    BackupFileMeta {
                        partition: part.clone(),
                        size,
                        sha256: digest.clone(),
                    },
                );
                state.lock().unwrap().push_log(
                    LogLevel::Ok,
                    format!("Backed up {}.img ({} bytes, sha256 {}...)", part, size, &digest[..12]),
                );
            }

            let manifest = BackupManifest {
                serial: serial.clone(),
                created_at: Local::now().to_rfc3339(),
                files,
            };
            let mtxt = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
            fs::write(out_dir.join("backup-manifest.json"), mtxt).map_err(|e| e.to_string())?;
            Ok(())
        })();
        finish_job(&state, res);
    });
}

pub fn spawn_restore(
    state: Arc<Mutex<SharedState>>,
    serial: String,
    backup_dir: PathBuf,
    include_sensitive: bool,
) {
    if !begin_job(&state, &format!("Restoring backup to {}...", serial)) {
        return;
    }
    thread::spawn(move || {
        let res = (|| -> Result<(), String> {
            let fb = state.lock().unwrap().checks.fastboot_path.clone().ok_or("Fastboot not found.")?;
            let manifest: Option<BackupManifest> = fs::read_to_string(backup_dir.join("backup-manifest.json"))
                .ok()
                .and_then(|t| serde_json::from_str(&t).ok());

            let mut imgs: Vec<(String, PathBuf)> = vec![];
            for entry in fs::read_dir(&backup_dir).map_err(|e| e.to_string())?.flatten() {
                let p = entry.path();
                if p.extension().and_then(|s| s.to_str()) == Some("img") {
                    let part = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
                    if SENSITIVE_PARTS.contains(&part.as_str()) && !include_sensitive {
                        state.lock().unwrap().push_log(LogLevel::Warn, format!("Skipping sensitive partition {} (safety lock)", part));
                        continue;
                    }
                    if let Some(ref m) = manifest {
                        let fname = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                        if let Some(meta) = m.files.get(&fname) {
                            set_step(&state, 0.12, format!("Verifying SHA-256 for {}...", fname));
                            let actual = sha256_file(&p)?;
                            if actual != meta.sha256 {
                                return Err(format!("SHA-256 mismatch for {}! Aborting restore.", fname));
                            }
                            state.lock().unwrap().push_log(LogLevel::Ok, format!("SHA-256 verified: {}", fname));
                        }
                    }
                    imgs.push((part, p));
                }
            }
            if imgs.is_empty() {
                return Err("No eligible .img files found to restore.".into());
            }
            let total = imgs.len();
            for (i, (part, img)) in imgs.iter().enumerate() {
                set_step(&state, 0.25 + (i as f32 / total as f32) * 0.65, format!("Restoring {}...", part));
                run_stream(&state, &fb, &["-s", &serial, "flash", part, &img.to_string_lossy()])?;
            }
            set_step(&state, 0.95, "Rebooting Pod...");
            run_stream(&state, &fb, &["-s", &serial, "reboot"])?;
            Ok(())
        })();
        finish_job(&state, res);
    });
}

pub fn spawn_app_job(
    state: Arc<Mutex<SharedState>>,
    app_ids: Vec<String>,
    force_download: bool,
    install_serial: Option<String>,
) {
    let label = if install_serial.is_some() {
        "Downloading & installing selected apps..."
    } else {
        "Downloading selected apps to APKs folder..."
    };
    if !begin_job(&state, label) {
        return;
    }
    thread::spawn(move || {
        let res = (|| -> Result<(), String> {
            let (cfg, adb) = {
                let s = state.lock().unwrap();
                (s.config.clone(), s.checks.adb_path.clone())
            };
            let apks_dir = PathBuf::from(&cfg.apks_dir);
            fs::create_dir_all(&apks_dir).map_err(|e| e.to_string())?;
            let selected: Vec<_> = catalog().into_iter().filter(|a| app_ids.contains(&a.id.to_string())).collect();
            if selected.is_empty() {
                return Err("Check at least one app first.".into());
            }

            let total = selected.len();
            let mut ready = vec![];
            for (i, app) in selected.iter().enumerate() {
                let dest = apks_dir.join(app.filename);
                let pct = 0.05 + (i as f32 / total as f32) * (if install_serial.is_some() { 0.50 } else { 0.90 });
                let is_cached = dest.is_file() && fs::metadata(&dest).map(|m| m.len() > 100_000).unwrap_or(false);
                if is_cached && !force_download {
                    set_step(&state, pct, format!("Cached: {}", app.name));
                    state.lock().unwrap().push_log(LogLevel::Ok, format!("[cached] {}", app.name));
                } else {
                    set_step(&state, pct, format!("Downloading {}...", app.name));
                    let url = resolve_download_url(app)?;
                    state.lock().unwrap().push_log(LogLevel::Info, format!("Downloading {}", url));
                    let bytes = download_to_file(&url, &dest)?;
                    state.lock().unwrap().push_log(
                        LogLevel::Ok,
                        format!("Saved {} ({:.1} MB)", app.filename, bytes as f64 / 1_048_576.0),
                    );
                }
                ready.push((app.clone(), dest));
            }

            if let Some(serial) = install_serial {
                let adb = adb.ok_or("ADB not found.")?;
                let mut set_proj = false;
                for (i, (app, path)) in ready.iter().enumerate() {
                    set_step(&state, 0.58 + (i as f32 / ready.len().max(1) as f32) * 0.36, format!("Installing {}...", app.name));
                    run_stream(&state, &adb, &["-s", &serial, "install", "-r", &path.to_string_lossy()])?;
                    if app.package == "com.spocky.projengmenu" {
                        set_proj = true;
                    }
                }
                if set_proj {
                    let _ = run_capture(
                        &adb,
                        &["-s", &serial, "shell", "cmd package set-home-activity com.spocky.projengmenu/.ui.home.MainActivity"],
                    );
                    state.lock().unwrap().push_log(LogLevel::Ok, "Set Projectivy Launcher as default TV home.");
                }
                let _ = run_capture(&adb, &["-s", &serial, "shell", "input keyevent KEYCODE_HOME"]);
            }
            Ok(())
        })();
        finish_job(&state, res);
    });
}

pub fn spawn_quick_action(state: Arc<Mutex<SharedState>>, serial: String, action: &'static str) {
    thread::spawn(move || {
        let (adb, fb) = {
            let s = state.lock().unwrap();
            (s.checks.adb_path.clone(), s.checks.fastboot_path.clone())
        };
        let res = match action {
            "reboot_fastboot" => adb
                .ok_or_else(|| "ADB not found".into())
                .and_then(|a| run_capture(&a, &["-s", &serial, "reboot", "bootloader"])),
            "reboot_android" => {
                if let Some(f) = fb {
                    if let Ok(o) = run_capture(&f, &["-s", &serial, "reboot"]) {
                        Ok(o)
                    } else if let Some(a) = adb {
                        run_capture(&a, &["-s", &serial, "reboot"])
                    } else {
                        Err("Reboot failed".into())
                    }
                } else if let Some(a) = adb {
                    run_capture(&a, &["-s", &serial, "reboot"])
                } else {
                    Err("No adb/fastboot found".into())
                }
            }
            "wifi_adb" => adb
                .ok_or_else(|| "ADB not found".into())
                .and_then(|a| run_capture(&a, &["-s", &serial, "tcpip", "5555"])),
            "mtp" => adb
                .ok_or_else(|| "ADB not found".into())
                .and_then(|a| run_capture(&a, &["-s", &serial, "shell", "svc usb setFunctions mtp true"])),
            _ => Err("Unknown action".into()),
        };
        let mut s = state.lock().unwrap();
        match res {
            Ok(msg) => s.push_log(LogLevel::Ok, format!("{}: {}", action, if msg.is_empty() { "OK" } else { &msg })),
            Err(e) => s.push_log(LogLevel::Error, format!("{} failed: {}", action, e)),
        }
    });
}
