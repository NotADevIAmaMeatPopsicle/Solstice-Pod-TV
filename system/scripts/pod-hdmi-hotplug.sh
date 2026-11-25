#!/system/bin/sh
# Auto-enable external HDMI displays on boot, hot-plug, or TV input switch.
# Android 15 assigns a new incrementing displayId on each HDMI reconnect and
# leaves it in state OFF waiting for a "Mirror to external display?" prompt.
# This lightweight watcher enables any connected external display that is OFF.

while true; do
    ids=$(/system/bin/dumpsys display 2>/dev/null \
        | /system/bin/grep -o 'DisplayViewport{type=EXTERNAL, valid=true, isActive=false, displayId=[0-9]*' \
        | /system/bin/sed 's/.*displayId=//')
    for id in $ids; do
        if [ -n "$id" ]; then
            /system/bin/cmd display enable-display "$id" >/dev/null 2>&1
            /system/bin/wm fixed-to-user-rotation -d "$id" enabled >/dev/null 2>&1
            /system/bin/wm user-rotation -d "$id" lock 0 >/dev/null 2>&1
        fi
    done
    sleep 2
done
