#!/bin/bash
# Warning: this is intended to be used inside docker, not directly.
# Creates loop devices for testing

# Example: create-loop-devices.sh test-create 3
# /loopdev1.img -> 1GB
# /loopdev2.img -> 2GB
# /loopdev3.img -> 3GB

# Example: create-loop-devices.sh test-extend 3
# /loopdev-extend1.img -> 1GB
# /loopdev-extend2.img -> 2GB
# /loopdev-extend3.img -> 3GB

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

# Creates loop device for testing
# Example:
# create-loop-dev /app/loopdev/loopdev1 1
# /app/loopdev/loopdev1 -> 1GB
create-loop-dev () {
    IMG_PATH="/app/loopdevimg/$1.img"

    dd if=/dev/zero of="$IMG_PATH" bs=1000M count="$2"

    losetup -fP "$IMG_PATH" 
    echo "w\nq\n" | fdisk "$IMG_PATH" > /dev/null
}

if [[ "$1" == "test-create" ]]; then 
    for ((i=0; i <= ($2 - 1); i++)) do    
        create-loop-dev loopdev"$i" $i
    done
fi

if [[ "$1" == "test-extend" ]]; then 
    for ((i=0; i <= ($2 - 1); i++)) do    
        create-loop-dev loopdev-extend"$i" $i
    done
fi