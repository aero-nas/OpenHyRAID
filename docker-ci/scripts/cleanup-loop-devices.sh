#!/bin/bash
# Warning: this is intended to be used inside docker, not directly.
# REQUIRES --privileged
# Cleans up loop devices created

# shellcheck disable=SC2028

losetup -d /app/loopdev/*