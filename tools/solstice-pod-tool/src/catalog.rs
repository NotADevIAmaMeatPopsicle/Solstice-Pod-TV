use regex::Regex;
use std::fs::{self, File};
use std::io;
use std::path::Path;

#[derive(Clone, Debug)]
pub enum AppSource {
    GitHub { repo: &'static str, asset_regex: &'static str },
    DirectUrl(&'static str),
    Aptoide(&'static str),
    Plex,
}

#[derive(Clone, Debug)]
pub struct CatalogApp {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub desc: &'static str,
    pub package: &'static str,
    pub filename: &'static str,
    pub license: &'static str,
    pub popularity: &'static str,
    pub vanilla_default: bool,
    pub source: AppSource,
}

/// Curated, vetted official upstream catalog (zero APKs bundled in repo).
/// Vanilla default: Projectivy Launcher + Aptoide TV only; all others are opt-in.
pub fn catalog() -> Vec<CatalogApp> {
    vec![
        // --- Launchers ---
        CatalogApp {
            id: "projectivy",
            name: "Projectivy Launcher",
            category: "Launchers",
            desc: "Ad-free TV home screen with custom wallpapers; auto-set as default home.",
            package: "com.spocky.projengmenu",
            filename: "projectivy.apk",
            license: "Freeware",
            popularity: "2.4k GH / 1M+ Play",
            vanilla_default: true,
            source: AppSource::GitHub {
                repo: "spocky/miproja1",
                asset_regex: r"ProjectivyLauncher.*\.apk$",
            },
        },
        CatalogApp {
            id: "flauncher",
            name: "FLauncher",
            category: "Launchers",
            desc: "100% open-source, minimalist, zero-telemetry Android TV launcher.",
            package: "me.efesser.flauncher",
            filename: "flauncher.apk",
            license: "GPL-3.0",
            popularity: "Open Source",
            vanilla_default: false,
            source: AppSource::DirectUrl(
                "https://gitlab.com/flauncher/flauncher/-/releases/permalink/latest/downloads/app-release.apk",
            ),
        },
        // --- App Stores ---
        CatalogApp {
            id: "aptoide-tv",
            name: "Aptoide TV",
            category: "App Stores",
            desc: "TV-friendly store (skip account login; stick to green Trusted badges).",
            package: "cm.aptoidetv.pt",
            filename: "aptoide-tv.apk",
            license: "Proprietary",
            popularity: "10M+ users",
            vanilla_default: true,
            source: AppSource::Aptoide("cm.aptoidetv.pt"),
        },
        CatalogApp {
            id: "fdroid",
            name: "F-Droid",
            category: "App Stores",
            desc: "Reproducible open-source Android app catalog.",
            package: "org.fdroid.fdroid",
            filename: "fdroid.apk",
            license: "GPL-3.0",
            popularity: "FOSS Standard",
            vanilla_default: false,
            source: AppSource::DirectUrl("https://f-droid.org/F-Droid.apk"),
        },
        CatalogApp {
            id: "obtainium",
            name: "Obtainium",
            category: "App Stores",
            desc: "Installs and updates apps directly from official GitHub/GitLab releases.",
            package: "dev.imranr.obtainium",
            filename: "obtainium.apk",
            license: "GPL-3.0",
            popularity: "21.9k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "ImranR98/Obtainium",
                asset_regex: r"app-arm64-v8a-release\.apk$",
            },
        },
        CatalogApp {
            id: "droidify",
            name: "Droid-ify",
            category: "App Stores",
            desc: "Fast, modern Material UI client for F-Droid repositories.",
            package: "com.looker.droidify",
            filename: "droidify.apk",
            license: "GPL-3.0",
            popularity: "7.8k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "Droid-ify/client",
                asset_regex: r"\.apk$",
            },
        },
        // --- Media & Streaming ---
        CatalogApp {
            id: "plex",
            name: "Plex",
            category: "Media & Streaming",
            desc: "Official Plex arm64 client for local media servers and live TV.",
            package: "com.plexapp.android",
            filename: "plex.apk",
            license: "Commercial",
            popularity: "10M+ Play",
            vanilla_default: false,
            source: AppSource::Plex,
        },
        CatalogApp {
            id: "jellyfin",
            name: "Jellyfin for Android TV",
            category: "Media & Streaming",
            desc: "Open-source self-hosted media client with 4K hardware video decode.",
            package: "org.jellyfin.androidtv",
            filename: "jellyfin-tv.apk",
            license: "GPL-2.0",
            popularity: "4.8k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "jellyfin/jellyfin-androidtv",
                asset_regex: r"jellyfin-androidtv-.*-release\.apk$",
            },
        },
        CatalogApp {
            id: "wholphin",
            name: "Wholphin (Jellyfin TV)",
            category: "Media & Streaming",
            desc: "Modern open-source Android TV client for Jellyfin servers.",
            package: "com.github.damontecres.wholphin",
            filename: "wholphin.apk",
            license: "GPL-2.0",
            popularity: "3.4k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "damontecres/Wholphin",
                asset_regex: r"\.apk$",
            },
        },
        CatalogApp {
            id: "tizentubecobalt",
            name: "TizenTubeCobalt",
            category: "Media & Streaming",
            desc: "Open-source ad-free YouTube TV client built on Cobalt.",
            package: "com.fgl27.twitch",
            filename: "tizentube-cobalt.apk",
            license: "BSD-3-Clause",
            popularity: "3.6k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "reisxd/TizenTubeCobalt",
                asset_regex: r"\.apk$",
            },
        },
        CatalogApp {
            id: "just-video-player",
            name: "Just Video Player",
            category: "Media & Streaming",
            desc: "Ultra-lightweight ExoPlayer video player (public domain).",
            package: "com.brouken.player",
            filename: "just-video-player.apk",
            license: "Unlicense",
            popularity: "2.8k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "moneytoo/Player",
                asset_regex: r"\.apk$",
            },
        },
        CatalogApp {
            id: "nuvio",
            name: "Nuvio",
            category: "Media & Streaming",
            desc: "Active open-source Stremio-style media streaming hub.",
            package: "com.tapframe.nuvio",
            filename: "nuvio.apk",
            license: "GPL-3.0",
            popularity: "4.9k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "tapframe/NuvioStreaming",
                asset_regex: r"\.apk$",
            },
        },
        CatalogApp {
            id: "nova",
            name: "Nova Video Player",
            category: "Media & Streaming",
            desc: "Open-source Android TV movie/show player with SMB/NFS scraping.",
            package: "org.courville.nova",
            filename: "nova-video.apk",
            license: "Apache-2.0",
            popularity: "4.8k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "nova-video-player/aos-AVP",
                asset_regex: r"arm64-v8a.*\.apk$|\.apk$",
            },
        },
        CatalogApp {
            id: "finamp",
            name: "Finamp",
            category: "Media & Streaming",
            desc: "Clean open-source Jellyfin music player.",
            package: "com.unicornsonlsd.finamp",
            filename: "finamp.apk",
            license: "MPL-2.0",
            popularity: "4.6k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "jmshrv/finamp",
                asset_regex: r"arm64-v8a.*\.apk$|\.apk$",
            },
        },
        CatalogApp {
            id: "antennapod",
            name: "AntennaPod",
            category: "Media & Streaming",
            desc: "Open-source podcast manager and player.",
            package: "de.danoeh.antennapod",
            filename: "antennapod.apk",
            license: "GPL-3.0",
            popularity: "8.4k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "AntennaPod/AntennaPod",
                asset_regex: r"\.apk$",
            },
        },
        CatalogApp {
            id: "aerialviews",
            name: "Aerial Views",
            category: "Media & Streaming",
            desc: "Apple TV-style 4K HDR aerial screensavers when idle.",
            package: "com.neilturner.aerialviews",
            filename: "aerial-views.apk",
            license: "Apache-2.0",
            popularity: "1.2k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "theothernt/AerialViews",
                asset_regex: r"\.apk$",
            },
        },
        // --- Gaming & Emulation ---
        CatalogApp {
            id: "better-xcloud",
            name: "Better xCloud",
            category: "Gaming & Emulation",
            desc: "Open-source Xbox Remote Play and Xbox Cloud Gaming TV client.",
            package: "com.redphx.betterxc",
            filename: "better-xcloud.apk",
            license: "MIT",
            popularity: "1M+ DL",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "redphx/better-xcloud-android",
                asset_regex: r"\.apk$",
            },
        },
        CatalogApp {
            id: "moonlight",
            name: "Moonlight",
            category: "Gaming & Emulation",
            desc: "Low-latency PC game streaming client for Sunshine / GameStream.",
            package: "com.limelight",
            filename: "moonlight.apk",
            license: "GPL-3.0",
            popularity: "7.6k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "moonlight-stream/moonlight-android",
                asset_regex: r"app-nonRoot-release\.apk$",
            },
        },
        CatalogApp {
            id: "snes9x-ex-plus",
            name: "Snes9x EX+",
            category: "Gaming & Emulation",
            desc: "Fast, controller-ready Super Nintendo emulator.",
            package: "com.explusalpha.Snes9xPlus",
            filename: "snes9x-ex-plus.apk",
            license: "GPL-3.0",
            popularity: "1.3k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "Rakashazi/emu-ex-plus-alpha",
                asset_regex: r"Snes9xEXPlus.*\.apk$",
            },
        },
        CatalogApp {
            id: "duckstation",
            name: "DuckStation",
            category: "Gaming & Emulation",
            desc: "High-accuracy PlayStation 1 emulator with upscaling.",
            package: "com.github.stenzek.duckstation",
            filename: "duckstation.apk",
            license: "GPL-3.0",
            popularity: "11.1k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "stenzek/duckstation",
                asset_regex: r"duckstation-.*-arm64\.apk$|\.apk$",
            },
        },
        CatalogApp {
            id: "retroarch",
            name: "RetroArch (64-bit)",
            category: "Gaming & Emulation",
            desc: "All-in-one multi-console emulator with TV-friendly gamepad UI.",
            package: "com.retroarch.aarch64",
            filename: "retroarch-aarch64.apk",
            license: "GPL-3.0",
            popularity: "14.7k stars",
            vanilla_default: false,
            source: AppSource::DirectUrl(
                "https://buildbot.libretro.com/stable/1.20.0/android/RetroArch_aarch64.apk",
            ),
        },
        // --- Homelab & Linux ---
        CatalogApp {
            id: "termux",
            name: "Termux",
            category: "Homelab & Linux",
            desc: "Full Linux shell + proot-distro (Debian/Ubuntu/Alpine), SSH, Python, Node.",
            package: "com.termux",
            filename: "termux.apk",
            license: "GPL-3.0",
            popularity: "65.8k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "termux/termux-app",
                asset_regex: r"termux-app_.*github-debug_arm64-v8a\.apk$|arm64-v8a.*\.apk$",
            },
        },
        CatalogApp {
            id: "termux-boot",
            name: "Termux:Boot",
            category: "Homelab & Linux",
            desc: "Auto-starts Termux scripts and daemons when the Pod boots.",
            package: "com.termux.boot",
            filename: "termux-boot.apk",
            license: "GPL-3.0",
            popularity: "1.9k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "termux/termux-boot",
                asset_regex: r"\.apk$",
            },
        },
        CatalogApp {
            id: "home-assistant",
            name: "Home Assistant",
            category: "Homelab & Linux",
            desc: "Smart-home dashboard and companion app.",
            package: "io.homeassistant.companion.android",
            filename: "home-assistant.apk",
            license: "Apache-2.0",
            popularity: "4.1k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "home-assistant/android",
                asset_regex: r"app-minimal-release\.apk$|\.apk$",
            },
        },
        CatalogApp {
            id: "syncthing",
            name: "Syncthing-Fork",
            category: "Homelab & Linux",
            desc: "Continuous P2P folder sync for ROMs, saves, and media.",
            package: "com.github.catfriend1.syncthingandroid",
            filename: "syncthing.apk",
            license: "MPL-2.0",
            popularity: "3.4k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "Catfriend1/syncthing-android",
                asset_regex: r"\.apk$",
            },
        },
        CatalogApp {
            id: "rethinkdns",
            name: "RethinkDNS",
            category: "Homelab & Linux",
            desc: "Open-source on-device firewall and DNS ad/tracker blocker.",
            package: "com.celzero.bravedns",
            filename: "rethinkdns.apk",
            license: "Apache-2.0",
            popularity: "6.0k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "celzero/rethink-app",
                asset_regex: r"\.apk$",
            },
        },
        CatalogApp {
            id: "wgtunnel",
            name: "WG Tunnel",
            category: "Homelab & Linux",
            desc: "Feature-rich open-source WireGuard and AmneziaWG VPN client.",
            package: "com.zaneschepke.wireguardautotunnel",
            filename: "wg-tunnel.apk",
            license: "MIT",
            popularity: "3.4k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "zaneschepke/wgtunnel",
                asset_regex: r"arm64.*\.apk$|\.apk$",
            },
        },
        CatalogApp {
            id: "round-sync",
            name: "Round Sync (Rclone)",
            category: "Homelab & Linux",
            desc: "Mount and sync S3, SFTP, WebDAV, or NAS storage via rclone.",
            package: "de.felixnuesse.extract",
            filename: "round-sync.apk",
            license: "GPL-3.0",
            popularity: "2.4k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "newhinton/Round-Sync",
                asset_regex: r"\.apk$",
            },
        },
        // --- Utilities & Casting ---
        CatalogApp {
            id: "localsend",
            name: "LocalSend",
            category: "Utilities & Casting",
            desc: "AirDrop-style local Wi-Fi file sharing across Mac/PC/Linux/phone.",
            package: "org.localsend.localsend_app",
            filename: "localsend.apk",
            license: "Apache-2.0",
            popularity: "101k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "localsend/localsend",
                asset_regex: r"LocalSend-.*-android-arm64v8\.apk$",
            },
        },
        CatalogApp {
            id: "material-files",
            name: "Material Files",
            category: "Utilities & Casting",
            desc: "File manager with built-in FTP/SFTP/SMB server.",
            package: "me.zhanghai.android.files",
            filename: "material-files.apk",
            license: "GPL-3.0",
            popularity: "9.6k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "zhanghai/MaterialFiles",
                asset_regex: r"\.apk$",
            },
        },
        CatalogApp {
            id: "keymapper",
            name: "Key Mapper",
            category: "Utilities & Casting",
            desc: "Remap TV remote, Flirc, or gamepad buttons.",
            package: "io.github.sds100.keymapper",
            filename: "keymapper.apk",
            license: "GPL-3.0",
            popularity: "2.9k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "keymapperorg/KeyMapper",
                asset_regex: r"\.apk$",
            },
        },
        CatalogApp {
            id: "tv-bro",
            name: "TV Bro",
            category: "Utilities & Casting",
            desc: "Open-source TV remote web browser with built-in ad blocker.",
            package: "com.phlox.tvwebbrowser",
            filename: "tv-bro.apk",
            license: "Apache-2.0",
            popularity: "1.9k stars",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "truefedex/tv-bro",
                asset_regex: r"\.apk$",
            },
        },
        CatalogApp {
            id: "fcast",
            name: "FCast Receiver",
            category: "Utilities & Casting",
            desc: "Open-source LAN casting receiver (works without Play Services).",
            package: "org.futo.fcast.receiver",
            filename: "fcast-receiver.apk",
            license: "MIT",
            popularity: "FUTO OSS",
            vanilla_default: false,
            source: AppSource::GitHub {
                repo: "futo-org/fcast",
                asset_regex: r"fcast-receiver-android.*\.apk$",
            },
        },
    ]
}

