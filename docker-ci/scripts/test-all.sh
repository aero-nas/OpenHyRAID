#!/bin/bash
# WARNING: Must be run inside docker with --privileged, and host must have no loop devices active. 
# Otherwise this will likely fail. Detach all loop devices before running this.

cleanup () {
    yes | vgremove /dev/hyraid_* && mdadm --stop --scan
    /app/scripts/cleanup-loop-devices.sh
}

failure () {
    echo "Unit tests failed. Cleaning up..."
    cleanup
    exit 1
}

/app/scripts/test-01-create.sh || failure
/app/scripts/test-02-extend.sh || failure

echo "Cleaning up..."
cleanup