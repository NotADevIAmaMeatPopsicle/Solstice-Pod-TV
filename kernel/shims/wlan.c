// SPDX-License-Identifier: GPL-2.0
/*
 * Minimal sysfs parameter shim for Android Wi-Fi HAL (android.hardware.wifi-service)
 * when using upstream ath10k_pci instead of qcacld-3.0 on SP8096.
 *
 * Exposes /sys/module/wlan/parameters/fwpath so wifi_change_fw_path() succeeds.
 */
#include <linux/module.h>
#include <linux/moduleparam.h>

static char *fwpath = "sta";
module_param(fwpath, charp, 0660);
MODULE_PARM_DESC(fwpath, "Dummy fwpath param for Android Wi-Fi HAL with ath10k_pci");
MODULE_LICENSE("GPL");