pub fn resolve_download_url(app: &CatalogApp) -> Result<String, String> {
    match &app.source {
        AppSource::DirectUrl(u) => Ok((*u).to_string()),
        AppSource::GitHub { repo, asset_regex } => {
            let api = format!("https://api.github.com/repos/{}/releases/latest", repo);
            let json: serde_json::Value = ureq::get(&api)
                .set("User-Agent", "Solstice-Pod-Tool/0.1")
                .call()
                .map_err(|e| format!("GitHub API error for {}: {}", repo, e))?
                .into_json()
                .map_err(|e| format!("Invalid GitHub JSON: {}", e))?;

            let assets = json["assets"]
                .as_array()
                .ok_or_else(|| format!("No assets in latest release of {}", repo))?;
            let rx = Regex::new(asset_regex).map_err(|e| e.to_string())?;
            for a in assets {
                if let (Some(name), Some(url)) = (a["name"].as_str(), a["browser_download_url"].as_str()) {
                    if rx.is_match(name) {
                        return Ok(url.to_string());
                    }
                }
            }
            for a in assets {
                if let (Some(name), Some(url)) = (a["name"].as_str(), a["browser_download_url"].as_str()) {
                    if name.to_lowercase().ends_with(".apk") {
                        return Ok(url.to_string());
                    }
                }
            }
            Err(format!("No matching APK asset found in {}", repo))
        }
        AppSource::Aptoide(pkg) => {
            let api = format!("https://ws75.aptoide.com/api/7/app/getMeta?package_name={}", pkg);
            let json: serde_json::Value = ureq::get(&api)
                .set("User-Agent", "Solstice-Pod-Tool/0.1")
                .call()
                .map_err(|e| format!("Aptoide API error: {}", e))?
                .into_json()
                .map_err(|e| format!("Invalid Aptoide JSON: {}", e))?;
            json["data"]["file"]["path"]
                .as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| format!("No download URL for {}", pkg))
        }
        AppSource::Plex => {
            let json: serde_json::Value = ureq::get("https://plex.tv/api/downloads/6.json")
                .set("User-Agent", "Solstice-Pod-Tool/0.1")
                .call()
                .map_err(|e| format!("Plex API error: {}", e))?
                .into_json()
                .map_err(|e| format!("Invalid Plex JSON: {}", e))?;
            if let Some(rels) = json["computer"]["Android"]["releases"].as_array() {
                for r in rels {
                    let lbl = r["label"].as_str().unwrap_or("").to_lowercase();
                    let url = r["url"].as_str().unwrap_or("");
                    if lbl.contains("arm64") || url.to_lowercase().contains("arm64") {
                        return Ok(url.to_string());
                    }
                }
                if let Some(url) = rels.first().and_then(|r| r["url"].as_str()) {
                    return Ok(url.to_string());
                }
            }
            Err("Could not resolve Plex Android APK URL".into())
        }
    }
}

pub fn download_to_file(url: &str, dest: &Path) -> Result<u64, String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = dest.with_extension("part");
    let resp = ureq::get(url)
        .set("User-Agent", "Solstice-Pod-Tool/0.1")
        .call()
        .map_err(|e| format!("Download error: {}", e))?;
    let mut reader = resp.into_reader();
    let mut out = File::create(&tmp).map_err(|e| e.to_string())?;
    let bytes = io::copy(&mut reader, &mut out).map_err(|e| e.to_string())?;
    drop(out);
    if bytes < 100_000 {
        let _ = fs::remove_file(&tmp);
        return Err(format!("Downloaded file too small ({} bytes)", bytes));
    }
    fs::rename(&tmp, dest).map_err(|e| e.to_string())?;
    Ok(bytes)
}
