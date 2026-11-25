/*
 * pod-cec-daemon.c - Lightweight HDMI-CEC remote daemon for Mersive Solstice Pod Gen3 (SP8096)
 *
 * Talks directly to Qualcomm mdss_hdmi_cec sysfs (/sys/devices/virtual/graphics/fb1/cec/)
 * and injects standard TV remote key presses via Linux /dev/uinput.
 *
 * License: Apache-2.0
 */

#include <errno.h>
#include <fcntl.h>
#include <linux/uinput.h>
#include <poll.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <unistd.h>

#define CEC_BASE        "/sys/devices/virtual/graphics/fb1/cec"
#define CEC_PA_PATH     "/sys/devices/virtual/graphics/fb1/pa"
#define LOGICAL_ADDR    8  /* Playback Device 2 (avoids colliding with consoles on LA 4) */
#define OSD_NAME        "Solstice Pod"

struct cec_msg {
    uint8_t sender_id;
    uint8_t recvr_id;
    uint8_t opcode;
    uint8_t operand[14];
    uint8_t frame_size;
    uint8_t retransmit;
};

static volatile sig_atomic_t g_running = 1;
static int g_uinput_fd = -1;
static int g_last_key = -1;

static void on_sig(int sig) {
    (void)sig;
    g_running = 0;
}

static int write_sysfs_str(const char *path, const char *val) {
    int fd = open(path, O_WRONLY);
    if (fd < 0) return -1;
    ssize_t n = write(fd, val, strlen(val));
    close(fd);
    return (n < 0) ? -1 : 0;
}

static uint16_t read_physical_addr(void) {
    char buf[32] = {0};
    int fd = open(CEC_PA_PATH, O_RDONLY);
    if (fd < 0) return 0x1000;
    ssize_t n = read(fd, buf, sizeof(buf) - 1);
    close(fd);
    if (n <= 0) return 0x1000;
    unsigned long v = strtoul(buf, NULL, 10);
    return (v > 0 && v <= 0xFFFF) ? (uint16_t)v : 0x1000;
}

static int cec_send(uint8_t recvr, uint8_t opcode, const uint8_t *op, uint8_t op_len) {
    struct cec_msg m;
    memset(&m, 0, sizeof(m));
    m.sender_id = LOGICAL_ADDR;
    m.recvr_id = recvr;
    m.opcode = opcode;
    if (op && op_len > 0) {
        if (op_len > 14) op_len = 14;
        memcpy(m.operand, op, op_len);
    }
    m.frame_size = (uint8_t)(2 + op_len);
    m.retransmit = 0;

    int fd = open(CEC_BASE "/wr_msg", O_WRONLY);
    if (fd < 0) return -1;
    ssize_t n = write(fd, &m, sizeof(m));
    close(fd);
    return (n == (ssize_t)sizeof(m)) ? 0 : -1;
}

static void send_report_pa(void) {
    uint16_t pa = read_physical_addr();
    uint8_t op[3] = { (uint8_t)(pa >> 8), (uint8_t)(pa & 0xFF), 0x04 /* Playback */ };
    cec_send(0x0F, 0x84, op, 3);
}

static void send_osd_name(uint8_t dst) {
    const char *name = OSD_NAME;
    size_t len = strlen(name);
    if (len > 14) len = 14;
    cec_send(dst, 0x47, (const uint8_t *)name, (uint8_t)len);
}

static void send_active_source(void) {
    uint16_t pa = read_physical_addr();
    uint8_t op[2] = { (uint8_t)(pa >> 8), (uint8_t)(pa & 0xFF) };
    cec_send(0x0F, 0x82, op, 2);
}

static int map_cec_key(uint8_t cec_code) {
    switch (cec_code) {
        case 0x00: return KEY_ENTER;       /* Select / OK */
        case 0x01: return KEY_UP;          /* Up */
        case 0x02: return KEY_DOWN;        /* Down */
        case 0x03: return KEY_LEFT;        /* Left */
        case 0x04: return KEY_RIGHT;       /* Right */
        case 0x09: return KEY_HOMEPAGE;    /* Root Menu */
        case 0x0A: return KEY_MENU;        /* Setup Menu */
        case 0x0B: return KEY_HOMEPAGE;    /* Contents Menu */
        case 0x0D: return KEY_BACK;        /* Exit */
        case 0x2B: return KEY_BACK;        /* Return / Back */
        case 0x41: return KEY_VOLUMEUP;    /* Volume Up */
        case 0x42: return KEY_VOLUMEDOWN;  /* Volume Down */
        case 0x43: return KEY_MUTE;        /* Mute */
        case 0x44: return KEY_PLAY;        /* Play */
        case 0x45: return KEY_STOP;        /* Stop */
        case 0x46: return KEY_PAUSE;       /* Pause */
        case 0x48: return KEY_REWIND;      /* Rewind */
        case 0x49: return KEY_FASTFORWARD; /* Fast Forward */
        case 0x4B: return KEY_NEXTSONG;    /* Forward / Next */
        case 0x4C: return KEY_PREVIOUSSONG;/* Backward / Prev */
        default:   return -1;
    }
}

