#!/bin/bash
# Warning: this is intended to be used inside docker, not directly.

# Tests if array can be created without issues.

/app/scripts/loop-devices.sh test-create 3

if /app/hyraid-unittest create --array-name unittest --raid-level 5 /dev/loop{0..2}; then 
    # HyRAID has a cargo feature for unit testing where it writes 
    # the path of the logical volume created to a file.
    # all we have to do is check if it exists.
    if ls "$(cat /tmp/hyraid_unittest)"; then
        echo "Create HyRAID array .. OK ✅"
        exit 0
    fi
else
    echo "Create HyRAID array .. Failed ❌"
    exit 1
fi