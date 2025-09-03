#!/bin/bash
# Warning: this is intended to be used inside docker, not directly.

# Tests if array can be extended without issues.

# shut the fuck up
# shellcheck disable=SC2002
# shellcheck disable=SC1091

source /app/scripts/lib.sh

create-multiple 3

check_dev_mdadm () {
    cat /proc/mdstat | grep "${1#/dev/}" > /dev/null && return 0 || return 1
}

list_raid_arrays () {
    cat /proc/mdstat | grep "${1#/dev/}" | grep -oP '^md\d+' 
}

if /app/hyraid-unittest extend --array-name unittest /dev/loop{3..5}; then 
    list_raid_arrays "$1" | 
    while read -r item; do 
        pvs | grep "$item" || {
            echo "Extend HyRAID array .. Failed ❌"
            exit 1
        }
    done
    echo "Extend HyRAID array .. OK ✅"
    exit 0
fi