static int init_uinput(void) {
    int fd = open("/dev/uinput", O_WRONLY | O_NONBLOCK);
    if (fd < 0) fd = open("/dev/input/uinput", O_WRONLY | O_NONBLOCK);
    if (fd < 0) return -1;

    ioctl(fd, UI_SET_EVBIT, EV_KEY);
    ioctl(fd, UI_SET_EVBIT, EV_SYN);

    const int keys[] = {
        KEY_ENTER, KEY_UP, KEY_DOWN, KEY_LEFT, KEY_RIGHT,
        KEY_HOMEPAGE, KEY_MENU, KEY_BACK,
        KEY_VOLUMEUP, KEY_VOLUMEDOWN, KEY_MUTE,
        KEY_PLAY, KEY_STOP, KEY_PAUSE, KEY_REWIND, KEY_FASTFORWARD,
        KEY_NEXTSONG, KEY_PREVIOUSSONG
    };
    for (size_t i = 0; i < sizeof(keys) / sizeof(keys[0]); i++) {
        ioctl(fd, UI_SET_KEYBIT, keys[i]);
    }

    struct uinput_user_dev uidev;
    memset(&uidev, 0, sizeof(uidev));
    snprintf(uidev.name, UINPUT_MAX_NAME_SIZE, "Solstice Pod HDMI-CEC Remote");
    uidev.id.bustype = BUS_VIRTUAL;
    uidev.id.vendor  = 0x18d1;
    uidev.id.product = 0x8096;
    uidev.id.version = 1;

    if (write(fd, &uidev, sizeof(uidev)) < 0 || ioctl(fd, UI_DEV_CREATE) < 0) {
        close(fd);
        return -1;
    }
    return fd;
}

static void emit_key(int code, int value) {
    if (g_uinput_fd < 0 || code < 0) return;
    struct input_event ev;
    memset(&ev, 0, sizeof(ev));
    ev.type = EV_KEY;
    ev.code = (uint16_t)code;
    ev.value = value;
    (void)write(g_uinput_fd, &ev, sizeof(ev));
    memset(&ev, 0, sizeof(ev));
    ev.type = EV_SYN;
    ev.code = SYN_REPORT;
    ev.value = 0;
    (void)write(g_uinput_fd, &ev, sizeof(ev));
}

static void handle_msg(const struct cec_msg *rx) {
    switch (rx->opcode) {
        case 0x83: /* Give Physical Address */
            send_report_pa();
            break;
        case 0x46: /* Give OSD Name */
            send_osd_name(rx->sender_id);
            break;
        case 0x8F: { /* Give Device Power Status */
            uint8_t on = 0x00;
            cec_send(rx->sender_id, 0x90, &on, 1);
            break;
        }
        case 0x9F: { /* Get CEC Version */
            uint8_t ver = 0x05; /* CEC 1.4 */
            cec_send(rx->sender_id, 0x9E, &ver, 1);
            break;
        }
        case 0x8C: { /* Give Device Vendor ID */
            uint8_t vid[3] = { 0x00, 0x15, 0x82 };
            cec_send(0x0F, 0x87, vid, 3);
            break;
        }
        case 0x85: /* Request Active Source - do not steal TV input when user switches away */
            break;
        case 0x86: { /* Set Stream Path - reply Active Source ONLY when TV selects OUR HDMI port */
            if (rx->frame_size >= 4) {
                uint16_t target_pa = (uint16_t)((rx->operand[0] << 8) | rx->operand[1]);
                if (target_pa == read_physical_addr()) {
                    send_active_source();
                }
            }
            break;
        }
        case 0x44: { /* User Control Pressed */
            if (rx->frame_size >= 3) {
                int key = map_cec_key(rx->operand[0]);
                if (key >= 0) {
                    emit_key(key, 1);
                    g_last_key = key;
                }
            }
            break;
        }
        case 0x45: /* User Control Released */
            if (g_last_key >= 0) {
                emit_key(g_last_key, 0);
                g_last_key = -1;
            }
            break;
        default:
            break;
    }
}

int main(void) {
    signal(SIGTERM, on_sig);
    signal(SIGINT, on_sig);

    write_sysfs_str(CEC_BASE "/clear_logical_addr", "1");
    write_sysfs_str(CEC_BASE "/enable", "1");
    write_sysfs_str(CEC_BASE "/logical_addr", "8");

    g_uinput_fd = init_uinput();
    if (g_uinput_fd < 0) {
        fprintf(stderr, "pod-cec-daemon: failed to open uinput: %s\n", strerror(errno));
        return 1;
    }

    /* Announce ourselves to the TV */
    send_report_pa();
    send_osd_name(0);

    int rfd = open(CEC_BASE "/rd_msg", O_RDONLY);
    if (rfd < 0) {
        fprintf(stderr, "pod-cec-daemon: failed to open rd_msg: %s\n", strerror(errno));
        close(g_uinput_fd);
        return 1;
    }

    struct pollfd pfd = { .fd = rfd, .events = POLLPRI | POLLERR };
    while (g_running) {
        int pr = poll(&pfd, 1, 2000);
        if (pr > 0) {
            for (;;) {
                struct cec_msg rx;
                memset(&rx, 0, sizeof(rx));
                lseek(rfd, 0, SEEK_SET);
                ssize_t n = read(rfd, &rx, sizeof(rx));
                if (n <= 0) break;
                handle_msg(&rx);
            }
        }
    }

    close(rfd);
    ioctl(g_uinput_fd, UI_DEV_DESTROY);
    close(g_uinput_fd);
    return 0;
}
