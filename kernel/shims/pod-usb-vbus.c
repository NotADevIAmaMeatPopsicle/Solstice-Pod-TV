// SPDX-License-Identifier: GPL-2.0-only
/* Experimental SP8096 VBUS-only extcon provider. Not tested on hardware. */
#include <linux/err.h>
#include <linux/extcon.h>
#include <linux/interrupt.h>
#include <linux/module.h>
#include <linux/mutex.h>
#include <linux/of.h>
#include <linux/platform_device.h>
#include <linux/regulator/consumer.h>
#include <linux/slab.h>

struct pod_vbus {
	struct device *dev;
	struct extcon_dev *extcon;
	struct regulator *vin;
	int irq;
	struct mutex sample_lock;
};

static const unsigned int pod_cables[] = {
	EXTCON_USB, EXTCON_USB_HOST, EXTCON_USB_SPEED, EXTCON_NONE,
};

static int pod_vbus_sample(struct pod_vbus *pod)
{
	bool level;
	int ret;

	/* Serialize probe's first sample with threaded cable-edge updates. */
	mutex_lock(&pod->sample_lock);
	ret = irq_get_irqchip_state(pod->irq, IRQCHIP_STATE_LINE_LEVEL, &level);
	if (ret) {
		dev_err(pod->dev, "cannot read VBUS IRQ line: %d\n", ret);
		goto out;
	}
	/* The Pod HS controller supports high speed; do not advertise SS. */
	ret = extcon_set_cable_state_(pod->extcon, EXTCON_USB_SPEED, 0);
	if (ret)
		goto out;
	ret = extcon_set_cable_state_(pod->extcon, EXTCON_USB, level);
out:
	mutex_unlock(&pod->sample_lock);
	return ret;
}

static irqreturn_t pod_vbus_update(int irq, void *data)
{
	struct pod_vbus *pod = data;

	pod_vbus_sample(pod);
	return IRQ_HANDLED;
}

static int pod_vbus_probe(struct platform_device *pdev)
{
	struct pod_vbus *pod;
	int ret;

	pod = devm_kzalloc(&pdev->dev, sizeof(*pod), GFP_KERNEL);
	if (!pod)
		return -ENOMEM;
	pod->dev = &pdev->dev;
	mutex_init(&pod->sample_lock);
	pod->extcon = devm_extcon_dev_allocate(&pdev->dev, pod_cables);
	if (IS_ERR(pod->extcon))
		return PTR_ERR(pod->extcon);
	ret = devm_extcon_dev_register(&pdev->dev, pod->extcon);
	if (ret)
		return ret;
	pod->vin = devm_regulator_get(&pdev->dev, "vin");
	if (IS_ERR(pod->vin))
		return PTR_ERR(pod->vin);
	ret = regulator_enable(pod->vin);
	if (ret)
		return ret;
	pod->irq = platform_get_irq_byname(pdev, "vbus_det_irq");
	if (pod->irq < 0) {
		ret = pod->irq;
		goto disable_vin;
	}
	ret = devm_request_threaded_irq(&pdev->dev, pod->irq, NULL,
			pod_vbus_update, IRQF_TRIGGER_RISING | IRQF_TRIGGER_FALLING |
			IRQF_ONESHOT, "pod_vbus", pod);
	if (ret)
		goto disable_vin;
	platform_set_drvdata(pdev, pod);
	ret = pod_vbus_sample(pod);
	if (ret) {
		devm_free_irq(&pdev->dev, pod->irq, pod);
		goto disable_vin;
	}
	return 0;

disable_vin:
	regulator_disable(pod->vin);
	return ret;
}

static int pod_vbus_remove(struct platform_device *pdev)
{
	struct pod_vbus *pod = platform_get_drvdata(pdev);
	/* Stop the devm IRQ before removing power or extcon state. */
	devm_free_irq(&pdev->dev, pod->irq, pod);
	regulator_disable(pod->vin);
	return 0;
}

static const struct of_device_id pod_vbus_match[] = {
	{ .compatible = "mersive,sp8096-usb-vbus" },
	{ }
};
MODULE_DEVICE_TABLE(of, pod_vbus_match);

static struct platform_driver pod_vbus_driver = {
	.probe = pod_vbus_probe,
	.remove = pod_vbus_remove,
	.driver = { .name = "sp8096-usb-vbus", .of_match_table = pod_vbus_match },
};
module_platform_driver(pod_vbus_driver);
MODULE_LICENSE("GPL v2");
