#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backend;
mod catalog;

use backend::{
    save_config, spawn_app_job, spawn_backup, spawn_download_platform_tools,
    spawn_install_or_upgrade, spawn_quick_action, spawn_restore, start_poller, DeviceMode,
    JobStatus, LogLevel, SharedState,
};
use catalog::catalog;
use eframe::egui::{self, Color32, RichText, Rounding, Stroke, Vec2};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(PartialEq, Eq, Clone, Copy)]
enum Tab {
    Dashboard,
    Install,
    Apps,
    Backup,
    Settings,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum OsTarget {
    Android15Tv,
    LinuxDockerSoon,
}

struct PodToolApp {
    state: Arc<Mutex<SharedState>>,
    tab: Tab,
    os_target: OsTarget,
    selected_serial: String,
    install_apps_on_flash: bool,
    selected_app_ids: HashSet<String>,
    force_redownload: bool,
    bk_boot: bool,
    bk_recovery: bool,
    bk_persist: bool,
    bk_modem: bool,
    bk_bluetooth: bool,
    bk_aboot: bool,
    selected_backup_idx: usize,
    restore_sensitive: bool,
    confirm_sensitive: bool,
    show_logs: bool,
}

impl PodToolApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let state = Arc::new(Mutex::new(SharedState::new()));
        start_poller(Arc::clone(&state));
        let mut selected_app_ids = HashSet::new();
        for app in catalog() {
            if app.vanilla_default {
                selected_app_ids.insert(app.id.to_string());
            }
        }
        Self {
            state,
            tab: Tab::Dashboard,
            os_target: OsTarget::Android15Tv,
            selected_serial: String::new(),
            install_apps_on_flash: true,
            selected_app_ids,
            force_redownload: false,
            bk_boot: true,
            bk_recovery: true,
            bk_persist: true,
            bk_modem: true,
            bk_bluetooth: true,
            bk_aboot: false,
            selected_backup_idx: 0,
            restore_sensitive: false,
            confirm_sensitive: false,
            show_logs: true,
        }
    }

    fn apply_theme(ctx: &egui::Context, dark: bool) {
        let mut visuals = if dark {
            let mut v = egui::Visuals::dark();
            v.panel_fill = Color32::from_rgb(20, 24, 23);
            v.window_fill = Color32::from_rgb(26, 31, 29);
            v.faint_bg_color = Color32::from_rgb(31, 37, 35);
            v.widgets.noninteractive.bg_fill = Color32::from_rgb(30, 36, 34);
            v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(48, 57, 54));
            v.widgets.inactive.bg_fill = Color32::from_rgb(38, 46, 43);
            v.widgets.hovered.bg_fill = Color32::from_rgb(49, 60, 56);
            v.selection.bg_fill = Color32::from_rgb(46, 107, 79);
            v
        } else {
            let mut v = egui::Visuals::light();
            v.panel_fill = Color32::from_rgb(246, 244, 239);
            v.window_fill = Color32::from_rgb(255, 253, 248);
            v.faint_bg_color = Color32::from_rgb(239, 236, 228);
            v.widgets.noninteractive.bg_fill = Color32::from_rgb(255, 253, 248);
            v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(218, 213, 201));
            v.selection.bg_fill = Color32::from_rgb(46, 107, 79);
            v
        };
        visuals.window_rounding = Rounding::same(10.0);
        visuals.widgets.inactive.rounding = Rounding::same(7.0);
        visuals.widgets.hovered.rounding = Rounding::same(7.0);
        visuals.widgets.active.rounding = Rounding::same(7.0);
        ctx.set_visuals(visuals);
    }
}

fn pill(ui: &mut egui::Ui, text: &str, bg: Color32, fg: Color32) {
    egui::Frame::none()
        .fill(bg)
        .rounding(Rounding::same(12.0))
        .inner_margin(egui::Margin::symmetric(8.0, 2.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.5).strong().color(fg));
        });
}

fn card_frame(dark: bool) -> egui::Frame {
    egui::Frame::none()
        .fill(if dark {
            Color32::from_rgb(28, 34, 32)
        } else {
            Color32::from_rgb(255, 253, 248)
        })
        .stroke(Stroke::new(
            1.0_f32,
            if dark {
                Color32::from_rgb(48, 57, 54)
            } else {
                Color32::from_rgb(220, 215, 203)
            },
        ))
        .rounding(Rounding::same(10.0))
        .inner_margin(egui::Margin::same(14.0))
}

