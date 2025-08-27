# OpenHyRAID

Project still in alpha. Not yet ready for production use. But tests can be done to verify functionality.
<p align="center">
  <img src="img/hyraid.svg"
    height="100"
    style="padding:16px;"  
  >
</p>

Open-source, high-level mdadm wrapper to implement Synology SHR written in Rust.

# What is Synology SHR

SHR is a proprietary raid level that allows adding drives that are larger without wasting space. If you create a RAID array with 3 disks, 2 10TB and 1 15TB, you will lose 5TB of space. HyRAID and SHR, allow you to add larger disks by automatically partitioning the disks so you don't lose space.

# SHR vs HyRAID

HyRAID is free and licensed under GPLv2. While SHR is proprietary.

HyRAID also has priority aarch64 (architecture of Raspberry Pi 5/CM5) support.

made with ❤️ by lizard >w<

# Roadmap
Necessary for 1.0
  - [x] Creating arrays
  - [x] Adding and removing disks
  - [ ] Unit tests - partially done
  - [ ] ZFS support