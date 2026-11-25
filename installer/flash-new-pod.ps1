param(
    [string]$PodName = "pod-02",
    [string]$ImagesDir = ".\out\images",
    [string]$PlatformToolsDir = ".\platform-tools",
    [string]$ApksDir = ".\out\tv-apks",
    [string]$WifiSsid = "",
    [string]$WifiPassword = "",
    [switch]$SkipProvisionApps,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"
$adb = Join-Path $PlatformToolsDir "adb.exe"
$fastboot = Join-Path $PlatformToolsDir "fastboot.exe"

$bootImg = Join-Path $ImagesDir "pod44-los22-boot-signed.img"
$recImg  = Join-Path $ImagesDir "pod44-los22-recovery-signed.img"
$sysImg  = Join-Path $ImagesDir "pod-los22-system.simg"

Write-Host "=== SP8096 Android 15 (LineageOS 22.2) Turnkey Pod Installer ($PodName) ==="
foreach ($f in @($adb, $fastboot, $bootImg, $recImg, $sysImg)) {
    if (-not (Test-Path $f)) { throw "Missing required file: $f" }
}
Write-Host "Verified images:"
Get-Item $bootImg, $recImg, $sysImg | Format-Table Name, Length, LastWriteTime -AutoSize

if ($DryRun) {
    Write-Host "DryRun enabled: exiting before touching any device."
    exit 0
}

Write-Host "=== 1. Checking fastboot connection ==="
$fb = & $fastboot devices
if (-not $fb) {
    throw "No fastboot device detected. Boot the target Pod into fastboot (hold internal Vol-Down on power-on, or 'adb reboot bootloader') and retry."
}
Write-Host "Target fastboot device: $fb"
Write-Host "Note: This script NEVER touches persist, modem, or bt_firmware (preserves this Pod's unique MACs/calibration)."

Write-Host "`n=== 2. Flashing boot, recovery, and system (LineageOS 22.2) ==="
& $fastboot flash boot $bootImg
& $fastboot flash recovery $recImg
& $fastboot flash system $sysImg
& $fastboot format userdata
& $fastboot format cache
& $fastboot reboot

Write-Host "`n=== 3. Waiting for first Android 15 boot + sys.boot_completed=1 ==="
& $adb wait-for-device
for ($i = 0; $i -lt 90; $i++) {
    $bc = (& $adb shell "getprop sys.boot_completed" 2>$null).Trim()
    if ($bc -eq "1") { break }
    Start-Sleep -Seconds 2
}
if ($bc -ne "1") { throw "Timed out waiting for sys.boot_completed=1" }
Start-Sleep -Seconds 10
& $adb root | Out-Null
Start-Sleep -Seconds 2
& $adb wait-for-device

Write-Host "`n=== 4. Configuring TV box settings (disable SetupWizard, lockscreen, sleep) ==="
& $adb shell "settings put global device_provisioned 1; settings put secure user_setup_complete 1; settings put system screen_off_timeout 2147483647; locksettings set-disabled true 2>/dev/null || true; pm disable-user --user 0 org.lineageos.setupwizard 2>/dev/null || true"

if (-not $SkipProvisionApps -and (Test-Path $ApksDir)) {
    Write-Host "`n=== 5. Installing optional TV APKs from $ApksDir ==="
    Get-ChildItem $ApksDir -Filter "*.apk" | ForEach-Object {
        Write-Host "Installing $($_.Name)..."
        & $adb install -r $_.FullName
    }
    $hasTvos = (& $adb shell "pm list packages com.techgenius.tvoslauncher" 2>$null)
    if ($hasTvos -match "com.techgenius.tvoslauncher") {
        & $adb shell "cmd package set-home-activity com.techgenius.tvoslauncher/.MainActivity"
    }
}

if ($WifiSsid -and $WifiPassword) {
    Write-Host "`n=== 6. Connecting Wi-Fi ($WifiSsid) ==="
    & $adb shell "svc wifi enable; sleep 4; cmd wifi connect-network '$WifiSsid' wpa2 '$WifiPassword'; sleep 8; cmd wifi status | head -n 6"
}

Write-Host "`n=== 7. Final verification ==="
& $adb shell "uname -a; echo ---lsmod---; lsmod; echo ---wifi---; ip link show wlan0; echo ---bt---; dumpsys bluetooth_manager | grep -E 'enabled:|state:' | head -n 4"
Write-Host "`n=== DONE: $PodName provisioned to Android 15 (LineageOS 22.2) ==="