impl eframe::App for PodToolApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(std::time::Duration::from_millis(400));

        let (dark, checks, devices, backups, flash_state, job_status, job_step, job_progress, job_checks, logs, cfg_snap) = {
            let s = self.state.lock().unwrap();
            (
                s.config.dark_mode,
                s.checks.clone(),
                s.devices.clone(),
                s.backups.clone(),
                s.flash_state.clone(),
                s.job_status.clone(),
                s.job_step.clone(),
                s.job_progress,
                s.job_checks.clone(),
                s.logs.clone(),
                s.config.clone(),
            )
        };
        Self::apply_theme(ctx, dark);

        let busy = job_status == JobStatus::Running;
        let moss = Color32::from_rgb(46, 139, 97);
        let pond = Color32::from_rgb(56, 128, 184);
        let amber = Color32::from_rgb(214, 146, 42);
        let red = Color32::from_rgb(206, 78, 78);

        // Keep selected_serial synced to connected devices
        if !devices.is_empty() && !devices.iter().any(|d| d.serial == self.selected_serial) {
            self.selected_serial = devices[0].serial.clone();
        } else if devices.is_empty() {
            self.selected_serial.clear();
        }
        let active_dev = devices.iter().find(|d| d.serial == self.selected_serial).cloned();

        // --- LEFT SIDEBAR ---
        egui::SidePanel::left("sidebar")
            .exact_width(232.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.label(RichText::new("Pod Patcher").size(18.0).strong().color(moss));
                ui.label(RichText::new("Solstice Pod Gen3 (SP8096)").size(11.5).weak());
                ui.add_space(10.0);

                // Easy / Advanced mode toggle
                ui.label(RichText::new("MODE").size(10.5).strong().weak());
                ui.horizontal(|ui| {
                    let adv = cfg_snap.advanced_mode;
                    let easy_btn = egui::Button::new(
                        RichText::new("Easy").color(if !adv { Color32::WHITE } else { ui.visuals().text_color() }),
                    )
                    .fill(if !adv { moss } else { Color32::TRANSPARENT })
                    .min_size(Vec2::new(98.0, 26.0));
                    let adv_btn = egui::Button::new(
                        RichText::new("Advanced").color(if adv { Color32::WHITE } else { ui.visuals().text_color() }),
                    )
                    .fill(if adv { pond } else { Color32::TRANSPARENT })
                    .min_size(Vec2::new(98.0, 26.0));
                    if ui.add(easy_btn).clicked() && adv {
                        let mut s = self.state.lock().unwrap();
                        s.config.advanced_mode = false;
                        save_config(&s.config);
                    }
                    if ui.add(adv_btn).clicked() && !adv {
                        let mut s = self.state.lock().unwrap();
                        s.config.advanced_mode = true;
                        save_config(&s.config);
                    }
                });
                ui.add_space(10.0);

                // OS Target selector (Android 15 now, Linux/Docker slot ready)
                ui.label(RichText::new("OS TARGET").size(10.5).strong().weak());
                egui::ComboBox::from_id_salt("os_target")
                    .width(205.0)
                    .selected_text(match self.os_target {
                        OsTarget::Android15Tv => "Android 15 TV (Active)",
                        OsTarget::LinuxDockerSoon => "Linux / Docker (Coming Soon)",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.os_target, OsTarget::Android15Tv, "Android 15 TV (Active)");
                        ui.selectable_value(&mut self.os_target, OsTarget::LinuxDockerSoon, "Linux / Docker (Coming Soon)");
                    });
                ui.add_space(14.0);

                ui.label(RichText::new("NAVIGATION").size(10.5).strong().weak());
                ui.add_space(4.0);
                for (t, lbl) in [
                    (Tab::Dashboard, "Dashboard"),
                    (Tab::Install, "Install & Upgrade"),
                    (Tab::Apps, "App Picker"),
                    (Tab::Backup, "Backup & Restore"),
                    (Tab::Settings, "Paths & Settings"),
                ] {
                    let selected = self.tab == t;
                    let btn = egui::Button::new(
                        RichText::new(lbl)
                            .size(13.0)
                            .color(if selected { Color32::WHITE } else { ui.visuals().text_color() }),
                    )
                    .fill(if selected { moss } else { Color32::TRANSPARENT })
                    .min_size(Vec2::new(206.0, 32.0));
                    if ui.add(btn).clicked() {
                        self.tab = t;
                    }
                    ui.add_space(2.0);
                }

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label(RichText::new("LIVE SELF-CHECKS").size(10.5).strong().weak());
                ui.add_space(6.0);

                let pt_ok = checks.adb_path.is_some() && checks.fastboot_path.is_some();
                ui.horizontal(|ui| {
                    ui.label(RichText::new(if pt_ok { "OK" } else { "MISSING" }).strong().color(if pt_ok { moss } else { red }));
                    ui.label("Platform Tools");
                });
                if !pt_ok {
                    if ui.add_enabled(!busy, egui::Button::new("Download Platform Tools").fill(pond)).clicked() {
                        spawn_download_platform_tools(Arc::clone(&self.state));
                    }
                }

                let img_ok = checks.images_found == 3;
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{}/3", checks.images_found)).strong().color(if img_ok { moss } else { amber }));
                    ui.label("OS Images Ready");
                });

                if !checks.driver_warnings.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("WARN").strong().color(amber));
                        ui.label("Win USB Driver Needed");
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("OK").strong().color(moss));
                        ui.label(format!("OS: {}", checks.os_name));
                    });
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(8.0);
                    let theme_lbl = if dark { "Switch to Light Theme" } else { "Switch to Dark Theme" };
                    if ui.button(theme_lbl).clicked() {
                        let mut s = self.state.lock().unwrap();
                        s.config.dark_mode = !s.config.dark_mode;
                        save_config(&s.config);
                    }
                });
            });

        // --- BOTTOM PROGRESS & LOG DRAWER ---
        egui::TopBottomPanel::bottom("progress_drawer")
            .resizable(true)
            .min_height(if self.show_logs { 175.0 } else { 54.0 })
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let (st_txt, st_col) = match job_status {
                        JobStatus::Idle => ("IDLE", Color32::GRAY),
                        JobStatus::Running => ("RUNNING", pond),
                        JobStatus::Done => ("DONE", moss),
                        JobStatus::Error => ("ERROR", red),
                    };
                    pill(ui, st_txt, st_col, Color32::WHITE);
                    ui.label(RichText::new(&job_step).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(if self.show_logs { "Hide Log" } else { "Show Log" }).clicked() {
                            self.show_logs = !self.show_logs;
                        }
                        ui.label(format!("{}%", (job_progress * 100.0) as i32));
                    });
                });
                ui.add(
                    egui::ProgressBar::new(job_progress)
                        .desired_height(10.0)
                        .animate(busy),
                );
                if !job_checks.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        for c in &job_checks {
                            pill(
                                ui,
                                &format!("{}: {}", if c.ok { "PASS" } else { "FAIL" }, c.name),
                                if c.ok { moss } else { red },
                                Color32::WHITE,
                            );
                        }
                    });
                }
                if self.show_logs {
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .stick_to_bottom(true)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for entry in &logs {
                                let col = match entry.level {
                                    LogLevel::Ok => Color32::from_rgb(122, 224, 166),
                                    LogLevel::Warn => amber,
                                    LogLevel::Error => Color32::from_rgb(242, 139, 130),
                                    LogLevel::Info => ui.visuals().text_color(),
                                };
                                ui.label(RichText::new(format!("[{}] {}", entry.ts, entry.msg)).monospace().size(11.5).color(col));
                            }
                        });
                }
            });

        // --- MAIN WORKSPACE ---
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Windows driver warning banner
                if !checks.driver_warnings.is_empty() {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(78, 52, 16))
                        .stroke(Stroke::new(1.0_f32, amber))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(egui::Margin::same(10.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "Windows USB driver needed for {}. Open Device Manager -> right-click yellow Android device -> Update driver -> Browse -> Let me pick -> Android Bootloader Interface.",
                                    checks.driver_warnings.join(", ")
                                ))
                                .color(Color32::from_rgb(255, 232, 194)),
                            );
                        });
                    ui.add_space(10.0);
                }

                // Connected Pods header card
                card_frame(dark).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Connected Pods").size(15.0).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new("Auto-detecting every 2s").size(11.5).weak());
                        });
                    });
                    ui.add_space(6.0);
                    if devices.is_empty() {
                        ui.label(RichText::new("No Pods detected on USB. Plug into the front USB-C port (hold internal VOL- on power-on for Fastboot, or let Android 15 boot for ADB).").weak());
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            for d in &devices {
                                let is_sel = d.serial == self.selected_serial;
                                let frame = egui::Frame::none()
                                    .fill(if dark { Color32::from_rgb(35, 43, 40) } else { Color32::WHITE })
                                    .stroke(Stroke::new(if is_sel { 2.0_f32 } else { 1.0_f32 }, if is_sel { moss } else { Color32::GRAY }))
                                    .rounding(Rounding::same(8.0))
                                    .inner_margin(egui::Margin::same(10.0));
                                let resp = frame.show(ui, |ui| {
                                    ui.set_min_width(240.0);
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(&d.serial).monospace().strong().size(13.5));
                                        let (mtxt, mcol) = match d.mode {
                                            DeviceMode::Fastboot => ("FASTBOOT", amber),
                                            DeviceMode::Android => ("ANDROID", moss),
                                            DeviceMode::Recovery => ("RECOVERY", pond),
                                            DeviceMode::Unauthorized => ("UNAUTH", red),
                                        };
                                        pill(ui, mtxt, mcol, Color32::WHITE);
                                        if d.saved_state == "done" {
                                            pill(ui, "VERIFIED", moss, Color32::WHITE);
                                        }
                                    });
                                    if d.mode == DeviceMode::Fastboot {
                                        ui.label(RichText::new(format!("Product: {}", d.product)).size(11.5).weak());
                                    } else if d.mode == DeviceMode::Android {
                                        if !d.kernel.is_empty() {
                                            ui.label(RichText::new(format!("Android {} | {}", d.android_ver, d.kernel)).size(11.5).weak());
                                        }
                                        if !d.wifi_ip.is_empty() || !d.storage.is_empty() {
                                            ui.label(RichText::new(format!("IP: {} | {}", d.wifi_ip, d.storage)).size(11.5).weak());
                                        }
                                        ui.label(RichText::new(format!(
                                            "CEC: {} | Hot-plug: {}",
                                            if d.cec_on { "on" } else { "off" },
                                            if d.hotplug_on { "on" } else { "off" }
                                        )).size(11.5).weak());
                                    }
                                });
                                if resp.response.interact(egui::Sense::click()).clicked() {
                                    self.selected_serial = d.serial.clone();
                                }
                            }
                        });
                    }

                    ui.add_space(8.0);
                    let is_android = active_dev.as_ref().map(|d| d.mode == DeviceMode::Android).unwrap_or(false);
                    let has_dev = active_dev.is_some();
                    ui.horizontal_wrapped(|ui| {
                        if ui.add_enabled(!busy && is_android, egui::Button::new("Reboot to Fastboot")).clicked() {
                            spawn_quick_action(Arc::clone(&self.state), self.selected_serial.clone(), "reboot_fastboot");
                        }
                        if ui.add_enabled(!busy && has_dev, egui::Button::new("Reboot to Android")).clicked() {
                            spawn_quick_action(Arc::clone(&self.state), self.selected_serial.clone(), "reboot_android");
                        }
                        if cfg_snap.advanced_mode {
                            if ui.add_enabled(!busy && is_android, egui::Button::new("Enable Wi-Fi ADB (5555)")).clicked() {
                                spawn_quick_action(Arc::clone(&self.state), self.selected_serial.clone(), "wifi_adb");
                            }
                            if ui.add_enabled(!busy && is_android, egui::Button::new("Enable MTP + ADB")).clicked() {
                                spawn_quick_action(Arc::clone(&self.state), self.selected_serial.clone(), "mtp");
                            }
                        }
                    });
                });

                ui.add_space(12.0);

                if self.os_target == OsTarget::LinuxDockerSoon {
                    card_frame(dark).show(ui, |ui| {
                        ui.label(RichText::new("Linux / Docker Target (Roadmap)").size(16.0).strong());
                        ui.add_space(6.0);
                        ui.label("This slot is reserved for a mainline Linux / PostmarketOS / Debian image for SP8096 with container namespaces, cgroups, and overlayfs enabled so you can run Docker and homelab workloads natively on a Pod.");
                        ui.add_space(6.0);
                        ui.label(RichText::new("Switch OS Target back to Android 15 TV in the sidebar to flash or manage TV Pods today.").weak());
                    });
                    return;
                }

                // --- ACTIVE TAB ---
                match self.tab {
                    Tab::Dashboard => {
                        // 1. Quick Start Guide
                        card_frame(dark).show(ui, |ui| {
                            ui.label(RichText::new("Quick Start").size(15.5).strong());
                            ui.add_space(6.0);
                            ui.label("1. Enter Fastboot: connect the Pod's front USB-C port to this computer, hold internal VOL- (S3), and plug in 12 V power for 3 to 5 seconds.");
                            ui.label("2. Back up factory partitions (optional on stock Pods): use Backup & Restore to save boot, recovery, persist, modem, and bluetooth with SHA-256 verification.");
                            ui.label("3. Install or Upgrade: open Install & Upgrade for a clean flash (or keep-data upgrade), and pick optional upstream apps in App Picker.");
                            ui.add_space(8.0);
                            ui.horizontal_wrapped(|ui| {
                                if ui.button("Go to Install & Upgrade").clicked() {
                                    self.tab = Tab::Install;
                                }
                                if ui.button("Open App Picker").clicked() {
                                    self.tab = Tab::Apps;
                                }
                                if ui.button("Open Backup & Restore").clicked() {
                                    self.tab = Tab::Backup;
                                }
                            });
                        });

                        ui.add_space(10.0);

                        // 2. Selected Pod Live Health
                        card_frame(dark).show(ui, |ui| {
                            ui.label(RichText::new("Selected Pod Details").size(15.0).strong());
                            ui.add_space(6.0);
                            if let Some(ref d) = active_dev {
                                ui.label(format!("Serial: {}", d.serial));
                                ui.label(format!("Mode: {:?}", d.mode));
                                if d.mode == DeviceMode::Android {
                                    if !d.android_ver.is_empty() { ui.label(format!("Android Version: {}", d.android_ver)); }
                                    if !d.kernel.is_empty() { ui.label(format!("Kernel: {}", d.kernel)); }
                                    if !d.wifi_ip.is_empty() { ui.label(format!("Wi-Fi IP: {}", d.wifi_ip)); }
                                    if !d.storage.is_empty() { ui.label(format!("Internal Storage: {}", d.storage)); }
                                    if !d.home_app.is_empty() { ui.label(format!("Default Home: {}", d.home_app)); }
                                    ui.label(format!(
                                        "Background Services: HDMI-CEC {} | HDMI Hot-Plug {}",
                                        if d.cec_on { "Running" } else { "Stopped" },
                                        if d.hotplug_on { "Running" } else { "Stopped" }
                                    ));
                                } else if d.mode == DeviceMode::Fastboot {
                                    ui.label(format!("Fastboot Product: {}", d.product));
                                    ui.label(RichText::new("Ready for clean install, keep-data upgrade, or backup restore.").weak());
                                }
                            } else {
                                ui.label(RichText::new("Connect a Pod to inspect live hardware, kernel, network, and service health.").weak());
                            }
                        });

                        ui.add_space(10.0);

                        // 3. Flashed Pod Fleet History
                        card_frame(dark).show(ui, |ui| {
                            let done_cnt = flash_state.values().filter(|v| v.as_str() == "done").count();
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Flashed Pod Fleet").size(15.0).strong());
                                pill(ui, &format!("{} verified", done_cnt), moss, Color32::WHITE);
                            });
                            ui.add_space(6.0);
                            if flash_state.is_empty() {
                                ui.label(RichText::new("No Pods flashed by Pod Patcher yet. Verified serials will be tracked here automatically.").weak());
                            } else {
                                let mut entries: Vec<_> = flash_state.iter().collect();
                                entries.sort_by(|a, b| a.0.cmp(b.0));
                                for (ser, st) in entries {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(ser).monospace().strong());
                                        if st == "done" {
                                            pill(ui, "VERIFIED", moss, Color32::WHITE);
                                        } else {
                                            pill(ui, st, red, Color32::WHITE);
                                        }
                                    });
                                }
                            }
                        });
                    }
                    Tab::Install => {
                        card_frame(dark).show(ui, |ui| {
                            ui.label(RichText::new("Install, Upgrade & Resume").size(15.5).strong());
                            ui.add_space(6.0);
                            ui.label(RichText::new("Never touches persist, modem, or bt_firmware (preserves each Pod's factory MAC addresses and calibration).").weak());
                            ui.add_space(10.0);

                            ui.checkbox(&mut self.install_apps_on_flash, "Auto-install cached APKs from your TV APKs folder");

                            if cfg_snap.advanced_mode {
                                ui.add_space(10.0);
                                ui.separator();
                                ui.add_space(6.0);
                                ui.label(RichText::new("Granular Provisioning Options (Advanced)").size(13.0).strong().color(pond));
                                ui.add_space(4.0);
                                let mut s = self.state.lock().unwrap();
                                let mut changed = false;
                                changed |= ui.checkbox(&mut s.config.opt_skip_setup, "Skip phone setup wizard, lockscreen, and sleep timer").changed();
                                changed |= ui.checkbox(&mut s.config.opt_hide_battery, "Hide battery icon (wall-powered device)").changed();
                                changed |= ui.checkbox(&mut s.config.opt_lock_landscape, "Lock landscape orientation for TV").changed();
                                changed |= ui.checkbox(&mut s.config.opt_hide_status_bar, "Hide phone status bar (immersive.status=*)").changed();
                                changed |= ui.checkbox(&mut s.config.opt_disable_phone_apps, "Disable 12 leftover phone & stock Mersive apps").changed();
                                changed |= ui.checkbox(&mut s.config.opt_usb_mtp, "Enable USB MTP file transfer alongside ADB (opt-in)").changed();
                                changed |= ui.checkbox(&mut s.config.opt_wifi_adb, "Enable Wireless ADB on port 5555 (opt-in; opens LAN debugging)").changed();
                                if changed {
                                    save_config(&s.config);
                                }
                            } else {
                                ui.add_space(4.0);
                                ui.label(RichText::new("Easy Mode uses safe TV defaults (Wireless ADB & USB MTP stay off unless enabled in Advanced Mode).").size(11.5).weak());
                            }
                            ui.add_space(12.0);

                            let has_dev = active_dev.is_some();
                            let is_android = active_dev.as_ref().map(|d| d.mode == DeviceMode::Android).unwrap_or(false);
                            ui.horizontal_wrapped(|ui| {
                                if ui.add_enabled(!busy && has_dev, egui::Button::new(RichText::new("Start Clean Install / Resume").color(Color32::WHITE)).fill(moss).min_size(Vec2::new(200.0, 34.0))).clicked() {
                                    if let Some(ref d) = active_dev {
                                        spawn_install_or_upgrade(Arc::clone(&self.state), d.serial.clone(), d.mode.clone(), true, self.install_apps_on_flash, false);
                                    }
                                }
                                if ui.add_enabled(!busy && has_dev, egui::Button::new(RichText::new("Upgrade OS (Keep Data)").color(Color32::WHITE)).fill(pond).min_size(Vec2::new(180.0, 34.0))).clicked() {
                                    if let Some(ref d) = active_dev {
                                        spawn_install_or_upgrade(Arc::clone(&self.state), d.serial.clone(), d.mode.clone(), false, false, false);
                                    }
                                }
                                if cfg_snap.advanced_mode {
                                    if ui.add_enabled(!busy && is_android, egui::Button::new("Provision & Verify Only (ADB)").min_size(Vec2::new(180.0, 34.0))).clicked() {
                                        if let Some(ref d) = active_dev {
                                            spawn_install_or_upgrade(Arc::clone(&self.state), d.serial.clone(), d.mode.clone(), false, self.install_apps_on_flash, true);
                                        }
                                    }
                                }
                            });
                        });
                    }
                    Tab::Apps => {
                        card_frame(dark).show(ui, |ui| {
                            ui.label(RichText::new("App Picker (Official Upstream Releases)").size(15.5).strong());
                            ui.label(RichText::new("Vanilla default: Projectivy Launcher + Aptoide TV. All other vetted open-source/official apps are opt-in. Zero APKs are bundled in the repo.").size(12.0).weak());
                            ui.add_space(8.0);

                            let all = catalog();
                            let apks_dir = PathBuf::from(&cfg_snap.apks_dir);
                            ui.horizontal_wrapped(|ui| {
                                if ui.button("Vanilla Default (2)").clicked() {
                                    self.selected_app_ids.clear();
                                    for a in &all { if a.vanilla_default { self.selected_app_ids.insert(a.id.to_string()); } }
                                }
                                if ui.button("Select Cached Only").clicked() {
                                    self.selected_app_ids.clear();
                                    for a in &all {
                                        if apks_dir.join(a.filename).is_file() {
                                            self.selected_app_ids.insert(a.id.to_string());
                                        }
                                    }
                                }
                                if ui.button("Clear All").clicked() {
                                    self.selected_app_ids.clear();
                                }
                                ui.checkbox(&mut self.force_redownload, "Force re-download latest");
                            });
                            ui.add_space(8.0);

                            let mut by_cat: BTreeMap<&str, Vec<_>> = BTreeMap::new();
                            for a in &all {
                                by_cat.entry(a.category).or_default().push(a);
                            }

                            for (cname, list) in by_cat {
                                ui.label(RichText::new(cname).strong().color(moss).size(13.5));
                                ui.separator();
                                for app in list {
                                    let fpath = apks_dir.join(app.filename);
                                    let cached_sz = fs::metadata(&fpath).ok().filter(|m| m.len() > 100_000).map(|m| m.len() as f32 / 1_048_576.0);
                                    let mut checked = self.selected_app_ids.contains(app.id);
                                    ui.horizontal(|ui| {
                                        if ui.checkbox(&mut checked, RichText::new(app.name).strong()).changed() {
                                            if checked { self.selected_app_ids.insert(app.id.to_string()); } else { self.selected_app_ids.remove(app.id); }
                                        }
                                        if let Some(mb) = cached_sz {
                                            pill(ui, &format!("Cached {:.1} MB", mb), moss, Color32::WHITE);
                                        } else {
                                            pill(ui, "Upstream", Color32::from_rgb(70, 78, 75), Color32::WHITE);
                                        }
                                        ui.label(RichText::new(format!("{} | {}", app.license, app.popularity)).size(11.0).weak());
                                    });
                                    ui.label(RichText::new(format!("    {}", app.desc)).size(11.5).weak());
                                    ui.add_space(3.0);
                                }
                                ui.add_space(8.0);
                            }

                            let is_android = active_dev.as_ref().map(|d| d.mode == DeviceMode::Android).unwrap_or(false);
                            ui.horizontal_wrapped(|ui| {
                                if ui.add_enabled(!busy, egui::Button::new(RichText::new("Download Selected to APKs Folder").color(Color32::WHITE)).fill(pond).min_size(Vec2::new(220.0, 34.0))).clicked() {
                                    spawn_app_job(Arc::clone(&self.state), self.selected_app_ids.iter().cloned().collect(), self.force_redownload, None);
                                }
                                if ui.add_enabled(!busy && is_android, egui::Button::new(RichText::new("Download & Install on Pod").color(Color32::WHITE)).fill(moss).min_size(Vec2::new(200.0, 34.0))).clicked() {
                                    spawn_app_job(Arc::clone(&self.state), self.selected_app_ids.iter().cloned().collect(), self.force_redownload, Some(self.selected_serial.clone()));
                                }
                            });
                        });
                    }
                    Tab::Backup => {
                        card_frame(dark).show(ui, |ui| {
                            ui.label(RichText::new("Backup Partitions (ADB Root)").size(15.0).strong());
                            ui.label(RichText::new("Saves raw .img files + verified SHA-256 manifest into a timestamped folder per Pod serial.").weak());
                            ui.add_space(6.0);
                            if cfg_snap.advanced_mode {
                                ui.horizontal_wrapped(|ui| {
                                    ui.checkbox(&mut self.bk_boot, "boot");
                                    ui.checkbox(&mut self.bk_recovery, "recovery");
                                    ui.checkbox(&mut self.bk_persist, "persist");
                                    ui.checkbox(&mut self.bk_modem, "modem");
                                    ui.checkbox(&mut self.bk_bluetooth, "bluetooth");
                                    ui.checkbox(&mut self.bk_aboot, "aboot");
                                });
                            } else {
                                ui.label(RichText::new("Easy Mode backs up the 5 standard partitions: boot, recovery, persist, modem, and bluetooth.").size(11.5).weak());
                            }
                            ui.add_space(6.0);
                            let is_android = active_dev.as_ref().map(|d| d.mode == DeviceMode::Android).unwrap_or(false);
                            if ui.add_enabled(!busy && is_android, egui::Button::new(RichText::new("Backup Selected Pod").color(Color32::WHITE)).fill(moss)).clicked() {
                                let mut parts = vec![];
                                if self.bk_boot { parts.push("boot".into()); }
                                if self.bk_recovery { parts.push("recovery".into()); }
                                if self.bk_persist { parts.push("persist".into()); }
                                if self.bk_modem { parts.push("modem".into()); }
                                if self.bk_bluetooth { parts.push("bluetooth".into()); }
                                if self.bk_aboot { parts.push("aboot".into()); }
                                spawn_backup(Arc::clone(&self.state), self.selected_serial.clone(), parts);
                            }
                        });

                        ui.add_space(10.0);
                        card_frame(dark).show(ui, |ui| {
                            ui.label(RichText::new("Restore from Backup (Fastboot)").size(15.0).strong());
                            ui.label(RichText::new("Verifies SHA-256 hashes before flashing. Sensitive factory partitions (persist/modem/bluetooth) are skipped by default.").weak());
                            ui.add_space(6.0);
                            if backups.is_empty() {
                                ui.label(RichText::new("No backups found in your configured backups folder yet.").weak());
                            } else {
                                if self.selected_backup_idx >= backups.len() {
                                    self.selected_backup_idx = 0;
                                }
                                egui::ComboBox::from_id_salt("backup_pick")
                                    .width(380.0)
                                    .selected_text(format!("{} ({} files)", backups[self.selected_backup_idx].name, backups[self.selected_backup_idx].file_count))
                                    .show_ui(ui, |ui| {
                                        for (idx, b) in backups.iter().enumerate() {
                                            ui.selectable_value(&mut self.selected_backup_idx, idx, format!("{} ({} files)", b.name, b.file_count));
                                        }
                                    });
                                ui.add_space(6.0);
                                if cfg_snap.advanced_mode {
                                    ui.checkbox(&mut self.restore_sensitive, RichText::new("Include sensitive factory partitions (persist/modem/bluetooth)").color(red));
                                    if self.restore_sensitive {
                                        ui.checkbox(&mut self.confirm_sensitive, RichText::new("I confirm this backup belongs to this exact physical Pod").color(red));
                                    }
                                } else {
                                    self.restore_sensitive = false;
                                    self.confirm_sensitive = false;
                                    ui.label(RichText::new("Easy Mode safety lock: only restores boot/recovery/system (switch to Advanced Mode to unlock factory partitions).").size(11.5).weak());
                                }
                                ui.add_space(6.0);
                                let is_fb = active_dev.as_ref().map(|d| d.mode == DeviceMode::Fastboot).unwrap_or(false);
                                let allowed = !self.restore_sensitive || self.confirm_sensitive;
                                if ui.add_enabled(!busy && is_fb && allowed, egui::Button::new(RichText::new("Restore to Fastboot Pod").color(Color32::WHITE)).fill(red)).clicked() {
                                    let p = backups[self.selected_backup_idx].path.clone();
                                    spawn_restore(Arc::clone(&self.state), self.selected_serial.clone(), p, self.restore_sensitive);
                                }
                            }
                        });
                    }
                    Tab::Settings => {
                        card_frame(dark).show(ui, |ui| {
                            ui.label(RichText::new("Paths & Settings").size(15.5).strong());
                            ui.add_space(8.0);
                            let mut s = self.state.lock().unwrap();
                            let mut changed = false;

                            let mut row = |ui: &mut egui::Ui, label: &str, val: &mut String| {
                                ui.label(RichText::new(label).size(12.0).strong());
                                ui.horizontal(|ui| {
                                    if ui.add(egui::TextEdit::singleline(val).desired_width(460.0)).changed() {
                                        changed = true;
                                    }
                                    if ui.button("Browse...").clicked() {
                                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                            *val = folder.to_string_lossy().to_string();
                                            changed = true;
                                        }
                                    }
                                });
                                ui.add_space(6.0);
                            };

                            row(ui, "OS Images Folder (boot, recovery, system.simg)", &mut s.config.images_dir);
                            row(ui, "Platform Tools Folder (optional; leave blank for auto-detect/download)", &mut s.config.platform_tools_dir);
                            row(ui, "TV APKs Folder (used by App Picker & Clean Install)", &mut s.config.apks_dir);
                            row(ui, "Backups Folder", &mut s.config.backups_dir);

                            ui.label(RichText::new("Optional Wi-Fi SSID (auto-connect after install)").size(12.0).strong());
                            if ui.add(egui::TextEdit::singleline(&mut s.config.wifi_ssid).desired_width(300.0)).changed() {
                                changed = true;
                            }
                            ui.add_space(4.0);
                            ui.label(RichText::new("Optional Wi-Fi Password").size(12.0).strong());
                            if ui.add(egui::TextEdit::singleline(&mut s.config.wifi_password).password(true).desired_width(300.0)).changed() {
                                changed = true;
                            }

                            if changed {
                                save_config(&s.config);
                            }
                        });
                    }
                }
            });
        });
    }
}

fn load_app_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}

fn main() -> eframe::Result<()> {
    let mut vp = egui::ViewportBuilder::default()
        .with_title("Pod Patcher")
        .with_inner_size([1080.0, 760.0])
        .with_min_inner_size([880.0, 620.0]);
    if let Some(icon) = load_app_icon() {
        vp = vp.with_icon(Arc::new(icon));
    }
    let options = eframe::NativeOptions {
        viewport: vp,
        ..Default::default()
    };
    eframe::run_native(
        "Pod Patcher",
        options,
        Box::new(|cc| Ok(Box::new(PodToolApp::new(cc)))),
    )
}
