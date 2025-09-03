#!/bin/bash
# Warning: this is intended to be used inside docker, not directly.
# Functions available to other scripts
# Import by: source /app/scripts/lib.sh

# shellcheck disable=SC2028 

minor_number () {
    if ls /loop_minor > /dev/null 2>&1; then 
        MINOR_NUM=$(cat /loop_minor)
        NEW_MINOR_NUM=$((MINOR_NUM+1))
        echo $NEW_MINOR_NUM > /loop_minor
        echo $NEW_MINOR_NUM
    else
        echo 0 > /loop_minor
        echo 0
    fi
}

# Fix loopdevs not having a partition table.
# This causes HyRAID to panic, but it won't happen with real disks anyways.
fix-loopdev () {
    echo "w\nq\n" | fdisk "$1" > /dev/null
}

create-loop-dev () {
    MINOR_NUM=$(minor_number)
    IMG_PATH="/app/loopdevimg/loop$MINOR_NUM.img"

    dd if=/dev/zero of="$IMG_PATH" bs=1000M count="$1"

    fix-loopdev "/dev/loop$MINOR_NUM"
    losetup -fP "$IMG_PATH" 
}

create-multiple () {
    for ((i=0; i <= ($1 - 1); i++)) do    
        create-loop-dev $i
    done
}