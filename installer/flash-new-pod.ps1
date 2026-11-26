<#
.SYNOPSIS
    Resumable Android 15 (LineageOS 22.2) flasher and TV provisioner for Mersive Solstice Pod Gen3 (SP8096).

.DESCRIPTION
    Detects whether a connected Pod is in fastboot mode or already booted into Android over ADB:
      - Fastboot mode: flashes boot, recovery, and system, formats userdata/cache as ext4, reboots, then provisions.
      - Booted Android (ADB): skips flashing and resumes provisioning from wherever it left off.

    Never touches persist, modem, or bt_firmware (preserves each Pod's unique factory MAC addresses and calibration).
    Records verified units in flash-state.json so interrupted runs or multi-unit batches can resume safely.

.EXAMPLE
    .\installer\flash-new-pod.ps1 -ImagesDir .\out\images -PlatformToolsDir .\platform-tools -ApksDir .\out\tv-apks
.EXAMPLE
    .\installer\flash-new-pod.ps1 -ImagesDir .\out\images -PlatformToolsDir .\platform-tools -ApksDir .\out\tv-apks -Batch
#>
param(
    [string]$PodName = "pod",
    [string]$ImagesDir = ".\out\images",
    [string]$PlatformToolsDir = ".\platform-tools",
    [string]$ApksDir = ".\out\tv-apks",
    [string]$StateFile = ".\flash-state.json",
    [string]$WifiSsid = "",
    [string]$WifiPassword = "",
    [switch]$Batch,
    [switch]$Force,
    [switch]$SkipProvisionApps,
    [switch]$DryRun
)

# Native tools (adb/fastboot) print normal progress to stderr; check exit codes and outputs explicitly.
$ErrorActionPreference = "Continue"

$bootImg = Join-Path $ImagesDir "pod44-los22-boot-signed.img"
$recImg  = Join-Path $ImagesDir "pod44-los22-recovery-signed.img"
$sysImg  = Join-Path $ImagesDir "pod-los22-system.simg"

$adbOrig = Join-Path $PlatformToolsDir "adb.exe"
$fbOrig  = Join-Path $PlatformToolsDir "fastboot.exe"

Write-Host "=== Solstice Pod TV Resumable Flasher ($PodName) ===" -ForegroundColor Cyan
foreach ($f in @($adbOrig, $fbOrig, $bootImg, $recImg, $sysImg)) {
    if (-not (Test-Path $f)) { throw "Missing required file: $f" }
}
Write-Host "Verified images:"
Get-Item $bootImg, $recImg, $sysImg | Format-Table Name, Length, LastWriteTime -AutoSize

# Windows fastboot.exe fails to spawn mke2fs.exe (CreateProcess error 193) if its path contains spaces.
# Stage a space-free copy in %TEMP% automatically when needed.
$resolvedPt = (Resolve-Path $PlatformToolsDir).Path
if ($resolvedPt -match '\s') {
    $ptWork = Join-Path $env:TEMP "sp8096-platform-tools"
    if (-not (Test-Path (Join-Path $ptWork "fastboot.exe"))) {
        Write-Host "Staging space-free copy of platform-tools to $ptWork (fixes Windows fastboot format)..." -ForegroundColor Gray
        Copy-Item -Recurse -Force $resolvedPt $ptWork
    }
} else {
    $ptWork = $resolvedPt
}
$adb = Join-Path $ptWork "adb.exe"
$fb  = Join-Path $ptWork "fastboot.exe"

if ($DryRun) {
    Write-Host "DryRun enabled: verified images and tools ($ptWork). Exiting before touching any device." -ForegroundColor Green
    exit 0
}

$state = @{}
if (Test-Path $StateFile) {
    (Get-Content $StateFile -Raw | ConvertFrom-Json).PSObject.Properties | ForEach-Object { $state[$_.Name] = $_.Value }
}
function Save-State { $state | ConvertTo-Json | Set-Content $StateFile }
function Say($msg, $color = "Yellow") { Write-Host $msg -ForegroundColor $color }
function Sh($ser, $cmd) { (& $adb -s $ser shell $cmd 2>&1 | Out-String).Trim() }

function Wait-Boot([string]$ser) {
    & $adb -s $ser wait-for-device
    for ($i = 0; $i -lt 120; $i++) {
        if ((Sh $ser "getprop sys.boot_completed") -eq "1") {
            Start-Sleep -Seconds 5
            return
        }
        Start-Sleep -Seconds 2
    }
    throw "Timed out waiting for sys.boot_completed=1 on $ser"
}

function Flash-Device([string]$ser) {
    Say ">>> [flash] $ser (never touches persist, modem, or bt_firmware)" Cyan
    foreach ($step in @(
        @("flash", "boot", $bootImg),
        @("flash", "recovery", $recImg),
        @("flash", "system", $sysImg),
        @("format:ext4", "userdata"),
        @("format:ext4", "cache"),
        @("reboot")
    )) {
        Say "    fastboot $($step -join ' ')"
        & $fb -s $ser @step 2>&1 | ForEach-Object { "$_" } | Select-String -NotMatch "super|avb footer" | ForEach-Object { $_.Line }
        if ($LASTEXITCODE -ne 0) { throw "fastboot $($step -join ' ') failed" }
    }
}

function Provision-Device([string]$ser) {
    Say ">>> [provision] $ser" Cyan
    Wait-Boot $ser

    $hasBaked = ((Sh $ser "ls /vendor/bin/pod-cec-daemon 2>/dev/null") -match "pod-cec-daemon") -and
                ((Sh $ser "ls /vendor/bin/pod-hdmi-hotplug.sh 2>/dev/null") -match "hotplug")
    if ($hasBaked) {
        Say "    [ok] CEC daemon and HDMI hot-plug service already baked into /vendor" Green
    } else {
        Say "    [note] /vendor does not have baked CEC/hot-plug files; continuing with TV defaults and apps" Gray
    }

    Say "    applying TV defaults (skip setup wizard, hide battery icon, lock landscape, immersive status bar, disable phone apps)"
    Sh $ser "settings put global device_provisioned 1; settings put secure user_setup_complete 1; settings put secure icon_blacklist battery; dumpsys battery set present 0 2>/dev/null || true; settings put system screen_off_timeout 2147483647; settings put system accelerometer_rotation 0; settings put system user_rotation 0; settings put global policy_control immersive.status=*; wm fixed-to-user-rotation enabled; wm user-rotation lock 0; locksettings set-disabled true 2>/dev/null || true; pm disable-user --user 0 org.lineageos.setupwizard 2>/dev/null || true" | Out-Null

    foreach ($pkg in @("com.android.dialer","com.android.contacts","com.android.messaging","com.android.calculator2","com.android.deskclock","org.lineageos.aperture","org.lineageos.etar","org.lineageos.recorder","org.lineageos.glimpse","org.lineageos.twelve","com.mersive.solstice.webapp","com.splashtop.m360.mersive")) {
        Sh $ser "pm disable-user --user 0 $pkg 2>/dev/null || true" | Out-Null
    }

    if (-not $SkipProvisionApps -and (Test-Path $ApksDir)) {
        $apks = Get-ChildItem $ApksDir -Filter "*.apk" -ErrorAction SilentlyContinue
        foreach ($apk in $apks) {
            Say "    installing $($apk.Name)..."
            $out = (& $adb -s $ser install -r $apk.FullName 2>&1 | Out-String)
            if ($out -notmatch "Success") { throw "install failed for $($apk.Name): $out" }
        }
    }

    $hasProj = Sh $ser "pm list packages com.spocky.projengmenu"
    if ($hasProj -match "com.spocky.projengmenu") {
        Sh $ser "cmd package set-home-activity com.spocky.projengmenu/.ui.home.MainActivity" | Out-Null
    }

    if ($WifiSsid -and $WifiPassword) {
        Say "    connecting Wi-Fi ($WifiSsid)..."
        Sh $ser "svc wifi enable; sleep 4; cmd wifi connect-network '$WifiSsid' wpa2 '$WifiPassword'; sleep 6" | Out-Null
    }

    Sh $ser "input keyevent KEYCODE_HOME; sync" | Out-Null

    # Verification checklist
    $checks = [ordered]@{
        "kernel 4.4.302-podmin" = { (Sh $ser "uname -r") -match "4.4.302-podmin" }
        "android boot completed" = { (Sh $ser "getprop sys.boot_completed") -eq "1" }
        "wifi interface present" = { (Sh $ser "ip link show wlan0 2>/dev/null") -match "wlan0" }
    }
    if ($hasBaked) {
        $checks["cec daemon running"] = { (Sh $ser "pidof pod-cec-daemon") -match "\d+" }
        $checks["hotplug running"]    = { (Sh $ser "ps -ef | grep pod-hdmi-hotplug | grep -v grep") -match "pod-hdmi" }
    }
    if ($hasProj -match "com.spocky.projengmenu") {
        $checks["Projectivy is home"] = { (Sh $ser "cmd role get-role-holders --user 0 android.app.role.HOME") -match "spocky" }
    }

    $failed = @()
    foreach ($k in $checks.Keys) {
        $ok = $false
        try { $ok = [bool](& $checks[$k]) } catch {}
        if ($ok) { Say "    [PASS] $k" Green } else { Say "    [FAIL] $k" Red; $failed += $k }
    }
    if ($failed.Count -gt 0) { throw "verification failed: $($failed -join ', ')" }
}

function Handle-Device([string]$ser, [string]$mode) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    Say "`n=== POD $ser (detected in $mode) ===" Cyan
    try {
        if ($mode -eq "fastboot") { Flash-Device $ser }
        Provision-Device $ser
        $state[$ser] = "done"
        Save-State
        $sw.Stop()
        Say "=== POD $ser DONE in $([int]$sw.Elapsed.TotalSeconds)s ===" Green
        try { [console]::beep(880, 250); [console]::beep(1175, 350) } catch {}
        return $true
    } catch {
        $state[$ser] = "failed: $($_.Exception.Message)"
        Save-State
        Say "=== POD $ser FAILED: $($_.Exception.Message) ===" Red
        Say "    Re-running is safe: fastboot units are flashed, booted units resume provisioning." Red
        try { [console]::beep(300, 700) } catch {}
        return $false
    }
}

$tried = @{}
if ($Batch) {
    Say "Batch mode active: watching for Pods in fastboot or ADB... (Press Ctrl+C to stop)" Cyan
}

while ($true) {
    $units = @()
    (& $fb devices 2>$null)  | ForEach-Object { if ($_ -match '^(\S+)\s+fastboot') { $units += ,@($Matches[1], "fastboot") } }
    (& $adb devices 2>$null) | ForEach-Object { if ($_ -match '^(\S+)\s+device\b') { $units += ,@($Matches[1], "android") } }

    $next = $units | Where-Object { ($Force -or $state[$_[0]] -ne "done") -and -not $tried.ContainsKey($_[0]) } | Select-Object -First 1
    if ($next) {
        $tried[$next[0]] = $true
        $ok = Handle-Device $next[0] $next[1]
        if (-not $Batch) {
            if (-not $ok) { exit 1 }
            break
        }
        Say "Waiting for next Pod..." Gray
    } else {
        if (-not $Batch) {
            throw "No eligible fastboot or ADB device detected. Boot the Pod into fastboot (hold internal VOL- on power-on) or pass -Force to re-provision a completed unit."
        }
        Start-Sleep -Seconds 2
    }
}
